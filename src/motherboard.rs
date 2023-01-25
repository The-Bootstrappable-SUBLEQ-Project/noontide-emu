use std::sync::{Arc, Barrier};

pub fn motherboard_loop(io_barrier: Arc<Barrier>, cpu_barrier: Arc<Barrier>) {
    loop {
        io_barrier.wait();
        io_barrier.wait();
        cpu_barrier.wait();
        cpu_barrier.wait();
        std::thread::sleep(std::time::Duration::from_micros(20));
    }
}
