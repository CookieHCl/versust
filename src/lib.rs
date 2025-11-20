use std::sync::mpsc;
use std::thread;

pub type Job<T> = Box<dyn FnOnce() -> T + Send + 'static>;
pub type JobResult<T> = thread::Result<T>;

pub fn sync<T: Send + 'static>(jobs: Vec<Job<T>>) -> Vec<JobResult<T>> {
    let handles: Vec<_> = jobs
        .into_iter()
        .map(|job| thread::spawn(|| job()))
        .collect();

    handles
        .into_iter()
        .map(|h| h.join())
        .collect::<Vec<JobResult<T>>>()
}

pub fn rush<T: Send + 'static>(jobs: Vec<Job<T>>) -> (usize, T) {
    let (tx, rx) = mpsc::channel::<(usize, T)>();

    for (i, job) in jobs.into_iter().enumerate() {
        let tx = tx.clone();
        thread::spawn(move || {
            let out = job();
            let _ = tx.send((i, out)); // can return Err when rush is finished
        });
    }

    // Return on first exit, other threads will be continued
    rx.recv().expect("at least one job is required for rush()")
}

pub fn branch<T: Send + 'static>(jobs: Vec<Job<T>>) -> Vec<thread::JoinHandle<T>> {
    jobs.into_iter()
        .map(|job| thread::spawn(|| job()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}
