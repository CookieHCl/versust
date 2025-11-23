use crate::{Job, JobResult};
use std::thread;

// sync![ { ... }, { ... }, { ... }, ]
#[macro_export]
macro_rules! sync {
    ( $( { $($body:tt)* } ),+ $(,)? ) => {{
        $crate::sync([
            $(
                Box::new(|| { $($body)* }) as $crate::Job<_>
            ),+
        ])
    }};
}

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
