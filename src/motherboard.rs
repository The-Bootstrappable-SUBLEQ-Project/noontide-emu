use std::sync::{Arc, Barrier};

use bus::BusReader;

pub fn motherboard_loop(
    io_barrier: Arc<Barrier>,
    cpu_barrier: Arc<Barrier>,
    mut term_rx: BusReader<usize>,
) {
    loop {
        io_barrier.wait();
        io_barrier.wait();
        cpu_barrier.wait();
        cpu_barrier.wait();

        if let Ok(_val) = term_rx.try_recv() {
            return;
        }

        std::thread::sleep(std::time::Duration::from_micros(20));
    }
}
