use std::sync::{
    mpsc::{Receiver, Sender},
    Arc, Barrier,
};

pub fn motherboard_loop(
    io_barrier: Arc<Barrier>,
    cpu_barrier: Arc<Barrier>,
    mb1_receiver: Receiver<usize>,
    mb2_sender: Sender<usize>,
) {
    loop {
        io_barrier.wait();
        io_barrier.wait();
        cpu_barrier.wait();
        cpu_barrier.wait();
        std::thread::sleep(std::time::Duration::from_micros(20));

        if let Ok(_val) = mb1_receiver.try_recv() {
            mb2_sender.send(0).unwrap();
            mb1_receiver.recv().unwrap();
            io_barrier.wait();
            io_barrier.wait();
            cpu_barrier.wait();
            cpu_barrier.wait();
            eprintln!("Motherboard exited");
            return;
        }
    }
}
