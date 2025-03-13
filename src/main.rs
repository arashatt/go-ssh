//mod list;
//use list::Server;
use std::{
    io,
    sync::{atomic::{AtomicBool, Ordering}, Arc},
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};

fn main() -> io::Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

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
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, running: Arc<AtomicBool>) -> io::Result<()> {
    let mut search_query = String::new();
    let answers = vec![
        "Rust is a systems programming language.",
        "Cargo is Rust’s package manager.",
        "Rust has powerful ownership and borrowing rules.",
        "Tokio is an async runtime for Rust.",
        "Rust is great for safety and performance.",
    ];
    let mut filtered_answers = answers.clone();
    let mut selected_index: usize = 0;

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
            let search_paragraph = Paragraph::new(search_query.as_str())
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
                    ListItem::new(item.clone()).style(style)
                })
                .collect();
            let list = List::new(list_items).block(Block::default().title("Limoo Host Servers").borders(Borders::ALL));
            f.render_widget(list, chunks[0]);
            f.render_widget(search_paragraph, chunks[1]);
        })?;

        if event::poll(std::time::Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    KeyCode::Esc => break,
                    KeyCode::Enter => {
                        if let Some(selected) = filtered_answers.get(selected_index) {
                            println!("Selected: {}", selected);
                        }
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
                    KeyCode::Backspace => {
                        search_query.pop();
                    }
                    KeyCode::Char(c) => {
                        search_query.push(c);
                    }
                    _ => {}
                }

                // Filter answers based on the search query
                filtered_answers = answers
                    .iter()
                    .filter(|a| a.to_lowercase().contains(&search_query.to_lowercase()))
                    .cloned()
                    .collect();
                selected_index = 0; // Reset selection after filtering
            }
        }
    }
    Ok(())
}

