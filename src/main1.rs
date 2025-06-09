mod list;
mod tools;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, Mutex};
use signal_hook::iterator::Signals;
use signal_hook::consts::signal::{SIGINT, SIGHUP, SIGTERM, SIGQUIT, SIGUSR1, SIGUSR2};
use signal_hook::consts::signal::*;
use signal_hook::flag;

use crossterm::event::MouseEventKind;
use crossterm::{
    cursor::MoveTo,
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use list::Server;
use ratatui::widgets::ListState;

use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem},
};
use std::env;
use std::os::unix::process::CommandExt;
use std::time::{Duration, Instant};
use std::{
    io,
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
    },
};
use tui_textarea::TextArea;

fn main() -> io::Result<()> {
    let arg = env::args().nth(1); // 0 is the program name, so 1 is the first real argument
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = Arc::clone(&running);

    let mut signals = Signals::new(&[
        SIGINT,  // Ctrl+C
        SIGHUP,  // Terminal disconnect / hangup
        SIGTERM, // kill command
        SIGQUIT, // Ctrl+\ (Unix)
        SIGUSR1, // Custom user signal 1
        SIGUSR2, // Custom user signal 2
    ])?;
        let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("signals.log")?;

    let log_file_clone= Arc::new(Mutex::new(log_file));
std::thread::spawn(move || {
    for signal in signals.forever() {
        let mut file = log_file_clone.lock().unwrap();
        match signal {
            SIGINT | SIGTERM | SIGQUIT | SIGHUP => {
                writeln!(file, "Signal {} received. Exiting...", signal).unwrap();
                running_clone.store(false, Ordering::SeqCst); // Tell main to quit
                break;
            }
            SIGUSR1 => {
                writeln!(file, "SIGUSR1 received.").unwrap();
            }
            SIGUSR2 => {
                writeln!(file, "SIGUSR2 received.").unwrap();
            }
            _ => {
                writeln!(file, "Unhandled signal: {}", signal).unwrap();
            }
        }
    }
});

    if let Err(e) = enable_raw_mode() {
        eprintln!("Terminal doesn't support raw mode: {}", e);
        std::process::exit(1);
    }
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, arg, running.clone());

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
            let mut stdout = std::io::stdout();
            // Move cursor to the top-left corner
            execute!(stdout, MoveTo(0, 0)).unwrap();

            // Clear the entire screen
            execute!(stdout, Clear(ClearType::All)).unwrap();

            let _ = Command::new("ssh").arg(server.alias).exec();
            std::process::exit(1);
        }
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    arg: Option<String>,
    running: Arc<AtomicBool>,
) -> io::Result<Option<list::List>> {
    let config_file = Server::get_list();
    let (_, raw_list) = Server::parse_list(&config_file).unwrap();
    let list: Vec<list::List> = Server::hash_list(raw_list);
    let mut textarea = TextArea::default();
    let mut last_click_time: Option<Instant> = None;
    let mut last_click_position: Option<(u16, u16)> = None;
    let double_click_threshold = Duration::from_millis(300); // 300ms for a double-click
    let mut height = None;
    textarea.set_block(Block::default().title("Search").borders(Borders::ALL));
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
            binding.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
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
                .split(f.area());
            height = Some(chunks[0].height.saturating_sub(2));
            //binding.push(list::List::default(format!("{}, {}", chunks[0].height, chunks[1].height)));
            // Search Box/
            let list_items: Vec<ListItem> = binding
                .iter_mut()
                .enumerate()
                .map(|(i, item)| {
                    let style = match list_state.selected() {
                        Some(current) => {
                            if i == current {
                                Style::default().fg(Color::Yellow)
                            } else {
                                let set;
                                if current > i {
                                    set = current - i;
                                } else {
                                    set = i - current;
                                }
                                let dim_factor;
                                if (set as u32 * 20) < 80 {
                                    dim_factor = set as u8 * 20; // Dims each item more as the index increases
                                } else {
                                    dim_factor = 80;
                                }
                                let dim_color = Color::Rgb(
                                    255 - dim_factor,
                                    255 - dim_factor,
                                    255 - dim_factor,
                                ); // Dimmer color

                                Style::default().fg(dim_color)
                            }
                        }
                        None => Style::default().fg(Color::White),
                    };
                    ListItem::new(item.display_name.clone()).style(style)
                })
                .collect();

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

        if event::poll(std::time::Duration::from_millis(150))? {
            let event = event::read()?;
            if let Event::Mouse(mouse_event) = event {
                match mouse_event.kind {
                    MouseEventKind::Down(_) => {
                        let (x, y) = (mouse_event.column, mouse_event.row);

                        if y > 0 && y - 1 < height.expect("Idon't know") {
                            list_state
                                .select(Some((list_state.offset() + (y - 1) as usize) as usize));

                            let current_time = Instant::now();
                            if let Some(last_time) = last_click_time {
                                // Check if time difference between clicks is within double-click threshold
                                if current_time.duration_since(last_time) <= double_click_threshold
                                {
                                    if let Some((last_x, last_y)) = last_click_position {
                                        if last_x == x && last_y == y {
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
                        list_state.select_previous();
                    }
                    MouseEventKind::ScrollDown => {
                        list_state.select_next();
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
                        list_state.select_next();
                    }
                    KeyCode::Up => {
                        list_state.select_previous();
                    }
                    KeyCode::Home | KeyCode::PageUp => {
                        list_state.select(Some(list_state.offset()));
                    }
                    KeyCode::End | KeyCode::PageDown => {
                        list_state.select(Some(list_state.offset()));
                        if let Some(height) = height {
                            list_state.scroll_down_by(height - 1);
                        }
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
            let search_query = tools::persian_to_qwerty(&textarea.lines().join("\n"));
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
        if !running.load(Ordering::SeqCst) {
            return Ok(None);
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

        println!("{}: {}", name, char_extract(&name));
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

        println!("{}: {}", name, num_extract(&name));
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
