use std::{cell::UnsafeCell, sync::Arc};

pub struct SyncUnsafeCell<T: ?Sized>(pub UnsafeCell<T>);

unsafe impl<T: ?Sized> Sync for SyncUnsafeCell<T> {}

fn main() {
    let mut mem = vec![0u8; 0x14000000];
    let data = std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
    mem[..data.len()].copy_from_slice(&data);

    let arc = Arc::new(SyncUnsafeCell(UnsafeCell::new(mem)));

    {
        let arc = Arc::clone(&arc);
        std::thread::spawn(move || cpu_loop(arc)).join().unwrap();
    }
}

fn read_mem(mem: &[u8], offset: usize) -> i64 {
    i64::from_be_bytes(mem[offset..offset + 8].try_into().unwrap())
}

fn cpu_loop(mem: Arc<SyncUnsafeCell<Vec<u8>>>) {
    let mut eip = 0;
    loop {
        let b = read_mem(unsafe { mem.0.get().as_mut().unwrap() }, eip);
        println!("{b:#X}");
        eip += 8;
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
}
