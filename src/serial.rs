use std::sync::mpsc::{Receiver, Sender};

use crate::msg::UIMessage;

const SERIAL_CONNECTED: usize = 0x13ED27E0;
const SERIAL_IN: usize = 0x13ED27E8;
const SERIAL_OUT: usize = 0x13ED27F0;

pub fn serial_loop(mem: &mut [u8], sender: Sender<UIMessage>, receiver: Receiver<char>) {
    crate::mem::write(mem, SERIAL_CONNECTED, &i64::to_be_bytes(1));

    loop {
        if let Ok(input) = receiver.try_recv() {
            crate::mem::write(mem, SERIAL_IN, &i64::to_be_bytes(input as i64 + 1));
        }

        let out = crate::mem::read(mem, SERIAL_OUT);
        if out != 0 {
            if sender
                .send(UIMessage::Serial(char::from_u32(out as u32).unwrap()))
                .is_err()
            {
                break;
            }

            crate::mem::write(mem, SERIAL_OUT, &i64::to_be_bytes(0));
        }

        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}
