use crate::util;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use std::io::stdout;

pub struct PickItem {
    pub selected: bool,
    pub locked: bool,
    pub title: String,
    pub detail: String,
    pub bytes: u64,
}

struct Guard;

impl Drop for Guard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = stdout().execute(LeaveAlternateScreen);
    }
}

fn visible_indices(items: &[PickItem], filter: &str) -> Vec<usize> {
    let f = filter.trim().to_ascii_lowercase();
    if f.is_empty() {
        return (0..items.len()).collect();
    }
    items
        .iter()
        .enumerate()
        .filter(|(_, it)| {
            it.title.to_ascii_lowercase().contains(&f) || it.detail.to_ascii_lowercase().contains(&f)
        })
        .map(|(i, _)| i)
        .collect()
}

/// Interactive multi-select. Returns `true` if the user confirmed.
/// When stdin/stdout are not a TTY, keeps the current selection and confirms.
pub fn select_items(title: &str, items: &mut [PickItem]) -> Result<bool> {
    if items.is_empty() {
        return Ok(false);
    }
    if !util::is_tty() {
        return Ok(true);
    }

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let _guard = Guard;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    let mut state = ListState::default();
    state.select(Some(0));
    let mut filter = String::new();
    let mut filtering = false;

    loop {
        let vis = visible_indices(items, &filter);
        if vis.is_empty() {
            state.select(None);
        } else if state.selected().map(|i| i >= vis.len()).unwrap_or(true) {
            state.select(Some(0));
        }

        terminal.draw(|frame| draw(frame, title, items, &vis, &mut state, &filter, filtering))?;
        if !event::poll(std::time::Duration::from_millis(200))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        if filtering {
            match key.code {
                KeyCode::Esc => {
                    if filter.is_empty() {
                        filtering = false;
                    } else {
                        filter.clear();
                    }
                }
                KeyCode::Enter => filtering = false,
                KeyCode::Backspace => {
                    filter.pop();
                }
                KeyCode::Char(c) => filter.push(c),
                KeyCode::Down => move_sel(&mut state, vis.len(), 1),
                KeyCode::Up => move_sel(&mut state, vis.len(), -1),
                _ => {}
            }
            continue;
        }

        match key.code {
            KeyCode::Char('q') => return Ok(false),
            KeyCode::Esc => return Ok(false),
            KeyCode::Char('/') => filtering = true,
            KeyCode::Down | KeyCode::Char('j') => move_sel(&mut state, vis.len(), 1),
            KeyCode::Up | KeyCode::Char('k') => move_sel(&mut state, vis.len(), -1),
            KeyCode::Char(' ') => {
                if let Some(vi) = state.selected() {
                    if let Some(&idx) = vis.get(vi) {
                        if !items[idx].locked {
                            items[idx].selected = !items[idx].selected;
                        }
                    }
                }
            }
            KeyCode::Char('a') => {
                for &idx in &vis {
                    if !items[idx].locked {
                        items[idx].selected = true;
                    }
                }
            }
            KeyCode::Char('n') => {
                for &idx in &vis {
                    items[idx].selected = false;
                }
            }
            KeyCode::Char('i') => {
                for &idx in &vis {
                    if !items[idx].locked {
                        items[idx].selected = !items[idx].selected;
                    }
                }
            }
            KeyCode::Enter => return Ok(true),
            _ => {}
        }
    }
}

fn move_sel(state: &mut ListState, len: usize, delta: i32) {
    if len == 0 {
        state.select(None);
        return;
    }
    let i = state.selected().unwrap_or(0) as i32;
    let next = (i + delta).rem_euclid(len as i32) as usize;
    state.select(Some(next));
}

fn draw(
    frame: &mut Frame,
    title: &str,
    items: &[PickItem],
    vis: &[usize],
    state: &mut ListState,
    filter: &str,
    filtering: bool,
) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Length(2),
        ])
        .split(area);

    let selected_bytes: u64 = items.iter().filter(|i| i.selected).map(|i| i.bytes).sum();
    let selected_count = items.iter().filter(|i| i.selected).count();
    let filter_note = if filter.is_empty() {
        String::new()
    } else {
        format!("  /{filter}")
    };
    let header = Paragraph::new(format!(
        "{title}  {} · {selected_count} selected{filter_note}",
        util::format_bytes(selected_bytes)
    ))
    .style(Style::default().fg(Color::Cyan).bold())
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, chunks[0]);

    let rows: Vec<ListItem> = vis
        .iter()
        .map(|&idx| {
            let item = &items[idx];
            let mark = if item.locked {
                "◎"
            } else if item.selected {
                "●"
            } else {
                "○"
            };
            let style = if item.locked {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White)
            };
            let title = util::truncate_right(&item.title, 32);
            let detail = util::truncate_left(&item.detail, 36);
            ListItem::new(Line::from(vec![
                Span::styled(format!(" {mark} {title:<32} "), style),
                Span::styled(
                    format!("{:>10}  ", util::format_bytes(item.bytes)),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(detail, Style::default().fg(Color::Gray)),
            ]))
        })
        .collect();

    let list = List::new(rows)
        .block(Block::default().borders(Borders::ALL).title(" Items "))
        .highlight_style(
            Style::default()
                .bg(Color::Indexed(238))
                .fg(Color::Cyan)
                .bold(),
        )
        .highlight_symbol("▶");
    frame.render_stateful_widget(list, chunks[1], state);

    let help = if filtering {
        format!("filter: {filter}_   type to search · enter done · esc clear")
    } else {
        "↑/↓  space select  a all  n none  i invert  / search  enter confirm  q quit".into()
    };
    frame.render_widget(
        Paragraph::new(help).style(Style::default().fg(Color::DarkGray)),
        chunks[2],
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_not_confirmed() {
        let mut items: Vec<PickItem> = vec![];
        assert!(!select_items("t", &mut items).unwrap());
    }

    #[test]
    fn filter_matches_title_and_detail() {
        let items = [
            PickItem {
                selected: false,
                locked: false,
                title: "firefox".into(),
                detail: "web browser".into(),
                bytes: 1,
            },
            PickItem {
                selected: false,
                locked: false,
                title: "vlc".into(),
                detail: "media player".into(),
                bytes: 1,
            },
        ];
        assert_eq!(visible_indices(&items, "fire"), vec![0]);
        assert_eq!(visible_indices(&items, "PLAYER"), vec![1]);
        assert_eq!(visible_indices(&items, "").len(), 2);
    }
}
