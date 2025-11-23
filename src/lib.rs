use std::sync::atomic::AtomicBool;
use std::thread;

pub type Job<T> = Box<dyn FnOnce() -> T + Send + 'static>;
pub type JobResult<T> = thread::Result<T>;

pub type RaceJob<T> = Box<dyn FnOnce(&AtomicBool) -> Option<T> + Send + 'static>;

mod concurrency;
pub use concurrency::{branch, race, rush, sync};
