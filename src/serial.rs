use std::{
    collections::VecDeque,
    sync::{
        mpsc::{Receiver, Sender},
        Arc, Barrier,
    },
};

use bus::BusReader;

use crate::msg::UIMessage;

const SERIAL_CONNECTED: usize = 0x13ED27E0;
const SERIAL_IN: usize = 0x13ED27E8;
const SERIAL_OUT: usize = 0x13ED27F0;

pub fn serial_loop(
    mem: &mut [u8],
    io_barrier: Arc<Barrier>,
    ui_sender: Sender<UIMessage>,
    serial_receiver: Receiver<char>,
    mut term_rx: BusReader<usize>,
) {
    crate::mem::write(mem, SERIAL_CONNECTED, &i64::to_be_bytes(1));
    let mut input_buffer: VecDeque<char> = VecDeque::new();

    loop {
        io_barrier.wait();

        if let Ok(_val) = term_rx.try_recv() {
            io_barrier.wait();
            // eprintln!("Serial exited");
            return;
        }

        while let Ok(input) = serial_receiver.try_recv() {
            input_buffer.push_back(input);
        }

        if !input_buffer.is_empty() && crate::mem::read(mem, SERIAL_IN) == 0 {
            crate::mem::write(
                mem,
                SERIAL_IN,
                &i64::to_be_bytes(input_buffer.pop_front().unwrap() as i64 + 1),
            );
        }

        let mut out: u64 = crate::mem::read(mem, SERIAL_OUT) as u64;
        if out != 0 {
            out -= 1;
            if out > 255 {
                eprintln!("Bad serial output: {:#x}", out);
            } else if ui_sender
                .send(UIMessage::Serial(out.try_into().unwrap()))
                .is_err()
            {
                break;
            }

            crate::mem::write(mem, SERIAL_OUT, &i64::to_be_bytes(0));
        }
        io_barrier.wait();
    }
}
