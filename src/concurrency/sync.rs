use crate::{Job, JobResult};
use std::thread;

// sync![ { ... }, { ... }, { ... }, ]
#[macro_export]
macro_rules! sync {
    ( $( { $($body:tt)* } ),+ $(,)? ) => {{
        $crate::sync(vec![
            $(
                Box::new(|| { $($body)* })
            ),+
        ])
    }};
}

pub fn sync<T: Send + 'static>(jobs: Vec<Job<T>>) -> Vec<JobResult<T>> {
    let handles: Vec<_> = jobs
        .into_iter()
        .map(|job| thread::spawn(|| job()))
        .collect();

    handles
        .into_iter()
        .map(|h| h.join())
        .collect::<Vec<JobResult<T>>>()
}
