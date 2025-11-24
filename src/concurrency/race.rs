use crate::RaceJob;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, mpsc};
use std::thread;

// race![ { ... }, { ... }, [ ... ]{ ... }, ]
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

// __race_job![ ...; ...; ... ]
// not intended to be used directly;
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

pub fn race<I, T>(jobs: I) -> (usize, T)
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
                tx.send((i, out)).unwrap(); // shouldn't fail
            }
        });
    }

    // Return on first exit, other threads are already terminated
    rx.recv().unwrap()
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
            function_result,
            (1, "2nd job"),
            "race function did not execute jobs correctly"
        );

        /* macro */
        let macro_result: (_, i64) = race![
            {
                thread::sleep(Duration::from_millis(30));
                1
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
            macro_result,
            (1, 2),
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
        assert_eq!(result, (2, ()), "race did not execute jobs correctly");

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
        assert_eq!(result, (2, ()), "race did not execute jobs correctly");

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
}
