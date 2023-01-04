use std::sync::mpsc::Sender;

use crate::msg::UIMessage;

pub fn cpu_loop(mem: &mut [u8], sender: Sender<UIMessage>) {
    let mut eip: i64 = 0;
    loop {
        let a_addr = crate::mem::read(mem, eip as usize);
        let b_addr = crate::mem::read(mem, (eip + 8) as usize);
        let c_addr = crate::mem::read(mem, (eip + 16) as usize);

        let mut a_val = crate::mem::read(mem, a_addr as usize);
        let b_val = crate::mem::read(mem, b_addr as usize);

        if sender
            .send(UIMessage::Debug(format!(
                "{eip:#X} {a_addr:#X}({a_val:#X}) {b_addr:#X}({b_val:#X}) {c_addr:#X}\r\n"
            )))
            .is_err()
        {
            break;
        }

        a_val -= b_val;
        crate::mem::write(mem, a_addr as usize, &i64::to_be_bytes(a_val));
        if a_val <= 0 {
            eip = c_addr;
        } else {
            eip += 24;
        }

        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}
