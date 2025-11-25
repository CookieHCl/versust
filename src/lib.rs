use std::fmt;
use std::sync::atomic::AtomicBool;
use std::thread;

/// An error returned from the [`race`](race()) function or the [`rush`](rush()) function.
///
/// This error indicates that no job completed successfully.  
/// Either no jobs were provided to the function or all jobs panicked/failed to send results.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct NoSuccessfulJobError;
impl fmt::Display for NoSuccessfulJobError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "No job completed successfully; Either no jobs were provided or all jobs failed"
        )
    }
}
impl std::error::Error for NoSuccessfulJobError {}

/// A closure that can be executed in a separate thread.
///
/// Normally you would use the [`into_job`] function to create a [`Job<T>`].  
/// It is used by [`sync`](sync()), [`branch`](branch()), and [`rush`](rush()) functions.
pub type Job<T> = Box<dyn FnOnce() -> T + Send + 'static>;

/// The result of a job execution.
///
/// It is a type alias for [`std::thread::Result<T>`].
pub type JobResult<T> = thread::Result<T>;

/// A closure that can be executed in a [`race`](race()) function.
///
/// Normally you would use the [`into_race_job`] function to create a [`RaceJob<T>`].
///
/// To ensure that [`race`](race()) function operates correctly, [`RaceJob`] must satisfy the following requirements:
///
/// - Each job should periodically check the provided [`AtomicBool`] flag to see if another job has already finished.
/// - If flag is set by another job, the job should return [`None`] immediately.
/// - If flag remains unset after the job completes, it should set the flag and return [`Some(result)`](Some).
///   Using [`AtomicBool::compare_exchange`] is highly recommended.
/// - Only the fastest job should return [`Some(result)`](Some); all other jobs must return [`None`].
pub type RaceJob<T> = Box<dyn FnOnce(&AtomicBool) -> Option<T> + Send + 'static>;

/// Creates a [`Job<T>`] from a closure.
///
/// Helper function for [`sync`](sync()), [`branch`](branch()), and [`rush`](rush()) functions.  
/// This function performs coercion to assist with type inference.
///
/// If you really want to use [`Box`] directly, you can either:
///
/// - Cast the Box to [`Job<T>`] explicitly, e.g. `Box::new(|| 1) as Job<i32>`
/// - Declare variable to trigger coercion, e.g. `let job: Job<i32> = Box::new(|| 1);`
pub fn into_job<T, F>(f: F) -> Job<T>
where
    F: FnOnce() -> T + Send + 'static,
{
    Box::new(f)
}

/// Creates a [`RaceJob<T>`] from a closure.
///
/// Helper function for [`race`](race()) function.  
/// This function performs coercion to assist with type inference.
///
/// If you really want to use [`Box`] directly, you can either:
///
/// - Cast the Box to [`RaceJob<T>`] explicitly, e.g. `Box::new(|_: &AtomicBool| Some(1)) as RaceJob<i32>`
/// - Declare variable to trigger coercion, e.g. `let job: RaceJob<i32> = Box::new(|_: &AtomicBool| Some(1));`
pub fn into_race_job<T, F>(f: F) -> RaceJob<T>
where
    F: FnOnce(&AtomicBool) -> Option<T> + Send + 'static,
{
    Box::new(f)
}

mod concurrency;
pub use concurrency::{branch, race, rush, sync};

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_concurrency() {
        let results = sync![
            {
                race![
                    {
                        thread::sleep(Duration::from_millis(100));
                        "race_slow"
                    },
                    {
                        thread::sleep(Duration::from_millis(10));
                        "race_fast"
                    }
                ]
            },
            {
                rush![
                    {
                        thread::sleep(Duration::from_millis(10));
                        "rush_fast"
                    },
                    {
                        thread::sleep(Duration::from_millis(100));
                        "rush_slow"
                    }
                ]
            }
        ];

        let race_result = results[0].as_ref().unwrap().as_ref().unwrap();
        let rush_result = results[1].as_ref().unwrap().as_ref().unwrap();

        assert_eq!(
            *race_result,
            (1, "race_fast"),
            "race did not return correct result"
        );
        assert_eq!(
            *rush_result,
            (0, "rush_fast"),
            "rush did not return correct result"
        );
    }
}
