use std::{cell::UnsafeCell, sync::Arc};

use pancurses::{Input::Character, Window};

pub struct SyncUnsafeCell<T: ?Sized>(pub UnsafeCell<T>);

// Allows accessing the UnsafeCell without the .0
impl<T: ?Sized> core::ops::Deref for SyncUnsafeCell<T> {
    type Target = UnsafeCell<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: ?Sized> core::ops::DerefMut for SyncUnsafeCell<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

unsafe impl<T: ?Sized> Sync for SyncUnsafeCell<T> {}

fn main() {
    let mut mem = vec![0u8; 0x14000000];
    let data = std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
    mem[..data.len()].copy_from_slice(&data);

    let mut handles = vec![];
    let arc = Arc::new(SyncUnsafeCell(UnsafeCell::new(mem)));

    {
        let arc = Arc::clone(&arc);
        handles.push(
            std::thread::Builder::new()
                .name("Serial".to_string())
                .spawn(move || serial_loop(arc))
                .unwrap(),
        );
    }

    {
        let arc = Arc::clone(&arc);
        handles.push(
            std::thread::Builder::new()
                .name("CPU 0".to_string())
                .spawn(move || cpu_loop(arc))
                .unwrap(),
        );
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

fn read_mem(mem: &[u8], offset: usize) -> i64 {
    i64::from_be_bytes(mem[offset..offset + 8].try_into().unwrap())
}

fn write_mem(mem: &mut [u8], offset: usize, data: &[u8; 8]) {
    mem[offset..offset + 8].clone_from_slice(data);
}

fn cpu_loop(mem: Arc<SyncUnsafeCell<Vec<u8>>>) {
    let mut eip: i64 = 0;
    loop {
        let a_addr = read_mem(unsafe { mem.get().as_mut().unwrap() }, eip as usize);
        let b_addr = read_mem(unsafe { mem.get().as_mut().unwrap() }, (eip + 8) as usize);
        let c_addr = read_mem(unsafe { mem.get().as_mut().unwrap() }, (eip + 16) as usize);

        let mut a_val = read_mem(unsafe { mem.get().as_mut().unwrap() }, a_addr as usize);
        let b_val = read_mem(unsafe { mem.get().as_mut().unwrap() }, b_addr as usize);

        print!("{eip:#X} {a_addr:#X}({a_val:#X}) {b_addr:#X}({b_val:#X}) {c_addr:#X}\r\n");

        a_val -= b_val;
        write_mem(
            unsafe { mem.get().as_mut().unwrap() },
            a_addr as usize,
            &i64::to_be_bytes(a_val),
        );
        if a_val <= 0 {
            eip = c_addr;
        } else {
            eip += 24;
        }

        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

fn kbhit(window: &Window) -> bool {
    let ch = window.getch();

    if let Some(ch) = ch {
        window.ungetch(&ch);
        true
    } else {
        false
    }
}

const SERIAL_CONNECTED: usize = 0x13ED27E0;
const SERIAL_IN: usize = 0x13ED27E8;
const SERIAL_OUT: usize = 0x13ED27F0;
fn serial_loop(mem: Arc<SyncUnsafeCell<Vec<u8>>>) {
    // https://stackoverflow.com/a/27335584
    let window = pancurses::initscr();
    window.nodelay(true);

    write_mem(
        unsafe { mem.get().as_mut().unwrap() },
        SERIAL_CONNECTED,
        &i64::to_be_bytes(1),
    );

    loop {
        if read_mem(unsafe { mem.get().as_mut().unwrap() }, SERIAL_IN) == 0 && kbhit(&window) {
            let input = window.getch().unwrap();
            if let Character(input) = input {
                write_mem(
                    unsafe { mem.get().as_mut().unwrap() },
                    SERIAL_IN,
                    &i64::to_be_bytes(input as i64 + 1),
                );
            }
        }

        let out = read_mem(unsafe { mem.get().as_mut().unwrap() }, SERIAL_OUT);
        if out != 0 {
            window.printw(&(out as char).to_string());
            write_mem(unsafe { mem.get().as_mut().unwrap() }, SERIAL_OUT, &i64::to_be_bytes(0));
        }

        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}
