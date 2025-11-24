use crate::Job;
use std::thread;

// branch![ { ... }, { ... }, { ... }, ]
#[macro_export]
macro_rules! branch {
    ( $( { $($body:tt)* } ),+ $(,)? ) => {{
        $crate::branch([
            $(
                $crate::make_job(|| { $($body)* })
            ),+
        ])
    }};
}

pub fn branch<I, T>(jobs: I) -> Vec<thread::JoinHandle<T>>
where
    I: IntoIterator<Item = Job<T>>,
    T: Send + 'static,
{
    jobs.into_iter().map(|job| thread::spawn(job)).collect()
}

#[cfg(test)]
mod tests {
    use crate::make_job;

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
            make_job(|| {
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
        let macro_handles: Vec<thread::JoinHandle<i64>> = branch![
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

        let handles = branch([
            make_job({
                let tx = tx.clone();
                move || {
                    thread::sleep(Duration::from_millis(200));
                    tx.send(()).unwrap();
                }
            }),
            make_job({
                let tx = tx.clone();
                move || {
                    thread::sleep(Duration::from_millis(300));
                    tx.send(()).unwrap();
                }
            }),
        ]);

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
