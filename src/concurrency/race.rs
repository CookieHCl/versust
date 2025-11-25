use crate::{NoSuccessfulJobError, RaceJob};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, mpsc};
use std::thread;

/// Waits for the fastest job to complete and returns its index and result.  
/// Remaining jobs are terminated.
///
/// This is a helper macro for [`race`](race()) function.
///
/// The macro accepts the body of a closure taking no arguments, with an optional preprocessing section.  
/// Preprocessing sections can be used to set up variables that will be moved into the closure.
///
/// For each semicolon-separated statement in the body, the macro injects a code that checks [`AtomicBool`] flag to see if another job has already finished.
/// Then, the macro sets the flag when the job finishes and returns its result.  
/// Due to the parsing limitations of macros, a semicolon should be added after every statement, even after statements such as function declarations.
///
/// Since a job may terminate after any semicolon, it must guarantee that it does proper cleanup at any point of termination.
///
/// See [`RaceJob<T>`] for more details.
///
/// If you need more fine-grained control, (e.g. checking inside for loop) you should use the [`race`](race()) function.
///
/// # Examples
///
/// Using macro without preprocessing section:
///
/// ```
/// use versust::race;
/// use std::thread;
/// use std::time::Duration;
///
/// let result = race![
///     {
///         thread::sleep(Duration::from_millis(100));
///         "Finished"
///     }
/// ];
///
/// // the closure inside the macro will be expanded into
/// /*
/// |__is_finished: &AtomicBool| {
///     if __is_finished.load(Ordering::Acquire) {
///         return None;
///     }
///     thread::sleep(Duration::from_millis(100));
///
///     if __is_finished.load(Ordering::Acquire) {
///         return None;
///     }
///     let __check_result = "Finished";
///
///     if __is_finished
///         .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
///         .is_ok()
///     {
///         Some(__check_result)
///     } else {
///         None
///     }
/// }
/// */
///
/// assert_eq!(result.unwrap(), (0, "Finished"));
/// ```
///
/// Using macro with preprocessing section:
///
/// ```
/// use versust::race;
/// use std::sync::Arc;
/// use std::sync::atomic::{AtomicU8, Ordering};
/// use std::thread;
/// use std::time::Duration;
///
/// let finished_count = Arc::new(AtomicU8::new(0));
///
/// let result = race![
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
/// // 1st job is terminated when 2nd job is finished, so finished_count should not increase
/// thread::sleep(Duration::from_millis(150));
/// assert_eq!(finished_count.load(Ordering::Acquire), 1);
/// ```
#[macro_export]
macro_rules! race {
    ( $( $( [ $($preprocessing:tt)+ ] )? { $($body:tt)* } ),+ $(,)? ) => {{
        $crate::race([
            $({
                $($($preprocessing)+)?

                use std::sync::atomic::{ AtomicBool, Ordering };
                $crate::into_race_job(
                    move |__is_finished: &AtomicBool| {
                        $crate::__race_job![ __is_finished; $($body)* ]
                    }
                )
            }),+
        ])
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __race_job {
    // recursion base case; if ends without expr, return ()
    ( $flag:ident; ) => {
        if $flag
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            Some(())
        } else {
            None
        }
    };
    // another recursion base case; if ends with expr, return its value
    ( $flag:ident; $last:expr ) => {{
        if $flag.load(Ordering::Acquire) {
            return None;
        }
        let __check_result = $last;

        if $flag
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            Some(__check_result)
        } else {
            None
        }
    }};

    // recursion step 1; handle expression statement
    ( $flag:ident; $head:expr; $($tail:tt)* ) => {{
        if $flag.load(Ordering::Acquire) {
            return None;
        }
        $head;
        $crate::__race_job![ $flag; $($tail)* ]
    }};
    // recursion step 2; handle other statements
    ( $flag:ident; $head:stmt; $($tail:tt)* ) => {{
        if $flag.load(Ordering::Acquire) {
            return None;
        }
        $head
        $crate::__race_job![ $flag; $($tail)* ]
    }};
}

/// Waits for the fastest job to complete and returns its index and result.  
/// Remaining jobs are terminated.
///
/// Normally you would use the [`race!`](crate::race!) macro instead of using this function directly.
///
/// To ensure that this function operates correctly, [`RaceJob`] must satisfy the following requirements:
///
/// - Each job should periodically check the provided [`AtomicBool`] flag to see if another job has already finished.
/// - If flag is set by another job, the job should return [`None`] immediately.
/// - If flag remains unset after the job completes, it should set the flag and return [`Some(result)`](Some).
///   Using [`compare_exchange`](std::sync::atomic::AtomicBool::compare_exchange) is highly recommended.
/// - Only the fastest job should return [`Some(result)`](Some); all other jobs must return [`None`].
///
/// See [`RaceJob<T>`] for more details.
///
/// # Errors
///
/// Returns [`NoSuccessfulJobError`] if:
///
/// - No jobs are provided.
/// - All jobs panicked.
/// - All jobs failed to return a result (i.e. All jobs returned [`None`]).
///
/// # Panics
///
/// This function does not panic.  
/// However, spawned threads may panic if more than one thread returns [`Some(result)`](Some).
///
/// # Examples
///
/// ```
/// use versust::race;
/// use std::sync::Arc;
/// use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
/// use std::thread;
/// use std::time::Duration;
///
/// let counter = Arc::new(AtomicU8::new(0));
///
/// let result = race([
///     versust::into_race_job(|is_finished: &AtomicBool| {
///         thread::sleep(Duration::from_millis(55));
///
///         if is_finished
///             .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
///             .is_ok()
///         {
///             Some("1st job")
///         } else {
///             None
///         }
///     }),
///     versust::into_race_job({
///         let counter = counter.clone();
///
///         move |is_finished: &AtomicBool| {
///             for _ in 0..100 {
///                 thread::sleep(Duration::from_millis(10));
///
///                 if is_finished.load(Ordering::Acquire) {
///                     return None;
///                 }
///                 counter.fetch_add(1, Ordering::AcqRel);
///             }
///
///             if is_finished
///                 .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
///                 .is_ok()
///             {
///                 Some("2nd job")
///             } else {
///                 None
///             }
///         }
///     }),
/// ]);
///
/// assert_eq!(result.unwrap(), (0, "1st job"));
/// assert_eq!(counter.load(Ordering::Acquire), 5);
///
/// // 2nd job is terminated when 1st job is finished, so counter should not increase
/// thread::sleep(Duration::from_millis(100));
/// assert_eq!(counter.load(Ordering::Acquire), 5);
/// ```
pub fn race<I, T>(jobs: I) -> Result<(usize, T), NoSuccessfulJobError>
where
    I: IntoIterator<Item = RaceJob<T>>,
    T: Send + 'static,
{
    let is_finished = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel::<(usize, T)>();

    for (i, job) in jobs.into_iter().enumerate() {
        let is_finished = Arc::clone(&is_finished);
        let tx = tx.clone();

        thread::spawn(move || {
            if let Some(out) = job(&is_finished) {
                // shouldn't fail
                tx.send((i, out))
                    .expect("Only one job can finish successfully");
            }
        });
    }

    // drop tx to prevent the receiver from hanging when all jobs fail.
    drop(tx);

    // return on first exit, other threads are already terminated
    rx.recv().map_err(|_| NoSuccessfulJobError)
}

#[cfg(test)]
mod tests {
    use crate::into_race_job;

    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU8, Ordering};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn race_runs_correctly() {
        /* function */
        let function_result = race(vec![
            Box::new(|is_finished: &AtomicBool| {
                thread::sleep(Duration::from_millis(30));

                if is_finished
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
                {
                    Some("1st job")
                } else {
                    None
                }
            }) as RaceJob<&str>,
            into_race_job(|is_finished| {
                thread::sleep(Duration::from_millis(10));

                if is_finished
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
                {
                    Some("2nd job")
                } else {
                    None
                }
            }),
        ]);
        assert_eq!(
            function_result.unwrap(),
            (1, "2nd job"),
            "race function did not execute jobs correctly"
        );

        /* macro */
        let macro_result = race![
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
            "race macro did not execute jobs correctly"
        );
    }

    #[test]
    fn race_terminates_other_jobs() {
        let finished_count = Arc::new(AtomicU8::new(0));

        let result = race![
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
            "race didn't exit after the fastest job"
        );
        assert_eq!(
            result.unwrap(),
            (2, ()),
            "race did not execute jobs correctly"
        );

        // wait for other jobs to finish
        thread::sleep(Duration::from_millis(200));

        assert_eq!(
            finished_count.load(Ordering::Acquire),
            1,
            "race didn't terminate other jobs"
        );
    }

    #[test]
    fn race_run_jobs_atomically() {
        let finished_count = Arc::new(AtomicU8::new(0));
        let finished_thread = Arc::new(AtomicU8::new(0));

        let result = race![
            [
                let finished_count = finished_count.clone();
                let finished_thread = finished_thread.clone();
            ]
            {
                thread::sleep(Duration::from_millis(100));
                finished_count.fetch_add(1, Ordering::AcqRel);
                finished_thread.store(1, Ordering::Release);
            },
            [
                let finished_count = finished_count.clone();
                let finished_thread = finished_thread.clone();
            ]
            {
                // two statements should run atomically
                {
                    thread::sleep(Duration::from_millis(150));
                    finished_count.fetch_add(1, Ordering::AcqRel);
                };
                finished_thread.store(2, Ordering::Release);
            },
            [
                let finished_count = finished_count.clone();
                let finished_thread = finished_thread.clone();
            ]
            {
                thread::sleep(Duration::from_millis(50));
                finished_count.fetch_add(1, Ordering::AcqRel);
                finished_thread.store(3, Ordering::Release);
            },
        ];

        assert_eq!(
            finished_count.load(Ordering::Acquire),
            1,
            "race didn't exit after the fastest job"
        );
        assert_eq!(
            finished_thread.load(Ordering::Acquire),
            3,
            "race did not execute jobs correctly"
        );
        assert_eq!(
            result.unwrap(),
            (2, ()),
            "race did not execute jobs correctly"
        );

        // wait for other jobs to finish
        thread::sleep(Duration::from_millis(200));

        // since sleep and fetch_add are in one statement, fetch_add should be run
        assert_eq!(
            finished_count.load(Ordering::Acquire),
            2,
            "race didn't run jobs atomically"
        );
        // however, thread is terminated before storing to finished_thread
        assert_eq!(
            finished_thread.load(Ordering::Acquire),
            3,
            "race didn't run jobs atomically"
        )
    }

    #[test]
    fn race_fails_on_empty_jobs() {
        let result: Result<(usize, ()), _> = race(vec![]);
        assert_eq!(result, Err(NoSuccessfulJobError));
    }

    #[test]
    fn race_fails_when_all_jobs_panic() {
        #[allow(unreachable_code)]
        let result = race![
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
