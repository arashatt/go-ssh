mod list;
use crossterm::{
    cursor::{DisableBlinking, EnableBlinking, MoveTo, RestorePosition, SavePosition, Show},
    event::{self, MouseEvent, MouseButton, MouseEventKind, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use list::Server;
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use std::os::unix::process::CommandExt;
use std::thread;
use std::{
    io::{self, Write, stdout},
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use strsim::normalized_levenshtein;
fn main() -> io::Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, running.clone());

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
    } else {
        if let Some(server) = res.unwrap() {
            let mut ssh = Command::new("ssh")
                .arg(server.alias)
                .spawn()
                .expect("Failed to execute the command.");
            ssh.wait().expect("failed to wait for the process.");
            //println!("{:#?}", server);
            execute!(io::stdout(), Clear(ClearType::All), MoveTo(0, 0)).unwrap();
        }
    }
    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    running: Arc<AtomicBool>,
) -> io::Result<Option<list::List>> {
    let mut search_query = String::new();
    let mut cursor_pos = 0; // Track cursor position inside input
    let server = Server {};
    let config_file = Server::get_list();
    let (_, list) = Server::parse_list(&config_file).unwrap();
    let list = Server::hash_list(list);
    let answers = list;
    let mut filtered_answers = answers.clone();
    let mut selected_index: usize = 0;

    let visible_lines = 3; // Number of lines visible at a time

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(1),    // List of Answers
                    Constraint::Length(3), // Search Box
                ])
                .split(f.size());

            // Search Box
            let search_block = Block::default().title("Search").borders(Borders::ALL);
            let search_paragraph = Paragraph::new(Span::raw(search_query.as_str()))
                .block(search_block)
                .style(Style::default().fg(Color::White));

            // List of Answers
            let list_items: Vec<ListItem> = filtered_answers
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    let style = if i == selected_index {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    //ListItem::new(item.hostname.split(".").next().unwrap() ).style(style)
                    // For Debug:
                    ListItem::new(format!(
                        "{} : {}",
                        item.hostname.split(".").next().unwrap(),
                        item.score
                    ))
                    .style(style)
                })
                .collect();
            let list = List::new(list_items)
                .block(
                    Block::default()
                        .title("Limoo Host Servers")
                        .borders(Borders::ALL),
                )
                .highlight_symbol(">>")
                .highlight_style(
                    ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
                );

            f.render_widget(list, chunks[0]);
            f.render_widget(search_paragraph, chunks[1]);
            let cursor_x = chunks[1].x + cursor_pos as u16 + 1; // +1 for padding inside the block
            let cursor_y = chunks[1].y + 1;
            execute!(stdout(), Show, MoveTo(cursor_x, cursor_y)).unwrap();
        })?;

          // Capture mouse events

        if event::poll(std::time::Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(None);
                    }
                    KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(None);
                    }
                    KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        search_query.pop();
                        selected_index = 0; // Reset selection after filtering
                    }

                    KeyCode::Esc => return Ok(None),
                    KeyCode::Enter => {
                        if let Some(selected) = filtered_answers.get(selected_index) {}
                        break;
                    }
                    KeyCode::Up => {
                        if selected_index > 0 {
                            selected_index -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if selected_index < filtered_answers.len().saturating_sub(1) {
                            selected_index += 1;
                        }
                    }
                    KeyCode::Esc => break, // Exit on ESC
                    KeyCode::Enter => {
                        search_query.clear();
                        cursor_pos = 0;
                    }
                    KeyCode::Backspace => {
                        // Delete character at cursor position
                        if cursor_pos > 0 {
                            search_query.remove(cursor_pos - 1);
                            cursor_pos -= 1;
                            selected_index = 0;
                        }
                    }
                    KeyCode::Char(c) => {
                        // Insert character at cursor position
                        search_query.insert(cursor_pos, c);
                        cursor_pos += 1;
                        selected_index = 0;
                    }
                    KeyCode::Left => {
                        // Move cursor left
                        if cursor_pos > 0 {
                            cursor_pos -= 1;
                        }
                    }
                    KeyCode::Right => {
                        // Move cursor right
                        if cursor_pos < search_query.len() {
                            cursor_pos += 1;
                        }
                    }
                    _ => {}
                }
                if let event::Event::Mouse(MouseEvent {
                kind,
                column,
                row,
                ..
            }) = event::read()?
            {
                match kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        // Check if the mouse click is within the list items
                        return Ok(None);
                        if row >= 1 && row < 5 { // List range based on our items
                            selected_index = (row - 1) as usize; // Adjust index to the list range
                        }
                    }
                    _ => {}
                }
            } 
                // Filter answers based on the search query
                //  filtered_answers = answers
                //      .iter()
                //      .filter(|a| a.hostname.to_lowercase().contains(&search_query.to_lowercase()))
                //      .cloned()
                //      .collect();
                filtered_answers = answers.clone();
                for item in &mut filtered_answers {
                    //item.score = normalized_damerau_levenshtein(&search_query, &item.hostname);
                    //item.score = normalized_levenshtein(&search_query, &item.hostname);
                    item.score = strsim::jaro_winkler(&search_query, &item.hostname);
                }
                filtered_answers.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
            }
        }
    }

    Ok(Some(filtered_answers[selected_index].clone()))
}
