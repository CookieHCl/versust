use crate::{Job, NoSuccessfulJobError};
use std::sync::mpsc;
use std::thread;

/// Waits for the fastest job to complete and returns its index and result.  
/// Remaining jobs continue running in the background.
///
/// This is a helper macro for [`rush`](rush()) function.
///
/// The macro accepts closure's body, with an optional preprocessing section.  
/// Preprocessing sections can be used to set up variables that will be moved into the closure.
///
/// # Examples
///
/// Using macro without preprocessing section:
///
/// ```
/// use versust::rush;
/// use std::thread;
/// use std::time::Duration;
///
/// let result = rush![
///     {
///         thread::sleep(Duration::from_millis(100));
///         1
///     },
///     {
///         thread::sleep(Duration::from_millis(50));
///         2
///     }
/// ];
///
/// assert_eq!(result.unwrap(), (1, 2));
/// ```
///
/// Using macro with preprocessing section:
///
/// ```
/// use versust::rush;
/// use std::sync::Arc;
/// use std::sync::atomic::{AtomicU8, Ordering};
/// use std::thread;
/// use std::time::Duration;
///
/// let finished_count = Arc::new(AtomicU8::new(0));
///
/// let result = rush![
///     [let finished_count = finished_count.clone();]
///     {
///         thread::sleep(Duration::from_millis(100));
///         finished_count.fetch_add(1, Ordering::AcqRel);
///         "1st job"
///     },
///     [let finished_count = finished_count.clone();]
///     {
///         thread::sleep(Duration::from_millis(50));
///         finished_count.fetch_add(1, Ordering::AcqRel);
///         "2nd job"
///     }
/// ];
///
/// assert_eq!(result.unwrap(), (1, "2nd job"));
/// assert_eq!(finished_count.load(Ordering::Acquire), 1);
///
/// // 1st job is still running in the background
/// thread::sleep(Duration::from_millis(150));
/// assert_eq!(finished_count.load(Ordering::Acquire), 2);
/// ```
#[macro_export]
macro_rules! rush {
    ( $( $( [ $($preprocessing:tt)+ ] )? { $($body:tt)* } ),+ $(,)? ) => {{
        $crate::rush([
            $({
                $($($preprocessing)+)?
                $crate::into_job(move || { $($body)* })
            }),+
        ])
    }};
}

/// Waits for the fastest job to complete and returns its index and result.  
/// Remaining jobs continue running in the background.
///
/// Normally you would use the [`rush!`](crate::rush!) macro instead of using this function directly.
///
/// Note that when the main thread of a Rust program terminates, the entire program shuts down, terminating all running threads.
///
/// # Errors
///
/// Returns [`NoSuccessfulJobError`] if:
///
/// - No jobs are provided.
/// - All jobs panicked.
///
/// # Examples
///
/// ```
/// use versust::rush;
/// use std::thread;
/// use std::time::Duration;
///
/// let result = rush([
///     versust::into_job(|| {
///         thread::sleep(Duration::from_millis(100));
///         1
///     }),
///     versust::into_job(|| {
///         thread::sleep(Duration::from_millis(50));
///         2
///     })
/// ]);
///
/// assert_eq!(result.unwrap(), (1, 2));
/// ```
pub fn rush<I, T: Send + 'static>(jobs: I) -> Result<(usize, T), NoSuccessfulJobError>
where
    I: IntoIterator<Item = Job<T>>,
    T: Send + 'static,
{
    let (tx, rx) = mpsc::channel::<(usize, T)>();

    for (i, job) in jobs.into_iter().enumerate() {
        let tx = tx.clone();
        thread::spawn(move || {
            let out = job();
            let _ = tx.send((i, out)); // can return Err when rush is finished
        });
    }

    // drop tx to prevent the receiver from hanging when all jobs fail.
    drop(tx);

    // return on first exit, other threads will be continued
    rx.recv().map_err(|_| NoSuccessfulJobError)
}

#[cfg(test)]
mod tests {
    use crate::into_job;

    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU8, Ordering};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn rush_runs_correctly() {
        /* function */
        let function_result = rush(vec![
            Box::new(|| {
                thread::sleep(Duration::from_millis(30));
                "1st job"
            }) as Job<&str>,
            into_job(|| {
                thread::sleep(Duration::from_millis(10));
                "2nd job"
            }),
        ]);
        assert_eq!(
            function_result.unwrap(),
            (1, "2nd job"),
            "rush function did not execute jobs correctly"
        );

        /* macro */
        let macro_result = rush![
            {
                thread::sleep(Duration::from_millis(30));
                1i64
            },
            {
                thread::sleep(Duration::from_millis(10));
                2
            },
            {
                thread::sleep(Duration::from_millis(20));
                3
            }
        ];
        assert_eq!(
            macro_result.unwrap(),
            (1, 2i64),
            "rush macro did not execute jobs correctly"
        );
    }

    #[test]
    fn rush_runs_remaining_jobs() {
        let finished_count = Arc::new(AtomicU8::new(0));

        let result = rush![
            [let finished_count = finished_count.clone();]
            {
                thread::sleep(Duration::from_millis(100));
                finished_count.fetch_add(1, Ordering::AcqRel);
            },
            [let finished_count = finished_count.clone();]
            {
                thread::sleep(Duration::from_millis(150));
                finished_count.fetch_add(1, Ordering::AcqRel);
            },
            [let finished_count = finished_count.clone();]
            {
                thread::sleep(Duration::from_millis(50));
                finished_count.fetch_add(1, Ordering::AcqRel);
            },
        ];

        assert_eq!(
            finished_count.load(Ordering::Acquire),
            1,
            "rush didn't exit after the fastest job"
        );
        assert_eq!(
            result.unwrap(),
            (2, ()),
            "rush did not execute jobs correctly"
        );

        // wait for other jobs to finish
        thread::sleep(Duration::from_millis(200));

        assert_eq!(
            finished_count.load(Ordering::Acquire),
            3,
            "rush didn't run all remaining jobs"
        );
    }

    #[test]
    fn rush_fails_on_empty_jobs() {
        let result: Result<(usize, ()), _> = rush(vec![]);
        assert_eq!(result, Err(NoSuccessfulJobError));
    }

    #[test]
    fn rush_fails_when_all_jobs_panic() {
        let result = rush![
            {
                panic!("1st job panicked");
            },
            {
                panic!("2nd job panicked");
            },
        ];
        assert_eq!(result, Err(NoSuccessfulJobError));
    }
}
