mod list;
use list::Server;
use std::{
    io,
    sync::{atomic::{AtomicBool, Ordering}, Arc},
    process::Command
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
cursor::{DisableBlinking, EnableBlinking, MoveTo, RestorePosition, SavePosition}
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph },
    Terminal,
};
use strsim::normalized_levenshtein;
use std::os::unix::process::CommandExt;

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
    }else{
if let Some(server) = res.unwrap(){
let _ = Command::new("ssh").arg(server.alias).exec();
    }
    }
    terminal.clear();
    println!("Done");
    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, running: Arc<AtomicBool>) -> io::Result<Option<list::List>> {
    let mut search_query = String::new();
    let server = Server{};
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
                    //ListItem::new(item.hostname.split(".").next().unwrap() ).style(style)
                    // For Debug:
                    ListItem::new(format!("{} : {}", item.hostname.split(".").next().unwrap(), item.score) ).style(style)
                })
                .collect();
            let list = List::new(list_items).block(Block::default().title("Limoo Host Servers").borders(Borders::ALL));
            

            f.render_widget(list, chunks[0]);
            f.render_widget(search_paragraph, chunks[1]);
        })?;

        if event::poll(std::time::Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return Ok(None),
                    KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => return Ok(None),
                    KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        search_query.pop();
                        selected_index = 0; // Reset selection after filtering
                    }

                    KeyCode::Esc => return Ok(None),
                    KeyCode::Enter => {
                        if let Some(selected) = filtered_answers.get(selected_index) {
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
                        selected_index = 0; // Reset selection after filtering
                    }
                    KeyCode::Char(c) => {
                        search_query.push(c);
                        selected_index = 0; // Reset selection after filtering
                    }
                    _ => {}
                }

                // Filter answers based on the search query
              //  filtered_answers = answers
              //      .iter()
              //      .filter(|a| a.hostname.to_lowercase().contains(&search_query.to_lowercase()))
              //      .cloned()
              //      .collect();
            filtered_answers = answers.clone();
            for item in &mut filtered_answers{
                //item.score = normalized_damerau_levenshtein(&search_query, &item.hostname);
                item.score = normalized_levenshtein(&search_query, &item.hostname);
                item.score = strsim::jaro_winkler(&search_query, &item.hostname);
            }
           filtered_answers.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap() );

            }
        }
    }

    Ok(Some(filtered_answers[selected_index].clone()))
}

