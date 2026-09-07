use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use std::io::{self, stdout};
use std::process::Command;

struct Item {
    key: char,
    title: &'static str,
    subtitle: &'static str,
    command: Option<&'static str>,
    planned: bool,
}

const ITEMS: &[Item] = &[
    Item {
        key: '1',
        title: "Clean",
        subtitle: "Caches, trash, browser & dev junk",
        command: Some("clean"),
        planned: false,
    },
    Item {
        key: '2',
        title: "Purge",
        subtitle: "Project artifacts (node_modules, target, …)",
        command: Some("purge"),
        planned: false,
    },
    Item {
        key: '3',
        title: "Installer",
        subtitle: "Leftover iso / AppImage / package files",
        command: Some("installer"),
        planned: false,
    },
    Item {
        key: '4',
        title: "History",
        subtitle: "Recent cleanup operations",
        command: Some("history"),
        planned: false,
    },
    Item {
        key: '5',
        title: "Uninstall",
        subtitle: "Remove packages + leftovers",
        command: None,
        planned: true,
    },
    Item {
        key: '6',
        title: "Optimize",
        subtitle: "Bounded system maintenance",
        command: None,
        planned: true,
    },
    Item {
        key: '7',
        title: "Analyze",
        subtitle: "Disk explorer",
        command: None,
        planned: true,
    },
    Item {
        key: '8',
        title: "Status",
        subtitle: "Live system health",
        command: None,
        planned: true,
    },
];

pub fn run(_json: bool) -> Result<()> {
    let mut state = ListState::default();
    state.select(Some(0));

    loop {
        enable_raw_mode()?;
        stdout().execute(EnterAlternateScreen)?;
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

        let action = loop {
            terminal.draw(|frame| draw(frame, &mut state))?;

            if !event::poll(std::time::Duration::from_millis(200))? {
                continue;
            }
            let Event::Key(key) = event::read()? else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break MenuAction::Quit,
                KeyCode::Down | KeyCode::Char('j') => {
                    let i = state.selected().unwrap_or(0);
                    state.select(Some((i + 1) % ITEMS.len()));
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    let i = state.selected().unwrap_or(0);
                    state.select(Some(if i == 0 { ITEMS.len() - 1 } else { i - 1 }));
                }
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    if let Some((idx, _)) = ITEMS.iter().enumerate().find(|(_, it)| it.key == c) {
                        state.select(Some(idx));
                        if let Some(cmd) = ITEMS[idx].command {
                            break MenuAction::Run {
                                command: cmd,
                                dry_run: false,
                            };
                        }
                    }
                }
                KeyCode::Char('d') => {
                    if let Some(idx) = state.selected() {
                        if let Some(cmd) = ITEMS[idx].command {
                            if matches!(cmd, "clean" | "purge" | "installer") {
                                break MenuAction::Run {
                                    command: cmd,
                                    dry_run: true,
                                };
                            }
                        }
                    }
                }
                KeyCode::Enter => {
                    if let Some(idx) = state.selected() {
                        let item = &ITEMS[idx];
                        if item.planned {
                            continue;
                        }
                        if let Some(cmd) = item.command {
                            break MenuAction::Run {
                                command: cmd,
                                dry_run: false,
                            };
                        }
                    }
                }
                _ => {}
            }
        };

        disable_raw_mode()?;
        stdout().execute(LeaveAlternateScreen)?;

        match action {
            MenuAction::Quit => return Ok(()),
            MenuAction::Run { command, dry_run } => {
                run_external(command, dry_run)?;
            }
        }
    }
}

enum MenuAction {
    Quit,
    Run {
        command: &'static str,
        dry_run: bool,
    },
}

fn draw(frame: &mut Frame, state: &mut ListState) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(2),
        ])
        .split(area);

    let title = Paragraph::new("Omakeeper — Omarchy system maintenance")
        .style(Style::default().fg(Color::Cyan).bold())
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    let items: Vec<ListItem> = ITEMS
        .iter()
        .map(|item| {
            let status = if item.planned { " (soon)" } else { "" };
            let line = Line::from(vec![
                Span::styled(
                    format!(" {} ", item.key),
                    Style::default().fg(Color::Yellow).bold(),
                ),
                Span::styled(
                    format!("{}{status}", item.title),
                    Style::default().fg(if item.planned {
                        Color::DarkGray
                    } else {
                        Color::White
                    }),
                ),
                Span::raw("  "),
                Span::styled(item.subtitle, Style::default().fg(Color::Gray)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Tools "))
        .highlight_style(
            Style::default()
                .bg(Color::Indexed(238))
                .fg(Color::Cyan)
                .bold(),
        )
        .highlight_symbol("▶ ");
    frame.render_stateful_widget(list, chunks[1], state);

    let help = Paragraph::new("↑/↓ move · Enter run · d dry-run · q quit")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[2]);
}

fn run_external(subcommand: &str, dry_run: bool) -> Result<()> {
    let exe = std::env::current_exe().unwrap_or_else(|_| "omakeeper".into());
    let mut cmd = Command::new(&exe);
    cmd.arg(subcommand);
    if dry_run {
        cmd.arg("--dry-run");
    }
    let status = cmd.status()?;
    if !status.success() {
        anyhow::bail!("{subcommand} exited with {status}");
    }

    println!("\nPress Enter to return to menu…");
    let mut line = String::new();
    let _ = io::stdin().read_line(&mut line);
    Ok(())
}
