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
