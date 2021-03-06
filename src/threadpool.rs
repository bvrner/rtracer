// basically the pool implementation in the Rust book
use std::ops::Drop;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

type Job = Box<dyn FnOnce() + Send + 'static>;

enum Message {
    NewJob(Job),
    Finish,
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
}

impl ThreadPool {
    pub fn new(size: usize) -> Self {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for _ in 0..size {
            workers.push(Worker::new(Arc::clone(&receiver)));
        }

        ThreadPool { workers, sender }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Message::NewJob(Box::new(f));

        self.sender.send(job).unwrap();
    }

    pub fn join(&mut self) {
        for _ in self.workers.iter_mut() {
            self.sender.send(Message::Finish).unwrap();
        }

        for worker in self.workers.iter_mut() {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for _ in self.workers.iter_mut() {
            self.sender.send(Message::Finish).unwrap_or(());
        }

        for worker in self.workers.iter_mut() {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

struct Worker {
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Self {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv().unwrap();

            match message {
                Message::NewJob(job) => job(),
                Message::Finish => break,
            }
        });

        Worker {
            thread: Some(thread),
        }
    }
}
