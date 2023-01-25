use std::{
    collections::VecDeque,
    sync::{Arc, Barrier},
    thread,
};

use clap::Parser;
use crossterm::event::{Event, KeyCode, KeyModifiers, MouseEventKind};
use tui::{layout::*, text::Text, widgets::*};

use sync_unsafe_cell::*;

use itertools::Itertools;

mod cpu;
mod mem;
mod motherboard;
mod msg;
mod pdb;
mod serial;
mod sync_unsafe_cell;

#[derive(Parser)]
#[command(name = "noontide-emu")]
#[command(author = "NyanCatTW1")]
#[command(about = "An emulator/debugger of the Noontide SUBLEQ Computer to aid in the development of related projects", long_about = None)]
struct Cli {
    #[arg(help = "Base path of a program, without the .bin")]
    base_path: String,

    #[arg(short = 'b')]
    #[arg(help = "Disable the TUI and read/write from stdin/stdout directly")]
    batch: bool,
}

fn main() {
    let cli = Cli::parse();
    let base_path = cli.base_path;

    // Load the .bin file into mem
    let mut mem = vec![0u8; 0x14000000];
    let mut bin_path = base_path.clone();
    bin_path.push_str(".bin");
    let data = std::fs::read(bin_path).unwrap();
    mem[..data.len()].copy_from_slice(&data);

    // Load the debug data from hex*, if any
    let mut debug_data: Option<pdb::DebugData> = None;
    for ext in ["hex0", "hex1", "hex2"] {
        let mut hex_path_str = base_path.clone();
        hex_path_str.push('.');
        hex_path_str.push_str(ext);
        let hex_path = std::path::Path::new(&hex_path_str);
        if !hex_path.exists() {
            continue;
        }

        debug_data = Some(pdb::parse_hex_file(
            &std::fs::read_to_string(hex_path).unwrap(),
        ));
    }

    // Set up the Arcs
    let mut handles = vec![];
    let mem_arc = Arc::new(SyncUnsafeCell::new(mem));
    let io_barrier_arc = Arc::new(Barrier::new(2));
    let cpu_barrier_arc = Arc::new(Barrier::new(2));

    // Set up the mpsc channels
    let (ui_sender, ui_receiver) = std::sync::mpsc::channel();
    let (serial_sender, serial_receiver) = std::sync::mpsc::channel();

    // Start the Serial thread
    {
        let mem = Arc::clone(&mem_arc);
        let io_barrier = Arc::clone(&io_barrier_arc);
        let sender = ui_sender.clone();
        handles.push(
            thread::Builder::new()
                .name("Serial".to_string())
                .spawn(move || {
                    serial::serial_loop(
                        unsafe { mem.get().as_mut().unwrap() },
                        io_barrier,
                        sender,
                        serial_receiver,
                    )
                })
                .unwrap(),
        );
    }

    // Start the CPU 0 thread
    {
        let mem = Arc::clone(&mem_arc);
        let cpu_barrier = Arc::clone(&cpu_barrier_arc);
        handles.push(
            thread::Builder::new()
                .name("CPU 0".to_string())
                .spawn(move || {
                    cpu::cpu_loop(
                        unsafe { mem.get().as_mut().unwrap() },
                        cpu_barrier,
                        ui_sender,
                    )
                })
                .unwrap(),
        );
    }

    // Start the Motherboard thread
    {
        let io_barrier = Arc::clone(&io_barrier_arc);
        let cpu_barrier = Arc::clone(&cpu_barrier_arc);
        handles.push(
            thread::Builder::new()
                .name("Motherboard".to_string())
                .spawn(move || {
                    motherboard::motherboard_loop(io_barrier, cpu_barrier);
                })
                .unwrap(),
        );
    }

    if !cli.batch {
        // Make crossterm exit itself upon panic
        let original_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic| {
            crossterm::terminal::disable_raw_mode().unwrap();
            crossterm::execute!(
                std::io::stdout(),
                crossterm::terminal::LeaveAlternateScreen,
                crossterm::event::DisableMouseCapture,
            )
            .unwrap();
            original_hook(panic);
        }));

        // Initialize crossterm
        crossterm::terminal::enable_raw_mode().unwrap();
        let mut stdout = std::io::stdout();
        crossterm::execute!(
            stdout,
            crossterm::terminal::EnterAlternateScreen,
            crossterm::event::EnableMouseCapture,
        )
        .unwrap();

        let backend = tui::backend::CrosstermBackend::new(stdout);
        let mut terminal = tui::Terminal::new(backend).unwrap();
        terminal.show_cursor().unwrap();

        let mut serial_out = String::new();
        let mut code_out = "CPU 0 is still starting...".to_owned();
        let mut debug_entries = VecDeque::new();

        let mut cur_window = 0;
        let window_names = vec!["Code", "Memory Dump", "Debug (CPU 0)"];
        let window_types = window_names.len();

        let mut scroll = (0, 0);
        let mut previous_char = '\0';
        let debug_lines: usize = 10;

        'main: loop {
            while let Ok(msg) = ui_receiver.try_recv() {
                match msg {
                    msg::UIMessage::Serial(c) => {
                        serial_out.push(c);
                    }
                    msg::UIMessage::SetEIP(eip) => {
                        code_out = pdb::render_debug(&debug_data, eip, (debug_lines / 2) as usize);
                    }
                    msg::UIMessage::Debug(str) => {
                        debug_entries.push_back(str);
                        if debug_entries.len() > debug_lines {
                            debug_entries.pop_front();
                        }
                    }
                }
            }

            {
                let serial_out = serial_out.clone();
                let code_out = code_out.clone();
                let mem_out = pdb::memory_dump(unsafe { mem_arc.get().as_ref().unwrap() });
                let debug_out = debug_entries.iter().join("");

                let window_name = window_names[cur_window];
                terminal
                    .draw(move |f| {
                        let chunks = Layout::default()
                            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                            .split(f.size());

                        let block = Block::default().title("Serial").borders(Borders::ALL);
                        f.render_widget(block, chunks[0]);
                        let p = Paragraph::new(Text::from(serial_out)).wrap(Wrap { trim: false });
                        f.render_widget(
                            p,
                            chunks[0].inner(&Margin {
                                horizontal: 1,
                                vertical: 1,
                            }),
                        );

                        let block = Block::default().title(window_name).borders(Borders::ALL);
                        f.render_widget(block, chunks[1]);

                        let p = if cur_window == 0 {
                            Paragraph::new(Text::from(code_out))
                                .wrap(Wrap { trim: false })
                                .scroll(scroll)
                        } else if cur_window == 1 {
                            Paragraph::new(Text::from(mem_out))
                                .wrap(Wrap { trim: false })
                                .scroll(scroll)
                        } else {
                            Paragraph::new(Text::from(debug_out))
                                .wrap(Wrap { trim: false })
                                .scroll(scroll)
                        };

                        f.render_widget(
                            p,
                            chunks[1].inner(&Margin {
                                horizontal: 1,
                                vertical: 1,
                            }),
                        );
                    })
                    .unwrap();
            }

            while crossterm::event::poll(std::time::Duration::from_millis(10)).unwrap() {
                match crossterm::event::read().unwrap() {
                    Event::Key(key) => match key.code {
                        KeyCode::Esc | KeyCode::Char('c')
                            if key.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            break 'main;
                        }
                        KeyCode::Left => {
                            scroll = (0, 0);
                            if cur_window != 0 {
                                cur_window -= 1;
                            } else {
                                cur_window = window_types - 1;
                            }
                        }
                        KeyCode::Right => {
                            scroll = (0, 0);
                            cur_window = (cur_window + 1) % window_types
                        }
                        KeyCode::Up => {
                            if scroll.0 != 0 {
                                scroll.0 -= 1;
                            }
                        }
                        KeyCode::Down => {
                            scroll.0 += 1;
                        }
                        KeyCode::Enter => {
                            // serial_out.push_str("\r\n");
                            serial_sender.send('\r').unwrap();
                            serial_sender.send('\n').unwrap();
                        }
                        KeyCode::Char(c) => {
                            if c == '\n' && previous_char != '\r' {
                                // serial_out.push('\r');
                                serial_sender.send('\r').unwrap();
                            }
                            // serial_out.push(c);
                            serial_sender.send(c).unwrap();
                            previous_char = c;
                        }
                        _ => {}
                    },
                    Event::Mouse(e) => {
                        if let MouseEventKind::ScrollUp = e.kind {
                            if scroll.0 != 0 {
                                scroll.0 -= 1;
                            }
                        } else if let MouseEventKind::ScrollDown = e.kind {
                            scroll.0 += 1;
                        }
                    }
                    _ => {}
                }
            }
        }

        // Exit crossterm cleanly
        crossterm::terminal::disable_raw_mode().unwrap();
        crossterm::execute!(
            terminal.backend_mut(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::event::DisableMouseCapture,
            crossterm::event::DisableBracketedPaste
        )
        .unwrap();
        terminal.show_cursor().unwrap();
    } else {
        panic!("Batch mode is not implemented yet");
    }
}
