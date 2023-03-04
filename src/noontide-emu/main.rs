use std::{
    collections::{HashMap, VecDeque},
    fs::File,
    io::{BufWriter, Write},
    sync::{Arc, Barrier},
    thread,
};

use bus::Bus;
use clap::Parser;
use crossterm::event::{Event, KeyCode, KeyModifiers, MouseEventKind};
use tui::{layout::*, text::Text, widgets::*};

use sync_unsafe_cell::*;

use bincode::serialize_into;
use itertools::Itertools;

mod cpu;
mod mem;
mod motherboard;
mod msg;
use noontide_emu::pdb;
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
    #[arg(help = "Disable the TUI, read input from the input file, and output to stdout")]
    batch_input: Option<String>,

    #[arg(short = 'r')]
    #[arg(
        help = "Record processor EIPs into a file, which can later be analyzed with noontide-perf"
    )]
    record_path: Option<String>,
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

    let record_eips = cli.record_path.is_some();
    let mut recorded_eips: HashMap<u64, u64> = HashMap::new();

    // Set up the Arcs
    let mut handles = vec![];
    let mem_arc = Arc::new(SyncUnsafeCell::new(mem));
    let io_barrier_arc = Arc::new(Barrier::new(2));
    let cpu_barrier_arc = Arc::new(Barrier::new(2));

    // Set up the mpsc channels
    let (ui_sender, ui_receiver) = std::sync::mpsc::channel();
    let (serial_sender, serial_receiver) = std::sync::mpsc::channel();
    let (mb1_sender, mb1_receiver) = std::sync::mpsc::channel();
    let (mb2_sender, mb2_receiver) = std::sync::mpsc::channel();

    // Set up the broadcast bus for stopping threads
    let mut term_tx: Bus<usize> = Bus::new(10);
    let term_rx_serial = term_tx.add_rx();
    let term_rx_cpu0 = term_tx.add_rx();

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
                        term_rx_serial,
                    )
                })
                .unwrap(),
        );
    }

    #[cfg(feature = "debugger")]
    let mut cycle_length = 1;

    //  512: 16.3575 +- 0.0710 seconds time elapsed  ( +-  0.43% )
    // 1024: 16.3151 +- 0.0483 seconds time elapsed  ( +-  0.30% )
    // 2048: 16.4346 +- 0.0623 seconds time elapsed  ( +-  0.38% )
    // 4096: 16.5468 +- 0.0562 seconds time elapsed  ( +-  0.34% )
    #[cfg(not(feature = "debugger"))]
    let mut cycle_length = 4096;

    if record_eips {
        cycle_length = std::cmp::min(cycle_length, 100);
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
                        0,
                        cpu_barrier,
                        ui_sender,
                        term_rx_cpu0,
                        cycle_length,
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
                    motherboard::motherboard_loop(
                        io_barrier,
                        cpu_barrier,
                        mb1_receiver,
                        mb2_sender,
                    );
                })
                .unwrap(),
        );
    }

    let mut cpus_running = 1;
    match cli.batch_input {
        None => {
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

            let mut code_out = "CPU 0 is still starting...".to_owned();
            let mut debug_entries = VecDeque::new();

            let mut cur_window = 0;
            let window_names = vec!["Code", "Memory Dump", "Debug (CPU 0)"];
            let window_types = window_names.len();

            let mut scroll = (0, 0);
            let mut previous_char = '\0';
            let debug_lines: usize = 10;

            let mut serial_out = String::new();
            'main: loop {
                if let Ok(msg) = ui_receiver.recv() {
                    match msg {
                        msg::UIMessage::Serial(c) => {
                            serial_out.push(c as char);
                        }
                        msg::UIMessage::SetEIP(eip) => {
                            code_out = pdb::render_debug(
                                &debug_data,
                                eip,
                                (debug_lines / 2) as usize,
                                false,
                            );

                            if record_eips {
                                *recorded_eips.entry(eip).or_insert(0) += 1;
                            }
                        }
                        msg::UIMessage::Debug(_eip, str) => {
                            debug_entries.push_back(str);
                            if debug_entries.len() > debug_lines {
                                debug_entries.pop_front();
                            }
                        }
                        msg::UIMessage::CPUStarted(_cpu_id) => {
                            cpus_running += 1;
                        }
                        msg::UIMessage::CPUStopped(_cpu_id) => {
                            cpus_running -= 1;
                            if cpus_running == 0 {
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

                                break;
                            }
                        }
                    }
                } else {
                    panic!("ui_receiver failed");
                }

                {
                    let serial_out = serial_out.clone();
                    let code_out = code_out.clone();
                    let mem_out = pdb::memory_dump(unsafe { mem_arc.get().as_ref().unwrap() });
                    let debug_out = debug_entries.iter().join("\r\n");

                    let window_name = window_names[cur_window];
                    terminal
                        .draw(move |f| {
                            let chunks = Layout::default()
                                .constraints([
                                    Constraint::Percentage(50),
                                    Constraint::Percentage(50),
                                ])
                                .split(f.size());

                            let block = Block::default().title("Serial").borders(Borders::ALL);
                            f.render_widget(block, chunks[0]);
                            let p =
                                Paragraph::new(Text::from(serial_out)).wrap(Wrap { trim: false });
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
        }
        Some(batch_input) => {
            let input_data = std::fs::read(batch_input).unwrap();
            for chr in input_data {
                serial_sender.send(chr as char).unwrap();
            }

            loop {
                if let Ok(msg) = ui_receiver.recv() {
                    match msg {
                        msg::UIMessage::Debug(eip, dat) => {
                            eprintln!(
                                "{}\n{}\n",
                                dat,
                                pdb::render_debug(&debug_data, eip, 2, true)
                            );
                        }
                        msg::UIMessage::Serial(c) => {
                            std::io::stdout()
                                .write_all(std::slice::from_ref(&c))
                                .unwrap();
                            std::io::stdout().flush().unwrap();
                        }
                        msg::UIMessage::SetEIP(eip) => {
                            if record_eips {
                                *recorded_eips.entry(eip).or_insert(0) += 1;
                            }
                        }
                        msg::UIMessage::CPUStarted(_cpu_id) => {
                            cpus_running += 1;
                        }
                        msg::UIMessage::CPUStopped(_cpu_id) => {
                            cpus_running -= 1;
                            if cpus_running == 0 {
                                break;
                            }
                        }
                    }
                } else {
                    panic!("ui_receiver failed");
                }
            }
        }
    }

    // Start shutdown procedure
    mb1_sender.send(0).unwrap();
    // Wait for Motherboard's acknowledgement
    mb2_receiver.recv().unwrap();
    // Send termination broadcast
    term_tx.broadcast(0);
    // Let the Motherboard know that the broadcast has been sent
    mb1_sender.send(0).unwrap();

    for thread in handles {
        thread.join().unwrap();
    }

    if let Some(record_path) = cli.record_path {
        let mut f = BufWriter::new(File::create(record_path).unwrap());
        serialize_into(&mut f, &recorded_eips).unwrap();
    }
}
