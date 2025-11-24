use crate::Job;
use std::sync::mpsc;
use std::thread;

// rush![ { ... }, { ... }, { ... }, ]
#[macro_export]
macro_rules! rush {
    ( $( { $($body:tt)* } ),+ $(,)? ) => {{
        $crate::rush([
            $(
                $crate::make_job(|| { $($body)* })
            ),+
        ])
    }};
}

pub fn rush<I, T: Send + 'static>(jobs: I) -> (usize, T)
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

    // Return on first exit, other threads will be continued
    rx.recv().unwrap()
}

#[cfg(test)]
mod tests {
    use crate::make_job;

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
            make_job(|| {
                thread::sleep(Duration::from_millis(10));
                "2nd job"
            }),
        ]);
        assert_eq!(
            function_result,
            (1, "2nd job"),
            "rush function did not execute jobs correctly"
        );

        /* macro */
        let macro_result: (_, i64) = rush![
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
            "rush macro did not execute jobs correctly"
        );
    }

    #[test]
    fn rush_runs_remaining_jobs() {
        let finished_count = Arc::new(AtomicU8::new(0));

        let result = rush([
            make_job({
                let finished_count = finished_count.clone();
                move || {
                    thread::sleep(Duration::from_millis(100));
                    finished_count.fetch_add(1, Ordering::AcqRel);
                }
            }),
            make_job({
                let finished_count = finished_count.clone();
                move || {
                    thread::sleep(Duration::from_millis(150));
                    finished_count.fetch_add(1, Ordering::AcqRel);
                }
            }),
            make_job({
                let finished_count = finished_count.clone();
                move || {
                    thread::sleep(Duration::from_millis(50));
                    finished_count.fetch_add(1, Ordering::AcqRel);
                }
            }),
        ]);

        assert_eq!(
            finished_count.load(Ordering::Acquire),
            1,
            "rush didn't exit after the fastest job"
        );
        assert_eq!(result, (2, ()), "rush did not execute jobs correctly");

        // wait for other jobs to finish
        thread::sleep(Duration::from_millis(200));

        assert_eq!(
            finished_count.load(Ordering::Acquire),
            3,
            "rush didn't run all remaining jobs"
        );
    }
}
