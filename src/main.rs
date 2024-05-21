use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Sender, Receiver};
use std::thread;
use std::time::Duration;
use rand::Rng;

#[derive(Debug)]
struct Task {
    id: u32,
    progress: Arc<Mutex<u8>>,
}

impl Task {
    fn new(id: u32) -> Self {
        Task {
            id,
            progress: Arc::new(Mutex::new(0)),
        }
    }

    fn start(&self) {
        let progress = Arc::clone(&self.progress);
        let task_id = self.id;
        thread::spawn(move || {
            let mut rng = rand::thread_rng();
            for i in 0..=100 {
                thread::sleep(Duration::from_millis(rng.gen_range(10..100)));
                let mut prog = progress.lock().unwrap();
                *prog = i;
            }
            println!("Task {} finished.", task_id);
        });
    }
}

enum SchedulerMessage {
    NewTask(u32),
    Stop,
}

fn main() {
    // Channel for communicating with the scheduler thread
    let (tx, rx): (Sender<SchedulerMessage>, Receiver<SchedulerMessage>) = mpsc::channel();
    let (progress_tx, progress_rx): (Sender<(u32, Arc<Mutex<u8>>)>, Receiver<(u32, Arc<Mutex<u8>>)>) = mpsc::channel();

    // Scheduler thread
    let scheduler_tx = tx.clone();
    let scheduler_tx_clone = scheduler_tx.clone();
    let scheduler_progress_tx = progress_tx.clone();
    let scheduler_handle = thread::spawn(move || {
        let mut task_id_counter = 0;
        let mut threads = vec![];

        loop {
            match rx.recv() {
                Ok(SchedulerMessage::NewTask(task_id)) => {
                    if threads.len() < 10 {
                        let task = Task::new(task_id);
                        scheduler_progress_tx.send((task_id, Arc::clone(&task.progress))).unwrap();
                        task.start();
                        threads.push(task_id);
                        task_id_counter += 1;
                    } else {
                        println!("Maximum number of threads reached. Task {} will be delayed.", task_id);
                        // Add a delay to simulate distribution to other threads
                        thread::sleep(Duration::from_secs(1));
                        scheduler_tx.send(SchedulerMessage::NewTask(task_id)).unwrap();
                    }
                },
                Ok(SchedulerMessage::Stop) => {
                    println!("Scheduler stopping...");
                    break;
                },
                Err(_) => break,
            }
        }

        // Wait for all threads to finish
        for thread_id in threads {
            println!("Waiting for task {} to finish...", thread_id);
            scheduler_tx.send(SchedulerMessage::Stop).unwrap();
        }
    });

    // Wait for the scheduler thread to start
    thread::sleep(Duration::from_secs(2));

    // Create 200 tasks
    for i in 1..=200 {
        scheduler_tx_clone.send(SchedulerMessage::NewTask(i)).unwrap();
    }

    // Main thread polling the task's progress
    loop {
        thread::sleep(Duration::from_secs(1));
        if let Ok((task_id, progress)) = progress_rx.try_recv() {
            let progress = *progress.lock().unwrap();
            println!("Task {} progress: {}%", task_id, progress);
            if progress >= 100 {
                println!("Task {} finished.", task_id);
            }
        }

        // Check if all tasks finished
        if progress_rx.try_recv().is_err() {
            break;
        }
    }

    // Send a stop message to the scheduler
    scheduler_tx_clone.send(SchedulerMessage::Stop).unwrap();

    // Wait for the scheduler thread to finish
    scheduler_handle.join().unwrap();
}
