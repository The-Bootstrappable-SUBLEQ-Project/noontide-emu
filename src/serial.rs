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

        let out = crate::mem::read(mem, SERIAL_OUT);
        if out != 0 {
            if ui_sender
                .send(UIMessage::Serial(char::from_u32((out - 1) as u32).unwrap()))
                .is_err()
            {
                break;
            }

            crate::mem::write(mem, SERIAL_OUT, &i64::to_be_bytes(0));
        }
        io_barrier.wait();
    }
}
