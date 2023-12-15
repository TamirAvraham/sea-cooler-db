use std::{
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex, Once, RwLock,
    },
    thread::{self, JoinHandle},
};

use crate::helpers::get_cpu_cores;

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
            // println!("thread {} is empty",id);
            let message = receiver.lock().expect("cant get receiver").recv().expect("cant recv task at thread pool");
            // println!("thread {} got a message",id);
            match message {
                Message::Close => break,
                Message::Do(job) => {
                    job();
                    // println!("thread {} completed job",id);
                }
            }
        });

        Self {
            id,
            thread: Some(thread),
        }
    }
}
pub struct ComputedValue<T>{
    receiver:Receiver<T>
}
impl<T> ComputedValue<T> {
  pub fn new(receiver:Receiver<T>)->Self{
    Self { receiver }
  }  
  pub fn get(self)->T{
    self.receiver.recv().unwrap()
  }
}
pub struct ThreadPool {
    workers: Vec<Worker>,
    max_workers: usize,
    sender: Sender<Message>,
}

impl ThreadPool {
    fn new(max_workers: usize) -> Self {
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

    pub fn compute<F, R, Args>(&self, f: F, args: Args) -> ComputedValue<R>
    where
        F: FnOnce(Args) -> R + Send + 'static,
        R: 'static + Send,
        Args: Send + 'static,
    {
        let (result_sender, result_receiver) = mpsc::channel();

        let job = Message::Do(Box::new(move || {
            result_sender.send(f(args)).expect("Result sender failed");
        }));

        self.sender
            .send(job)
            .expect("Thread pool sender failed to send job");

        ComputedValue::new(result_receiver)
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


pub static mut SINGLETON:Option<Arc<ThreadPool>>=None;
static INIT:Once=Once::new();


impl ThreadPool {
    pub fn get_instance()-> Arc<Self>{
        INIT.call_once(||
            unsafe {
                SINGLETON=Some(Arc::new(Self::new(get_cpu_cores())));
            }
        );

        unsafe { Arc::clone(SINGLETON.as_ref().unwrap()) }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        let p = ThreadPool::new(4);
        p.execute(|| println!("do new job1"));
        p.execute(|| println!("do new job2"));
        p.execute(|| println!("do new job3"));
        p.execute(|| println!("do new job4"));

        p.execute(|| (0..1_000).for_each(|i| println!("{}",i)));
        p.execute(|| (0..1_000).for_each(|i| println!("{}",i)));
        p.execute(|| (0..1_000).for_each(|i| println!("{}",i)));
        p.execute(|| (0..1_000).for_each(|i| println!("{}",i)));

    }
    #[test]
    fn compute_test() {
        let thread_pool = ThreadPool::new(4);
        let num = 4;
        let result1 = thread_pool.compute(|num| num * 30303, num);
        assert!(result1.get() == num * 30303);
        let f = |mut x : i32| {(0..10).for_each(|i| x+=x*i); x};
        let res1=thread_pool.compute(f, num);
        let res2=thread_pool.compute(f, num);
        assert!(res1.get()==res2.get())
    }


    #[test]
    fn test_singleton() {
        let t=ThreadPool::get_instance();
        t.execute(|| println!("hello world"))
    }
}
