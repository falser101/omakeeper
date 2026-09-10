use crate::util;
use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use serde::Serialize;
use std::io::stdout;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const OLD_DAYS: u64 = 90;
#[allow(dead_code)]
const LARGE_FILE_MIN: u64 = 100 * 1024 * 1024;
#[allow(dead_code)]
const LARGE_FILE_CAP: usize = 20;

#[derive(Debug, Clone, Serialize)]
pub struct DiskEntry {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
    pub protected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age_days: Option<u64>,
}

#[derive(Debug, Serialize)]
struct AnalyzeReport {
    path: String,
    overview: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent: Option<String>,
    entries: Vec<DiskEntry>,
    large_files: Vec<DiskEntry>,
    total_size: u64,
    total_files: usize,
    disk_used: u64,
    disk_total: u64,
    disk_free: u64,
}

#[derive(Clone)]
enum Location {
    Dir(PathBuf),
    OldDownloads { root: PathBuf },
}

struct Row {
    entry: DiskEntry,
    location: Location,
}

struct Guard;
impl Drop for Guard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = stdout().execute(LeaveAlternateScreen);
    }
}

fn age_days(path: &Path) -> Option<u64> {
    let modified = path.metadata().ok()?.modified().ok()?;
    let now = SystemTime::now();
    Some(now.duration_since(modified).ok()?.as_secs() / 86400)
}

fn is_virtual_root(name: &str) -> bool {
    matches!(name, "proc" | "sys" | "dev" | "run")
}

fn is_view_only(path: &Path) -> bool {
    util::is_dangerous_delete(path)
}

fn entry_name(path: &Path) -> String {
    path.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| path.display().to_string())
}

fn disk_entry(path: &Path, size: u64, is_dir: bool) -> DiskEntry {
    DiskEntry {
        name: entry_name(path),
        path: path.display().to_string(),
        size,
        is_dir,
        protected: is_view_only(path),
        age_days: age_days(path),
    }
}

fn overview_locations() -> Result<Vec<(String, Location)>> {
    let home = util::home_dir()?;
    let mut out = vec![
        ("Home".into(), Location::Dir(home.clone())),
        ("Cache".into(), Location::Dir(home.join(".cache"))),
        ("Local data".into(), Location::Dir(home.join(".local"))),
        ("Config".into(), Location::Dir(home.join(".config"))),
        ("Downloads".into(), Location::Dir(home.join("Downloads"))),
        ("Desktop".into(), Location::Dir(home.join("Desktop"))),
        ("Projects".into(), Location::Dir(home.join("Projects"))),
        (
            format!("Old Downloads ({OLD_DAYS}d+)"),
            Location::OldDownloads {
                root: home.join("Downloads"),
            },
        ),
    ];
    out.retain(|(_, loc)| match loc {
        Location::Dir(p) => p.exists(),
        Location::OldDownloads { root } => root.is_dir(),
    });
    Ok(out)
}

fn old_download_files(root: &Path) -> Vec<PathBuf> {
    walkdir::WalkDir::new(root)
        .max_depth(3)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| age_days(e.path()).unwrap_or(0) >= OLD_DAYS)
        .map(|e| e.path().to_path_buf())
        .collect()
}

fn location_size(loc: &Location) -> u64 {
    match loc {
        Location::Dir(p) => util::path_size(p),
        Location::OldDownloads { root } => {
            let files = old_download_files(root);
            util::path_sizes(&files).into_iter().sum()
        }
    }
}

fn list_dir(path: &Path) -> Result<Vec<Row>> {
    let mut children: Vec<PathBuf> = std::fs::read_dir(path)
        .with_context(|| format!("read {}", path.display()))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n != "." && n != "..")
                .unwrap_or(true)
        })
        .collect();
    children.sort();
    let sizes = util::path_sizes(&children);
    let mut rows: Vec<Row> = children
        .into_iter()
        .zip(sizes)
        .map(|(p, size)| {
            let is_dir = p.is_dir();
            Row {
                entry: disk_entry(&p, size, is_dir),
                location: Location::Dir(p),
            }
        })
        .collect();
    rows.sort_by(|a, b| b.entry.size.cmp(&a.entry.size));
    Ok(rows)
}

fn list_disk_root() -> Result<Vec<Row>> {
    let root = PathBuf::from("/");
    let home = util::home_dir()?;
    let skip_home_mount = home.starts_with("/home");
    let mut children: Vec<PathBuf> = std::fs::read_dir(&root)
        .context("read /")?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if is_virtual_root(name) {
                return false;
            }
            if skip_home_mount && name == "home" {
                return false;
            }
            true
        })
        .collect();
    if skip_home_mount && home.is_dir() {
        children.push(home);
    }
    children.sort();
    let sizes = util::path_sizes(&children);
    let mut rows: Vec<Row> = children
        .into_iter()
        .zip(sizes)
        .map(|(p, size)| {
            let is_dir = p.is_dir();
            Row {
                entry: disk_entry(&p, size, is_dir),
                location: Location::Dir(p),
            }
        })
        .collect();
    rows.sort_by(|a, b| b.entry.size.cmp(&a.entry.size));
    Ok(rows)
}

fn list_old_downloads(root: &Path) -> Vec<Row> {
    let files = old_download_files(root);
    let sizes = util::path_sizes(&files);
    let mut rows: Vec<Row> = files
        .into_iter()
        .zip(sizes)
        .map(|(p, size)| Row {
            entry: disk_entry(&p, size, false),
            location: Location::Dir(p),
        })
        .collect();
    rows.sort_by(|a, b| b.entry.size.cmp(&a.entry.size));
    rows
}

#[allow(dead_code)]
fn large_files_in(path: &Path) -> Vec<DiskEntry> {
    let mut files: Vec<DiskEntry> = walkdir::WalkDir::new(path)
        .follow_links(false)
        .max_depth(4)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| {
            let size = e.metadata().ok()?.len();
            if size < LARGE_FILE_MIN {
                return None;
            }
            Some(disk_entry(e.path(), size, false))
        })
        .collect();
    files.sort_by(|a, b| b.size.cmp(&a.size));
    files.truncate(LARGE_FILE_CAP);
    files
}

fn overview_rows() -> Result<Vec<Row>> {
    let locs = overview_locations()?;
    let sizes: Vec<u64> = std::thread::scope(|scope| {
        let handles: Vec<_> = locs
            .iter()
            .map(|(_, loc)| scope.spawn(|| location_size(loc)))
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap_or(0))
            .collect()
    });
    Ok(locs
        .into_iter()
        .zip(sizes)
        .map(|((name, loc), size)| {
            let path = match &loc {
                Location::Dir(p) => p.display().to_string(),
                Location::OldDownloads { root } => root.display().to_string(),
            };
            Row {
                entry: DiskEntry {
                    name,
                    path: path.clone(),
                    size,
                    is_dir: true,
                    protected: is_view_only(Path::new(&path)),
                    age_days: None,
                },
                location: loc,
            }
        })
        .collect())
}

fn report_for(path: &Path, overview: bool) -> Result<AnalyzeReport> {
    let is_disk = path == Path::new("/");
    let rows = if overview {
        overview_rows()?
    } else if is_disk {
        list_disk_root()?
    } else {
        list_dir(path)?
    };
    let entries: Vec<DiskEntry> = rows.into_iter().map(|r| r.entry).collect();
    let stat = util::disk_stat(path).unwrap_or_default();
    let total_size = if is_disk && stat.used > 0 {
        stat.used
    } else {
        entries.iter().map(|e| e.size).sum()
    };
    let total_files = entries.len();
    let parent = if is_disk {
        None
    } else {
        path.parent().map(|p| {
            if p.as_os_str().is_empty() {
                "/".into()
            } else {
                p.display().to_string()
            }
        })
    };
    let large_files = Vec::new();
    Ok(AnalyzeReport {
        path: path.display().to_string(),
        overview,
        parent,
        entries,
        large_files,
        total_size,
        total_files,
        disk_used: stat.used,
        disk_total: stat.total,
        disk_free: stat.available,
    })
}

struct Screen {
    stack: Vec<(String, Vec<Row>)>,
    state: ListState,
    selected: Vec<bool>,
    message: String,
}

impl Screen {
    fn current_rows(&self) -> &[Row] {
        self.stack.last().map(|s| s.1.as_slice()).unwrap_or(&[])
    }
    fn title(&self) -> &str {
        self.stack.last().map(|s| s.0.as_str()).unwrap_or("Analyze")
    }
    fn reset_selection(&mut self) {
        self.selected = vec![false; self.current_rows().len()];
        self.state.select(Some(0).filter(|_| !self.current_rows().is_empty()));
    }
}

fn trash_selected(screen: &mut Screen) -> Result<()> {
    let plan: Vec<(PathBuf, u64, String)> = screen
        .current_rows()
        .iter()
        .zip(screen.selected.iter())
        .filter(|(row, on)| **on && !row.entry.protected)
        .map(|(row, _)| {
            (
                PathBuf::from(&row.entry.path),
                row.entry.size,
                row.entry.name.clone(),
            )
        })
        .collect();
    if plan.is_empty() {
        screen.message = "Nothing selected.".into();
        return Ok(());
    }

    let mut count = 0usize;
    let mut bytes = 0u64;
    let mut errors = Vec::new();
    let mut trashed = Vec::new();
    for (path, size, name) in plan {
        match util::move_to_trash(&path) {
            Ok(()) => {
                count += 1;
                bytes += size;
                trashed.push(path);
            }
            Err(e) => errors.push(format!("{name}: {e}")),
        }
    }

    if let Some((_, rows_mut)) = screen.stack.last_mut() {
        rows_mut.retain(|row| {
            let p = PathBuf::from(&row.entry.path);
            !trashed.iter().any(|t| t == &p)
        });
    }
    screen.reset_selection();
    if errors.is_empty() {
        screen.message = format!(
            "Moved {count} items to Trash ({})",
            util::format_bytes(bytes)
        );
    } else {
        screen.message = format!("Trashed {count}; {}", errors.join("; "));
    }
    Ok(())
}

fn run_tui(start: Option<PathBuf>) -> Result<()> {
    eprint!("Scanning disk… ");
    let (title, rows) = if let Some(path) = start {
        let rows = list_dir(&path)?;
        (path.display().to_string(), rows)
    } else {
        ("Overview".into(), overview_rows()?)
    };
    eprintln!("done.");

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let _guard = Guard;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    let mut screen = Screen {
        stack: vec![(title, rows)],
        state: ListState::default(),
        selected: Vec::new(),
        message: "Enter open · space select · d trash · h/backspace up · q quit".into(),
    };
    screen.reset_selection();

    loop {
        terminal.draw(|frame| draw_analyze(frame, &mut screen))?;
        if !event::poll(std::time::Duration::from_millis(200))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        let len = screen.current_rows().len();
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
            KeyCode::Down | KeyCode::Char('j') if len > 0 => {
                let i = screen.state.selected().unwrap_or(0);
                screen.state.select(Some((i + 1) % len));
            }
            KeyCode::Up | KeyCode::Char('k') if len > 0 => {
                let i = screen.state.selected().unwrap_or(0);
                screen.state.select(Some(if i == 0 { len - 1 } else { i - 1 }));
            }
            KeyCode::Char(' ') if len > 0 => {
                if let Some(i) = screen.state.selected() {
                    if let Some(flag) = screen.selected.get_mut(i) {
                        *flag = !*flag;
                    }
                }
            }
            KeyCode::Char('a') => {
                let all = screen.selected.iter().all(|s| *s);
                for flag in &mut screen.selected {
                    *flag = !all;
                }
            }
            KeyCode::Char('d') => {
                let n = screen.selected.iter().filter(|s| **s).count();
                if n == 0 {
                    screen.message = "Select items with space first.".into();
                    continue;
                }
                trash_selected(&mut screen)?;
            }
            KeyCode::Backspace | KeyCode::Char('h') => {
                if screen.stack.len() > 1 {
                    screen.stack.pop();
                    screen.reset_selection();
                    screen.message.clear();
                }
            }
            KeyCode::Enter if len > 0 => {
                let idx = screen.state.selected().unwrap_or(0);
                let loc = screen.current_rows()[idx].location.clone();
                let name = screen.current_rows()[idx].entry.name.clone();
                match loc {
                    Location::Dir(p) if p.is_dir() => {
                        match list_dir(&p) {
                            Ok(rows) => {
                                screen.stack.push((name, rows));
                                screen.reset_selection();
                                screen.message.clear();
                            }
                            Err(e) => screen.message = format!("{e}"),
                        }
                    }
                    Location::OldDownloads { root } => {
                        let rows = list_old_downloads(&root);
                        screen.stack.push((name, rows));
                        screen.reset_selection();
                    }
                    Location::Dir(_) => {
                        if let Some(flag) = screen.selected.get_mut(idx) {
                            *flag = !*flag;
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn draw_analyze(frame: &mut Frame, screen: &mut Screen) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Length(2),
        ])
        .split(area);

    let rows = screen.current_rows();
    let total: u64 = rows.iter().map(|r| r.entry.size).sum();
    let free = util::home_dir()
        .ok()
        .and_then(|h| util::free_space(&h))
        .map(util::format_bytes)
        .unwrap_or_else(|| "?".into());
    let header = Paragraph::new(format!(
        "Analyze  {}  ({})  free {free}",
        screen.title(),
        util::format_bytes(total)
    ))
    .style(Style::default().fg(Color::Cyan).bold())
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, chunks[0]);

    let max_size = rows.iter().map(|r| r.entry.size).max().unwrap_or(1).max(1);
    let items: Vec<ListItem> = rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let mark = if screen.selected.get(i).copied().unwrap_or(false) {
                "●"
            } else {
                "○"
            };
            let kind = if row.entry.is_dir { "/" } else { " " };
            let bar = bar_for(row.entry.size, max_size, 18);
            let age = row
                .entry
                .age_days
                .filter(|d| *d >= OLD_DAYS)
                .map(|d| format!("  >{d}d"))
                .unwrap_or_default();
            ListItem::new(Line::from(vec![
                Span::raw(format!(" {mark} {bar}  ")),
                Span::styled(
                    format!("{:>10}", util::format_bytes(row.entry.size)),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(format!("  {}{kind}{age}", row.entry.name)),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Usage "))
        .highlight_style(
            Style::default()
                .bg(Color::Indexed(238))
                .fg(Color::Cyan)
                .bold(),
        )
        .highlight_symbol("▶");
    frame.render_stateful_widget(list, chunks[1], &mut screen.state);

    let help = Paragraph::new(screen.message.as_str()).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[2]);
}

fn bar_for(size: u64, max: u64, width: usize) -> String {
    let filled = ((size as f64 / max as f64) * width as f64).round() as usize;
    let filled = filled.min(width);
    format!(
        "{}{}",
        "█".repeat(filled),
        "░".repeat(width.saturating_sub(filled))
    )
}

pub fn run(path: Option<String>, json: bool) -> Result<()> {
    if json {
        let (report_path, overview) = match path {
            Some(p) => (util::expand_user(&p), false),
            None => (util::home_dir()?, false),
        };
        if !overview && !report_path.exists() {
            anyhow::bail!("path not found: {}", report_path.display());
        }
        let report = report_for(&report_path, overview)?;
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    if !util::is_tty() {
        anyhow::bail!("analyze needs a terminal, or pass --json");
    }
    let start = path.map(|p| util::expand_user(&p));
    if let Some(p) = &start {
        if !p.exists() {
            anyhow::bail!("path not found: {}", p.display());
        }
        if p.is_file() {
            anyhow::bail!("analyze expects a directory: {}", p.display());
        }
    }
    run_tui(start)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bar_scales() {
        assert_eq!(bar_for(0, 100, 4), "░░░░");
        assert_eq!(bar_for(100, 100, 4), "████");
    }

    #[test]
    fn old_downloads_threshold() {
        assert_eq!(OLD_DAYS, 90);
    }

    #[test]
    fn virtual_roots_are_skipped() {
        assert!(is_virtual_root("proc"));
        assert!(is_virtual_root("sys"));
        assert!(is_virtual_root("dev"));
        assert!(is_virtual_root("run"));
        assert!(!is_virtual_root("usr"));
        assert!(!is_virtual_root("home"));
        assert!(!is_virtual_root("var"));
    }

    #[test]
    fn system_paths_are_view_only() {
        assert!(is_view_only(Path::new("/")));
        assert!(is_view_only(Path::new("/usr")));
        assert!(is_view_only(Path::new("/etc")));
        assert!(is_view_only(Path::new("/boot")));
        assert!(is_view_only(Path::new("/var/log")));
    }

    #[test]
    fn home_children_are_not_view_only() {
        let home = util::home_dir().expect("HOME");
        let child = home.join("Downloads");
        assert!(!is_view_only(&child));
        assert!(is_view_only(&home));
    }
}
