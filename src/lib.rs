use std::sync::atomic::AtomicBool;
use std::thread;

pub type Job<T> = Box<dyn FnOnce() -> T + Send + 'static>;
pub type JobResult<T> = thread::Result<T>;

pub type RaceJob<T> = Box<dyn FnOnce(&AtomicBool) -> Option<T> + Send + 'static>;

pub fn make_job<T, F>(f: F) -> Job<T>
where
    F: FnOnce() -> T + Send + 'static,
{
    Box::new(f)
}
pub fn make_race_job<T, F>(f: F) -> RaceJob<T>
where
    F: FnOnce(&AtomicBool) -> Option<T> + Send + 'static,
{
    Box::new(f)
}

mod concurrency;
pub use concurrency::{branch, race, rush, sync};
