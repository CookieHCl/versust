use crate::Job;
use std::sync::mpsc;
use std::thread;

// rush![ { ... }, { ... }, { ... }, ]
#[macro_export]
macro_rules! rush {
    ( $( { $($body:tt)* } ),+ $(,)? ) => {{
        $crate::rush([
            $(
                Box::new(|| { $($body)* }) as $crate::Job<_>
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
