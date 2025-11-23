use crate::RaceJob;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, mpsc};
use std::thread;

// race![ { ... }, { ... }, { ... }, ]
#[macro_export]
macro_rules! race {
    ( $( { $($body:tt)* } ),+ $(,)? ) => {{
        $crate::race([
            $(
                Box::new({
                    use std::sync::atomic::{ AtomicBool, Ordering };

                    move |__is_finished: &AtomicBool| {
                        $crate::__race_job![ __is_finished; $($body)* ]
                    }
                }) as $crate::RaceJob<_>
            ),+
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
