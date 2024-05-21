use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Sender, Receiver};
use std::thread;
use std::time::Duration;
use rand::Rng;

#[derive(Debug)]
struct Task {
    id: u32,
    progress: Arc<Mutex<u8>>,
    progress_tx: Sender<SchedulerMessage>,
}

impl Task {
    fn new(id: u32, progress_tx: Sender<SchedulerMessage>) -> Self {
        Task {
            id,
            progress: Arc::new(Mutex::new(0)),
            progress_tx,
        }
    }

    fn start(&self) {
        let progress = Arc::clone(&self.progress);
        let task_id = self.id;
        let progress_tx = self.progress_tx.clone();
        thread::spawn(move || {
            let mut rng = rand::thread_rng();
            for i in 0..=100 {
                thread::sleep(Duration::from_millis(rng.gen_range(10..100)));
                let mut prog = progress.lock().unwrap();
                *prog = i;
                progress_tx.send(SchedulerMessage::ProgressUpdate(task_id, i)).unwrap();
            }
            println!("Task {} finished.", task_id);
            progress_tx.send(SchedulerMessage::TaskFinished(task_id)).unwrap();
        });
    }
}

enum SchedulerMessage {
    NewTask(Sender<u32>),
    TaskFinished(u32),
    GetProgress(u32, Sender<u8>),
    ProgressUpdate(u32, u8),
    Stop,
}

fn main() {
    let (scheduler_tx, scheduler_rx): (Sender<SchedulerMessage>, Receiver<SchedulerMessage>) = mpsc::channel();

    let scheduler_tx_cone = scheduler_tx.clone();

    let scheduler = thread::spawn(move || {
        let mut actual_id = 0;
        let mut tasks_ids: Vec<u32> = Vec::new();
        let mut progress_map: std::collections::HashMap<u32, u8> = std::collections::HashMap::new();

        loop {
            match scheduler_rx.recv() {
                Ok(SchedulerMessage::NewTask(response_tx)) => {
                    tasks_ids.push(actual_id);
                    let task = Task::new(actual_id, scheduler_tx.clone());
                    task.start();
                    response_tx.send(actual_id).unwrap();
                    actual_id += 1;
                }
                Ok(SchedulerMessage::TaskFinished(id)) => {
                    tasks_ids.retain(|&x| x != id);
                    println!("--- Task {} finished.", id);
                }
                Ok(SchedulerMessage::GetProgress(id, response_tx)) => {
                    if let Some(&progress) = progress_map.get(&id) {
                        response_tx.send(progress).unwrap();
                    }
                }
                Ok(SchedulerMessage::ProgressUpdate(id, progress)) => {
                    progress_map.insert(id, progress);
                }
                Ok(SchedulerMessage::Stop) => {
                    println!("--- All tasks finished.");
                    break;
                }
                Err(_) => {
                    println!("--- Scheduler error.");
                    break;
                }
            }
        }
    });

    let (new_task_tx, new_task_rx) = mpsc::channel();
    scheduler_tx_cone.send(SchedulerMessage::NewTask(new_task_tx)).unwrap();
    let new_task_id = new_task_rx.recv().unwrap();
    println!("New task created with ID: {}", new_task_id);

    thread::sleep(Duration::from_millis(500));

    let (progress_query_tx, progress_query_rx) = mpsc::channel();
    scheduler_tx_cone.send(SchedulerMessage::GetProgress(new_task_id, progress_query_tx)).unwrap();
    match progress_query_rx.recv() {
        Ok(progress) => println!("Progress of task {}: {}%", new_task_id, progress),
        Err(_) => println!("Failed to get progress of task {}", new_task_id),
    }

    scheduler_tx_cone.send(SchedulerMessage::Stop).unwrap();
    scheduler.join().unwrap();
}
