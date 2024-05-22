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
            progress_tx.send(SchedulerMessage::TaskFinished(task_id)).unwrap();
        });
    }
}

enum SchedulerMessage {
    NewTask(Sender<u32>),
    TaskFinished(u32),
    GetProgress(u32, Sender<i8>),
    ProgressUpdate(u32, u8),
    Thanks,
}

fn start_task(
    id: u32,
    tasks_ids: &mut Vec<u32>,
    scheduler_tx: &Sender<SchedulerMessage>,
) {
    tasks_ids.push(id);
    let task = Task::new(id, scheduler_tx.clone());
    task.start();
}

fn main() {
    let (scheduler_tx, scheduler_rx): (Sender<SchedulerMessage>, Receiver<SchedulerMessage>) = mpsc::channel();
    let scheduler_tx_clone = scheduler_tx.clone();

    let scheduler = thread::spawn(move || {
        let mut actual_id = 0;
        let mut waiting_tasks: Vec<u32> = Vec::new();
        let mut tasks_ids: Vec<u32> = Vec::new();
        let mut progress_map: std::collections::HashMap<u32, u8> = std::collections::HashMap::new();

        loop {
            match scheduler_rx.recv() {
                Ok(SchedulerMessage::NewTask(response_tx)) => {
                    if tasks_ids.len() < 10 {
                        start_task(actual_id, &mut tasks_ids, &scheduler_tx);
                        response_tx.send(actual_id).unwrap();
                    } else {
                        println!("--- Maximum number of threads reached. Task {} will be delayed.", actual_id);
                        waiting_tasks.push(actual_id);
                        response_tx.send(actual_id).unwrap();
                    }
                    actual_id += 1;
                }
                Ok(SchedulerMessage::TaskFinished(id)) => {
                    tasks_ids.retain(|&x| x != id);
                    println!("--- Task {} finished.", id);
                    if !waiting_tasks.is_empty() && tasks_ids.len() < 10 {
                        let waiting_id = waiting_tasks.remove(0);
                        start_task(waiting_id, &mut tasks_ids, &scheduler_tx);
                        println!("--- Waiting task {} started.", waiting_id);
                    }
                }
                Ok(SchedulerMessage::GetProgress(id, response_tx)) => {
                    if let Some(&progress) = progress_map.get(&id) {
                        if progress == 100 { progress_map.remove(&id); }
                        response_tx.send(progress as i8).unwrap();
                    } else if waiting_tasks.contains(&id) {
                        response_tx.send(-1).unwrap();
                    } else {
                        response_tx.send(-2).unwrap();
                    }
                }
                Ok(SchedulerMessage::ProgressUpdate(id, progress)) => {
                    progress_map.insert(id, progress);
                }
                Ok(SchedulerMessage::Thanks) => {
                    println!("--- All tasks finished.");
                    println!("--- Merci au scheduler !");
                    break;
                }
                Err(_) => {
                    println!("--- Scheduler error.");
                    break;
                }
            }
        }
    });

    let mut ids: Vec<u32> = Vec::new();

    for _ in 1..=15 {
        let (new_task_tx, new_task_rx) = mpsc::channel();
        scheduler_tx_clone.send(SchedulerMessage::NewTask(new_task_tx)).unwrap();
        let new_task_id = new_task_rx.recv().unwrap();
        ids.push(new_task_id);
        println!("New task created with ID: {}", new_task_id);
    }

    println!("{:?}", ids);

    loop {
        thread::sleep(Duration::from_secs(1));

        let mut completed_ids: Vec<u32> = Vec::new();

        for &id in &ids {
            let (progress_tx, progress_rx) = mpsc::channel();
            scheduler_tx_clone.send(SchedulerMessage::GetProgress(id, progress_tx)).unwrap();
            let progress = progress_rx.recv().unwrap();
            if progress >= 0 { print!("Task {} progress: {}% / ", id, progress); }

            if progress == -2 {
                completed_ids.push(id);
            }
        }
        println!();

        ids.retain(|&x| !completed_ids.contains(&x));

        if ids.is_empty() { break; }
    }

    println!("End of code !");

    scheduler_tx_clone.send(SchedulerMessage::Thanks).unwrap();
    scheduler.join().unwrap();
}
