use crate::Job;
use std::sync::mpsc;
use std::thread;

// rush![ { ... }, { ... }, { ... }, ]
#[macro_export]
macro_rules! rush {
    ( $( { $($body:tt)* } ),+ $(,)? ) => {{
        $crate::rush(vec![
            $(
                Box::new(|| { $($body)* })
            ),+
        ])
    }};
}

pub fn rush<T: Send + 'static>(jobs: Vec<Job<T>>) -> (usize, T) {
    assert!(!jobs.is_empty(), "rush() needs at least one job");

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
