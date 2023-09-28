use std::{
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
};

type Job = Box<dyn FnOnce() + 'static + Send>;
enum Message {
    Close,
    Do(Job),
}
struct Worker {
    id: usize,
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    pub fn new(id: usize, receiver: Arc<Mutex<Receiver<Message>>>) -> Self {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv().unwrap();

            match message {
                Message::Close => break,
                Message::Do(job) => {
                    job();
                }
            }
        });

        Self {
            id,
            thread: Some(thread),
        }
    }
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    max_workers: usize,
    sender: Sender<Message>,
}

impl ThreadPool {
    pub fn new(max_workers: usize) -> Self {
        let (sender, receiver) = mpsc::channel();

        let mut workers = Vec::with_capacity(max_workers);

        let receiver = Arc::new(Mutex::new(receiver));

        for id in 1..=max_workers {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        Self {
            workers,
            max_workers,
            sender,
        }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + 'static + Send,
    {
        self.sender.send(Message::Do(Box::new(f))).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for _ in 0..self.max_workers {
            self.sender.send(Message::Close).unwrap();
        }

        self.workers.iter_mut().for_each(|worker| {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        let p = ThreadPoo::new(4);
        p.execute(|| println!("do new job1"));
        p.execute(|| println!("do new job2"));
        p.execute(|| println!("do new job3"));
        p.execute(|| println!("do new job4"));
    }
}