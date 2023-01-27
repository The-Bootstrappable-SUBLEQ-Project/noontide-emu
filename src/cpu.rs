use std::sync::{mpsc::Sender, Arc, Barrier};

use bus::BusReader;

use crate::msg::UIMessage;

const CPU_CONTROL_START: usize = 0x13EE0000;

pub fn cpu_loop(
    mem: &mut [u8],
    cpu_id: usize,
    cpu_barrier: Arc<Barrier>,
    ui_sender: Sender<UIMessage>,
    mut term_rx: BusReader<usize>,
) {
    let cpu_control_status = CPU_CONTROL_START + 16 * cpu_id;
    let cpu_control_eip = cpu_control_status + 8;

    crate::mem::write(mem, cpu_control_status, &u64::to_be_bytes(1));
    let mut eip: u64 = 0;
    loop {
        // CPU cycle start
        cpu_barrier.wait();

        if let Ok(_val) = term_rx.try_recv() {
            cpu_barrier.wait();
            // eprintln!("CPU {} exited", cpu_id);
            return;
        }

        // CPU is not running
        if crate::mem::read(mem, cpu_control_status) != 1 {
            if crate::mem::read(mem, cpu_control_status) == 2 {
                crate::mem::write(mem, cpu_control_status, &u64::to_be_bytes(4));
                ui_sender.send(UIMessage::CPUStopped(cpu_id)).unwrap();
            }

            // CPU cycle end
            cpu_barrier.wait();

            while crate::mem::read(mem, cpu_control_status) != 1 {
                cpu_barrier.wait();

                if let Ok(_val) = term_rx.try_recv() {
                    cpu_barrier.wait();
                    // eprintln!("CPU {} exited", cpu_id);
                    return;
                }

                cpu_barrier.wait();
            }

            eip = crate::mem::read(mem, cpu_control_eip) as u64;
            ui_sender.send(UIMessage::CPUStarted(cpu_id)).unwrap();

            // CPU cycle start
            cpu_barrier.wait();
        }

        //  512: 16.3575 +- 0.0710 seconds time elapsed  ( +-  0.43% )
        // 1024: 16.3151 +- 0.0483 seconds time elapsed  ( +-  0.30% )
        // 2048: 16.4346 +- 0.0623 seconds time elapsed  ( +-  0.38% )
        // 4096: 16.5468 +- 0.0562 seconds time elapsed  ( +-  0.34% )
        for _i in 0..1024 {
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
                ui_sender
                    .send(UIMessage::Debug(format!(
                        "{eip:#X} {a_addr:#X}({a_val:#X}) {b_addr:#X}({b_val:#X}) {c_addr:#X}\r\n"
                    )))
                    .unwrap();
            }

            a_val = a_val.wrapping_sub(b_val);
            crate::mem::write(mem, a_addr as usize, &i64::to_be_bytes(a_val));
            if a_val <= 0 {
                eip = c_addr as u64;
            } else {
                eip += 24;
            }
        }

        crate::mem::write(mem, cpu_control_eip, &u64::to_be_bytes(eip));
        ui_sender.send(UIMessage::SetEIP(eip)).unwrap();

        // CPU cycle end
        cpu_barrier.wait();
    }
}
