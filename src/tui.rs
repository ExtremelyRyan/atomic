use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};
use std::io;

/// Launches the atomic TUI dashboard
pub fn start_tui(commands: Vec<String>, branch: &str, changes: usize) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    let backend = CrosstermBackend::new(&mut stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut selected = 0;
    let mut list_state = ListState::default();
    list_state.select(Some(selected)); // initial selection

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Length(2),
                        Constraint::Min(5),
                    ]
                    .as_ref(),
                )
                .split(f.size());

            // Branch and pending changes
            let top = Paragraph::new(format!("Branch: {} | Pending changes: {}", branch, changes))
                .block(
                    Block::default()
                        .title("atomic status")
                        .borders(Borders::ALL),
                );
            f.render_widget(top, chunks[0]);

            // Instructions
            let inst = Paragraph::new("↑/↓ select command | Enter: run | q: quit")
                .style(Style::default().add_modifier(Modifier::ITALIC));
            f.render_widget(inst, chunks[1]);

            // Command list
            let items: Vec<ListItem> = commands.iter().map(|c| ListItem::new(c.clone())).collect();
            let list = List::new(items)
                .block(Block::default().title("Commands").borders(Borders::ALL))
                .highlight_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("▶ ");
            f.render_stateful_widget(list, chunks[2], &mut list_state);
        })?;

        // Only update selection *on key events*
        if event::poll(std::time::Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Down => {
                        selected = if selected >= commands.len() - 1 {
                            0
                        } else {
                            selected + 1
                        };
                        list_state.select(Some(selected));
                    }
                    KeyCode::Up => {
                        selected = if selected == 0 {
                            commands.len() - 1
                        } else {
                            selected - 1
                        };
                        list_state.select(Some(selected));
                    }
                    KeyCode::Enter => {
                        let command = &commands[selected];
                        // TODO: Replace this with real execution
                        show_popup(&terminal, &format!("Would run command: {command}"))?;
                    }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    Ok(())
}

/// Displays a quick popup message in the center of the terminal.
fn show_popup(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    message: &str,
) -> io::Result<()> {
    terminal.draw(|f| {
        let area = centered_rect(40, 5, f.size());
        let block = Block::default()
            .title("Info")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green));
        let paragraph = Paragraph::new(message).block(block);
        f.render_widget(paragraph, area);
    })?;
    std::thread::sleep(std::time::Duration::from_millis(1000));
    Ok(())
}

/// Helper for popup positioning
fn centered_rect(
    percent_x: u16,
    percent_y: u16,
    r: ratatui::layout::Rect,
) -> ratatui::layout::Rect {
    use ratatui::layout::{Alignment, Rect};
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);
    let vertical = popup_layout[1];

    let horizontal_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(vertical);

    horizontal_layout[1]
}
