use crate::Job;
use std::thread;

// branch![ { ... }, { ... }, { ... }, ]
#[macro_export]
macro_rules! branch {
    ( $( { $($body:tt)* } ),+ $(,)? ) => {{
        $crate::branch(vec![
            $(
                Box::new(|| { $($body)* })
            ),+
        ])
    }};
}

pub fn branch<T: Send + 'static>(jobs: Vec<Job<T>>) -> Vec<thread::JoinHandle<T>> {
    jobs.into_iter()
        .map(|job| thread::spawn(|| job()))
        .collect()
}
