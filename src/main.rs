use std::sync::Arc;

use crossterm::event::{Event, KeyCode, KeyModifiers, MouseEventKind};
use tui::{layout::*, text::Text, widgets::*};

use sync_unsafe_cell::*;

mod cpu;
mod mem;
mod msg;
mod serial;
mod sync_unsafe_cell;

fn main() {
    let original_hook = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |panic| {
        crossterm::terminal::disable_raw_mode().unwrap();
        crossterm::execute!(
            std::io::stdout(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::event::DisableMouseCapture,
            crossterm::event::DisableBracketedPaste
        )
        .unwrap();
        original_hook(panic);
    }));

    crossterm::terminal::enable_raw_mode().unwrap();
    let mut stdout = std::io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture,
        crossterm::event::EnableBracketedPaste
    )
    .unwrap();
    let backend = tui::backend::CrosstermBackend::new(stdout);
    let mut terminal = tui::Terminal::new(backend).unwrap();
    terminal.show_cursor().unwrap();

    let mut mem = vec![0u8; 0x14000000];
    let data = std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
    mem[..data.len()].copy_from_slice(&data);

    let mut handles = vec![];
    let arc = Arc::new(SyncUnsafeCell::new(mem));
    let (sender, receiver) = std::sync::mpsc::channel();
    let (serial_sender, serial_receiver) = std::sync::mpsc::channel();

    {
        let mem = Arc::clone(&arc);
        let sender = sender.clone();
        handles.push(std::thread::spawn(move || {
            serial::serial_loop(
                unsafe { mem.get().as_mut().unwrap() },
                sender,
                serial_receiver,
            )
        }));
    }

    let mem = Arc::clone(&arc);
    handles.push(std::thread::spawn(move || {
        cpu::cpu_loop(unsafe { mem.get().as_mut().unwrap() }, sender)
    }));

    let mut serial_out = String::new();
    let mut debug_out = String::new();

    let mut scroll = (0, 0);
    let mut previous_char = '\0';

    'main: loop {
        while let Ok(msg) = receiver.try_recv() {
            match msg {
                msg::UIMessage::Serial(c) => {
                    serial_out.push(c);
                }
                msg::UIMessage::Debug(s) => {
                    debug_out.push_str(&s);
                }
            }
        }

        {
            let serial_out = serial_out.clone();
            let debug_out = debug_out.clone();
            terminal
                .draw(move |f| {
                    let chunks = Layout::default()
                        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
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

                    let block = Block::default().title("Debug").borders(Borders::ALL);
                    f.render_widget(block, chunks[1]);
                    let p = Paragraph::new(Text::from(debug_out))
                        .wrap(Wrap { trim: false })
                        .scroll(scroll);
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
                        scroll.1 -= 1;
                    }
                    KeyCode::Right => {
                        scroll.1 += 1;
                    }
                    KeyCode::Up => {
                        scroll.0 -= 1;
                    }
                    KeyCode::Down => {
                        scroll.0 += 1;
                    }
                    KeyCode::Enter => {
                        serial_out.push_str("\r\n");
                        serial_sender.send('\r').unwrap();
                        serial_sender.send('\n').unwrap();
                    }
                    KeyCode::Char(c) => {
                        if c == '\n' && previous_char != '\r' {
                            serial_out.push('\r');
                            serial_sender.send('\r').unwrap();
                        }
                        serial_out.push(c);
                        serial_sender.send(c).unwrap();
                        previous_char = c;
                    }
                    _ => {}
                },
                Event::Paste(s) => {
                    for c in s.chars() {
                        if c == '\n' && previous_char != '\r' {
                            serial_out.push('\r');
                            serial_sender.send('\r').unwrap();
                        }
                        serial_out.push(c);
                        serial_sender.send(c).unwrap();
                        previous_char = c;
                    }
                }
                Event::Mouse(e) => {
                    if let MouseEventKind::ScrollUp = e.kind {
                        scroll.0 -= 1;
                    } else if let MouseEventKind::ScrollDown = e.kind {
                        scroll.0 += 1;
                    }
                }
                _ => {}
            }
        }
    }

    crossterm::terminal::disable_raw_mode().unwrap();
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture,
        crossterm::event::DisableBracketedPaste
    )
    .unwrap();
    terminal.show_cursor().unwrap();
}
