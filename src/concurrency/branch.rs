use crate::Job;
use std::thread;

/// Executes jobs concurrently and immediately returns their [`JoinHandle`](std::thread::JoinHandle)s.
///
/// This is a helper macro for [`branch`](branch()) function.
///
/// The macro accepts closure's body, with an optional preprocessing section.  
/// Preprocessing sections can be used to set up variables that will be moved into the closure.
///
/// # Examples
///
/// Using macro without preprocessing section:
///
/// ```
/// use versust::branch;
/// use std::thread;
/// use std::time::Duration;
///
/// let handles = branch![
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
/// let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
/// assert_eq!(results, vec![1, 2]);
/// ```
///
/// Using macro with preprocessing section:
///
/// ```
/// use versust::branch;
/// use std::sync::mpsc;
/// use std::thread;
/// use std::time::Duration;
///
/// let (tx, rx) = mpsc::channel();
///
/// let handles = branch![
///     [let tx = tx.clone();]
///     {
///         thread::sleep(Duration::from_millis(100));
///         let _ = tx.send("1st job");
///     },
///     [let tx = tx.clone();]
///     {
///         thread::sleep(Duration::from_millis(50));
///         let _ = tx.send("2nd job");
///     }
/// ];
///
/// assert_eq!(rx.recv().unwrap(), "2nd job");
/// assert_eq!(rx.recv().unwrap(), "1st job");
/// ```
#[macro_export]
macro_rules! branch {
    ( $( $( [ $($preprocessing:tt)+ ] )? { $($body:tt)* } ),+ $(,)? ) => {{
        $crate::branch([
            $({
                $($($preprocessing)+)?
                $crate::into_job(move || { $($body)* })
            }),+
        ])
    }};
}

/// Executes jobs concurrently and immediately returns their [`JoinHandle`](std::thread::JoinHandle)s.
///
/// Normally you would use the [`branch!`](crate::branch!) macro instead of using this function directly.
///
/// This allows the caller to continue execution while the jobs run in the background.  
/// The handles can be used to join the threads later.
///
/// Note that when the main thread of a Rust program terminates, the entire program shuts down, terminating all running threads.
///
/// # Examples
///
/// ```
/// use versust::branch;
/// use std::thread;
/// use std::time::Duration;
///
/// let handles = branch([
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
/// let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
/// assert_eq!(results, vec![1, 2]);
/// ```
pub fn branch<I, T>(jobs: I) -> Vec<thread::JoinHandle<T>>
where
    I: IntoIterator<Item = Job<T>>,
    T: Send + 'static,
{
    jobs.into_iter().map(|job| thread::spawn(job)).collect()
}

#[cfg(test)]
mod tests {
    use crate::into_job;

    use super::*;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn branch_runs_correctly() {
        /* function */
        let function_handles = branch(vec![
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
            function_handles.len(),
            2,
            "branch function returned incorrect number of handles"
        );

        // check results
        let mut function_results: Vec<&str> = function_handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect();
        function_results.sort();
        assert_eq!(
            function_results,
            vec!["1st job", "2nd job"],
            "branch function did not execute jobs correctly"
        );

        /* macro */
        let macro_handles = branch![
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
            macro_handles.len(),
            3,
            "branch macro returned incorrect number of handles"
        );

        // check results
        let mut macro_results: Vec<i64> = macro_handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect();
        macro_results.sort();
        assert_eq!(
            macro_results,
            vec![1i64, 2i64, 3i64],
            "branch macro did not execute jobs correctly"
        );
    }

    #[test]
    fn branch_returns_immediately() {
        let (tx, rx) = mpsc::channel();

        let handles = branch![
            [let tx = tx.clone();]
            {
                thread::sleep(Duration::from_millis(200));
                tx.send(()).unwrap();
            },
            [let tx = tx.clone();]
            {
                thread::sleep(Duration::from_millis(300));
                tx.send(()).unwrap();
            },
        ];

        // branch should return almost immediately
        assert_eq!(
            rx.recv_timeout(Duration::from_millis(50)),
            Err(mpsc::RecvTimeoutError::Timeout),
            "branch did not return immediately"
        );

        // clean up handles
        for handle in handles {
            let _ = handle.join();
        }
    }
}
