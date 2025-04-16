mod list;
use crossterm::event::MouseEventKind;
use crossterm::{
    cursor::{DisableBlinking, EnableBlinking, MoveTo, RestorePosition, SavePosition},
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use list::Server;
use ratatui::widgets::ListState;
use ratatui::widgets::Padding;
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
use std::thread;
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

            //        println!("{:#?}", server);
        }
    }

    terminal.clear()?;

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
    let (_, raw_list) = Server::parse_list(&config_file).unwrap();
    let list: Vec<list::List> = Server::hash_list(raw_list);
    let mut textarea = TextArea::default();
    let mut last_click_time: Option<Instant> = None;
    let mut last_click_position: Option<(u16, u16)> = None;
    let double_click_threshold = Duration::from_millis(300); // 300ms for a double-click
    textarea.set_block(Block::default().title("Search").borders(Borders::ALL));
    let search_block = Block::default().title("Search").borders(Borders::ALL);
    let mut list_state = ListState::default();
    list_state.select(Some(0)); // Start with first item selected

    let mut binding = list.clone();
    match arg {
        Some(argument) => {
            textarea.insert_str(argument);
            let search_query = textarea.lines().join("\n");
            binding = binding
                .iter()
                .filter(|a| {
                    num_extract(&a.display_name).contains(&num_extract(&search_query))
                        && char_extract(&a.display_name).contains(&char_extract(&search_query))
                })
                .cloned()
                .collect();
            for item in &mut binding {
                //   item.score = normalized_damerau_levenshtein(
                //       &search_query,
                //     &item.hostname.split(".").next().unwrap(),
                //   );
                item.score = strsim::jaro_winkler(&search_query, &item.display_name);
            }
            binding.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
            //println!("{}",format!("{:#?}", search_query).chars().filter(|c| !c.is_whitespace()).collect::<String>());
            //std::thread::sleep(std::time::Duration::from_millis(3000));

            list_state.select(Some(0)); // Start with first item selected
        }
        _ => {}
    }

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(1),    // List of Answers
                    Constraint::Length(3), // Search Box
                ])
                .split(f.size());

            // Search Box/
            let list_items: Vec<ListItem> = binding
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    let style = if i == list_state.selected().unwrap() {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    ListItem::new(item.display_name.clone()).style(style)
                })
                .collect();

            // List of Answers
            let widget_list = List::new(list_items.clone())
                .block(
                    Block::default()
                        .title("Limoo Host Servers")
                        .borders(Borders::ALL),
                )
                .highlight_symbol(">> ");

            //f.render_widget(list, chunks[0]);
            f.render_stateful_widget(widget_list, chunks[0], &mut list_state);
            f.render_widget(&textarea, chunks[1]);
        })?;

        if event::poll(std::time::Duration::from_millis(200))? {
            let event = event::read()?;
            if let Event::Mouse(mouse_event) = event {
                match mouse_event.kind {
                    MouseEventKind::Down(_) => {
                        let (x, y) = (mouse_event.column, mouse_event.row);

                        // Write a message to the socket
                        if y > 0 {
                            list_state.select(Some((y - 1) as usize));

                            let current_time = Instant::now();
                            if let Some(last_time) = last_click_time {
                                // Check if time difference between clicks is within double-click threshold
                                if current_time.duration_since(last_time) <= double_click_threshold
                                {
                                    if let Some((last_x, last_y)) = last_click_position {
                                        if (last_x == x && last_y == y) {
                                            // Double-click detected on the same position
                                            //return Ok(Some(filtered_answers[selected_index].clone()));
                                            break;
                                        }
                                    }
                                }
                            }

                            // Update the last click time and position
                            last_click_time = Some(current_time);
                            last_click_position = Some((x, y));
                        }
                        // You need to determine if (x, y) is within the list widget.
                        // If so, map `y` to list index and update your selection.
                    }
                    MouseEventKind::ScrollUp => {
                        let i = match list_state.selected() {
                            Some(i) => {
                                if i == 0 {
                                    binding.len() - 1
                                } else {
                                    i - 1
                                }
                            }
                            None => 0,
                        };
                        list_state.select(Some(i));
                    }
                    MouseEventKind::ScrollDown => {
                        let i = match list_state.selected() {
                            Some(i) => {
                                if i >= binding.len() - 1 {
                                    0
                                } else {
                                    i + 1
                                }
                            }
                            None => 0,
                        };
                        list_state.select(Some(i));
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
                    //KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    //    return Ok(None);
                    //}
                    KeyCode::Esc => return Ok(None),
                    KeyCode::Enter => {
                        if binding.len() == 0 {
                            continue;
                        }
                        break;
                    }
                    KeyCode::Down => {
                        let i = match list_state.selected() {
                            Some(i) => {
                                if i >= binding.len() - 1 {
                                    0
                                } else {
                                    i + 1
                                }
                            }
                            None => 0,
                        };
                        list_state.select(Some(i));
                    }
                    KeyCode::Up => {
                        let i = match list_state.selected() {
                            Some(i) => {
                                if i == 0 {
                                    binding.len() - 1
                                } else {
                                    i - 1
                                }
                            }
                            None => 0,
                        };
                        list_state.select(Some(i));
                    }

                    KeyCode::Backspace if !textarea.lines().join("").is_empty() => {
                        list_state.select(Some(0));
                    }
                    KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if !textarea.lines().join("").is_empty() {
                            list_state.select(Some(0));
                        }
                    }
                    KeyCode::Char(_) => {
                        list_state.select(Some(0));
                    }


                    _ => {}
                }

                textarea.input(key);
            }
            let search_query = textarea.lines().join("\n");
            // Filter answers based on the search query
            binding = list
                .clone()
                .iter()
                .filter(|a| {
                    num_extract(&a.display_name).contains(&num_extract(&search_query))
                        && char_extract(&a.display_name).contains(&char_extract(&search_query))
                })
                .cloned()
                .collect();
            for item in &mut binding {
                //   item.score = normalized_damerau_levenshtein(
                //       &search_query,
                //     &item.hostname.split(".").next().unwrap(),
                //   );
                item.score = strsim::jaro_winkler(&search_query, &item.display_name);
            }
            binding.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        }
    }
    match list_state.selected() {
        Some(i) => Ok(Some(binding[i].clone())),
        None => Ok(None),
    }
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
        let name = i.display_name;

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
        let name = i.display_name;

        println!("{}: {}", name, num_extract(name));
    }
}

/*
the right way to implement the functionality of gossh is to split
numbers from alphabet charcters in the input.
For example 100ooz input definitely should have highest score with yooz100,
but since the number is in the beginning, it matches pirouz100
*/

/*
TODO: mouse click resets from beginning, backspace when input is empty

*/
