use crate::{Job, JobResult};
use std::thread;

// sync![ { ... }, { ... }, [ ... ]{ ... }, ]
#[macro_export]
macro_rules! sync {
    ( $( $( [ $($preprocessing:tt)+ ] )? { $($body:tt)* } ),+ $(,)? ) => {{
        $crate::sync([
            $({
                $($($preprocessing)+)?
                $crate::make_job(move || { $($body)* })
            }),+
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

#[cfg(test)]
mod tests {
    use crate::make_job;

    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn sync_runs_correctly() {
        /* function */
        let function_results = sync(vec![
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
            function_results.len(),
            2,
            "sync function returned incorrect number of results"
        );

        // check results
        let function_results: Vec<&str> =
            function_results.into_iter().map(|h| h.unwrap()).collect();
        assert_eq!(
            function_results,
            vec!["1st job", "2nd job"],
            "sync function did not execute jobs correctly"
        );

        /* macro */
        let macro_results: Vec<JobResult<i64>> = sync![
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
            macro_results.len(),
            3,
            "sync macro returned incorrect number of results"
        );

        // check results
        let macro_results: Vec<i64> = macro_results.into_iter().map(|h| h.unwrap()).collect();
        assert_eq!(
            macro_results,
            vec![1i64, 2i64, 3i64],
            "sync macro did not execute jobs correctly"
        );
    }
}
