mod list;
use crossterm::event::MouseEventKind;
use crossterm::{
    cursor::{DisableBlinking, EnableBlinking, MoveTo, RestorePosition, SavePosition},
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use list::Server;
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use std::env;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::os::unix::process::CommandExt;
use std::time::{Duration, Instant};
use std::{
    io,
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use strsim::normalized_damerau_levenshtein;
use strsim::normalized_levenshtein;
use tui_textarea::TextArea;

fn main() -> io::Result<()> {
    let arg = env::args().nth(1); // 0 is the program name, so 1 is the first real argument
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableMouseCapture
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, running.clone(), arg);

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
            let _ = Command::new("ssh").arg(server.alias).exec();
            //println!("{:#?}", server);
            terminal.clear();
        }
    }
    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    running: Arc<AtomicBool>,
    arg: Option<String>,
) -> io::Result<Option<list::List>> {
    let mut search_query = String::new();
    let server = Server {};
    let config_file = Server::get_list();
    let (_, list) = Server::parse_list(&config_file).unwrap();
    let list = Server::hash_list(list);

    let answers = list;
    let mut filtered_answers = answers.clone();
    let mut selected_index: usize = 0;
    let mut textarea = TextArea::default();
    let mut last_click_time: Option<Instant> = None;
    let mut last_click_position: Option<(u16, u16)> = None;
    let double_click_threshold = Duration::from_millis(300); // 300ms for a double-click
    textarea.set_block(Block::default().title("Search").borders(Borders::ALL));
    match arg {
        Some(argument) => {
            textarea.insert_str(argument);
        }
        _ => {}
    }

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
                    ListItem::new(item.hostname.split(".").next().unwrap()).style(style)
                })
                .collect();
            let list = List::new(list_items).block(
                Block::default()
                    .title("Limoo Host Servers")
                    .borders(Borders::ALL),
            );

            f.render_widget(list, chunks[0]);
            // f.render_widget(search_paragraph, chunks[1]);
            f.render_widget(&textarea, chunks[1]);
        })?;

        if event::poll(std::time::Duration::from_millis(20))? {
            let event = event::read()?;
            if let Event::Mouse(mouse_event) = event {
                match mouse_event.kind {
                    MouseEventKind::Down(_) => {
                        let (x, y) = (mouse_event.column, mouse_event.row);

                        // Write a message to the socket
                        //            pipe.write_all(format!("x:{} y:{}\n", x, y).as_bytes())?;
                        selected_index = (y - 1) as usize;

                        let current_time = Instant::now();
                        if let Some(last_time) = last_click_time {
                            // Check if time difference between clicks is within double-click threshold
                            if current_time.duration_since(last_time) <= double_click_threshold {
                                if let Some((last_x, last_y)) = last_click_position {
                                    if (last_x == x && last_y == y) {
                                        // Double-click detected on the same position
                                        return Ok(Some(filtered_answers[selected_index].clone()));
                                    }
                                }
                            }
                        }

                        // Update the last click time and position
                        last_click_time = Some(current_time);
                        last_click_position = Some((x, y));

                        // You need to determine if (x, y) is within the list widget.
                        // If so, map `y` to list index and update your selection.
                    }
                    MouseEventKind::ScrollUp => {
                        if selected_index > 0 {
                        selected_index -= 1;
                        }
                    }
                    MouseEventKind::ScrollDown => {
                        if selected_index < filtered_answers.len().saturating_sub(1) {
                        selected_index += 1;
                        }
                    }
                    _ => {}
                }
            } else if let Event::Key(key) = event {
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
                    _ => {}
                }
                textarea.input(key);
                let search_query = textarea.lines().join("\n");
                // Filter answers based on the search query
                filtered_answers = answers
                    .iter()
                    .filter(|a| {
                        num_extract(a.hostname.split(".").next().unwrap())
                            .contains(&num_extract(&search_query))
                            && char_extract(a.hostname.split(".").next().unwrap())
                                .contains(&char_extract(&search_query))
                    })
                    .cloned()
                    .collect();
                for item in &mut filtered_answers {
                    //   item.score = normalized_damerau_levenshtein(
                    //       &search_query,
                    //     &item.hostname.split(".").next().unwrap(),
                    //   );
                    item.score = strsim::jaro_winkler(&search_query, &item.hostname);
                }
                filtered_answers.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
            }
        }
    }

    Ok(Some(filtered_answers[selected_index].clone()))
}

fn num_extract(name: &str) -> String {
    name.chars()
        .filter(|a| *a >= '0' && *a <= '9')
        .collect::<String>()
        .to_owned()
}
fn char_extract(name: &str) -> String {
    name.chars()
        .filter(|a| (*a >= 'a' && *a <= 'z') || (*a >= 'A' && *a <= 'Z'))
        .collect::<String>()
        .to_owned()
}

#[test]
fn char_extract_test() {
    let server = Server {};
    let config_file = Server::get_list();
    let (_, list) = Server::parse_list(&config_file).unwrap();

    let list = Server::hash_list(list);
    for i in list {
        let name = i.hostname.split(".").next().unwrap();

        println!("{}: {}", name, char_extract(name));
    }
}
#[test]
fn num_extract_test() {
    let server = Server {};
    let config_file = Server::get_list();
    let (_, list) = Server::parse_list(&config_file).unwrap();

    let list = Server::hash_list(list);
    for i in list {
        let name = i.hostname.split(".").next().unwrap();

        println!("{}: {}", name, num_extract(name));
    }
}
/*
the right way to implement the functionality of gossh is to split
numbers from alphabet charcters in the input.
For example 100ooz input definitely should have highest score with yooz100,
but since the number is in the beginning, it matches pirouz100
*/
