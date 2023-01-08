use std::sync::{mpsc::Sender, Arc, Barrier};

use crate::msg::UIMessage;

pub fn cpu_loop(mem: &mut [u8], cpu_barrier: Arc<Barrier>, ui_sender: Sender<UIMessage>) {
    let mut eip: i64 = 0;
    loop {
        cpu_barrier.wait();
        for _i in 0..1000 {
            if (eip as usize) >= mem.len() {
                if ui_sender.send(UIMessage::SetEIP(eip)).is_err() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(3600000));
                panic!("EIP is outside of the memory region!");
            }

            let a_addr = crate::mem::read(mem, eip as usize);
            let b_addr = crate::mem::read(mem, (eip + 8) as usize);
            let c_addr = crate::mem::read(mem, (eip + 16) as usize);

            let mut a_val = crate::mem::read(mem, a_addr as usize);
            let b_val = crate::mem::read(mem, b_addr as usize);

            #[cfg(feature = "debugger")]
            {
                if ui_sender
                    .send(UIMessage::Debug(format!(
                        "{eip:#X} {a_addr:#X}({a_val:#X}) {b_addr:#X}({b_val:#X}) {c_addr:#X}\r\n"
                    )))
                    .is_err()
                {
                    break;
                }
            }

            a_val = a_val.wrapping_sub(b_val);
            crate::mem::write(mem, a_addr as usize, &i64::to_be_bytes(a_val));
            if a_val <= 0 {
                eip = c_addr;
            } else {
                eip += 24;
            }
        }

        if ui_sender.send(UIMessage::SetEIP(eip)).is_err() {
            break;
        }
        cpu_barrier.wait();
    }
}
