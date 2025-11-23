use crate::Job;
use std::thread;

pub fn branch<T: Send + 'static>(jobs: Vec<Job<T>>) -> Vec<thread::JoinHandle<T>> {
    jobs.into_iter()
        .map(|job| thread::spawn(|| job()))
        .collect()
}
