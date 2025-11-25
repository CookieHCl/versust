use crate::{Job, JobResult};
use std::thread;

/// Waits for all jobs to complete and returns their [`thread::Result`](std::thread::Result)s.
///
/// This is a helper macro for [`sync`](sync()) function.
///
/// The macro accepts closure's body, with an optional preprocessing section.  
/// Preprocessing sections can be used to set up variables that will be moved into the closure.
///
/// # Examples
///
/// Using macro without preprocessing section:
///
/// ```
/// use versust::sync;
/// use std::thread;
/// use std::time::Duration;
///
/// let results = sync![
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
/// let results: Vec<_> = results.into_iter().map(|r| r.unwrap()).collect();
/// assert_eq!(results, vec![1, 2]);
/// ```
///
/// Using macro with preprocessing section:
///
/// ```
/// use versust::sync;
/// use std::sync::Arc;
/// use std::sync::atomic::{AtomicU8, Ordering};
/// use std::thread;
/// use std::time::Duration;
///
/// let finished_count = Arc::new(AtomicU8::new(0));
///
/// sync![
///     [let finished_count = finished_count.clone();]
///     {
///         thread::sleep(Duration::from_millis(100));
///         finished_count.fetch_add(1, Ordering::AcqRel);
///     },
///     [let finished_count = finished_count.clone();]
///     {
///         thread::sleep(Duration::from_millis(50));
///         finished_count.fetch_add(1, Ordering::AcqRel);
///     }
/// ];
///
/// assert_eq!(finished_count.load(Ordering::Acquire), 2);
/// ```
#[macro_export]
macro_rules! sync {
    ( $( $( [ $($preprocessing:tt)+ ] )? { $($body:tt)* } ),+ $(,)? ) => {{
        $crate::sync([
            $({
                $($($preprocessing)+)?
                $crate::into_job(move || { $($body)* })
            }),+
        ])
    }};
}

/// Waits for all jobs to complete and returns their [`thread::Result`](std::thread::Result)s.
///
/// Normally you would use the [`sync!`](crate::sync!) macro instead of using this function directly.
///
/// This function blocks the current thread until all jobs have completed.  
/// It then collects and returns their results in a vector.
///
/// # Examples
///
/// ```
/// use versust::sync;
/// use std::thread;
/// use std::time::Duration;
///
/// let results = sync([
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
/// let results: Vec<_> = results.into_iter().map(|h| h.unwrap()).collect();
/// assert_eq!(results, vec![1, 2]);
/// ```
pub fn sync<I, T>(jobs: I) -> Vec<JobResult<T>>
where
    I: IntoIterator<Item = Job<T>>,
    T: Send + 'static,
{
    let handles: Vec<_> = jobs.into_iter().map(|job| thread::spawn(job)).collect();

    handles
        .into_iter()
        .map(|h| h.join())
        .collect::<Vec<JobResult<T>>>()
}

#[cfg(test)]
mod tests {
    use crate::into_job;

    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn sync_runs_correctly() {
        /* function */
        let function_results = sync(vec![
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
            function_results.len(),
            2,
            "sync function returned incorrect number of results"
        );

        // check results
        let function_results: Vec<&str> =
            function_results.into_iter().map(|h| h.unwrap()).collect();
        assert_eq!(
            function_results,
            vec!["1st job", "2nd job"],
            "sync function did not execute jobs correctly"
        );

        /* macro */
        let macro_results = sync![
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
            macro_results.len(),
            3,
            "sync macro returned incorrect number of results"
        );

        // check results
        let macro_results: Vec<i64> = macro_results.into_iter().map(|h| h.unwrap()).collect();
        assert_eq!(
            macro_results,
            vec![1i64, 2i64, 3i64],
            "sync macro did not execute jobs correctly"
        );
    }
}
