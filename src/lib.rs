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

        let race_result = results[0].as_ref().unwrap();
        let rush_result = results[1].as_ref().unwrap();

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
