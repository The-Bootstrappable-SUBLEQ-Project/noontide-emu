use std::cell::UnsafeCell;
use std::env;
use std::fs;
use std::sync::Arc;
use std::thread;
use std::time;

fn main() {
    let args: Vec<String> = env::args().collect();

    let file_path = &args[1];
    let mut mem = fs::read(file_path).unwrap();
    mem.resize(0x14000000, 0);

    let arc = Arc::new(UnsafeCell::new(mem));

    {
        let arc = Arc::clone(&arc);
        let handle = thread::Builder::new()
            .name("CPU 0".to_string())
            .spawn(move || cpu_loop(arc))
            .unwrap();
        handle.join().unwrap();
    }
}

fn read_mem(mem: &[u8], offset: usize, len: usize) -> u64 {
    assert!(len <= 8);
    let mut ret: u64 = 0;
    for i in 0..len {
        ret *= 256;
        ret += mem[offset + i] as u64;
    }

    ret
}

fn cpu_loop(mem: Arc<UnsafeCell<Vec<u8>>>) {
    let mut eip = 0;
    loop {
        //let A = read_mem(&mem, eip, 8);
        thread::sleep(time::Duration::from_millis(1000));
        println!("Hello, world!");
    }
}
