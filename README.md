# versust

A Rust library for structured concurrency patterns.  
This library spawns threads for each job with various synchronization strategies.

Inspired from the [Verse Programming language](https://dev.epicgames.com/documentation/en-us/fortnite/time-flow-and-concurrency-in-verse).

## Install

```bash
cargo add versust
```

## Usage

We have a macro API and a function API for each function.

Macro API has a simpler syntax, with no need to declare Boxes or closures.  
Roughly `macro![[optional preprocessing section]{closure body}, ...]` will be converted into:

```rust
macro([
    {
        optional preprocessing section;

        // closure can consume variables defined in preprocessing section
        Box::new(move || {
            closure body
        })
    },
    ...
])
```

### `sync!`

```rust
fn sync<I: IntoIterator<Item = Job<T: Send + 'static>>, T>(jobs: I) -> Vec<JobResult<T>>
```

Waits for all jobs to complete and returns their results.

```rust
use versust::sync;
use std::thread;
use std::time::Duration;

let results = sync![
    {
        thread::sleep(Duration::from_millis(100));
        1
    },
    {
        thread::sleep(Duration::from_millis(50));
        2
    }
];

let results: Vec<_> = results.into_iter().map(|r| r.unwrap()).collect();
assert_eq!(results, vec![1, 2]);
```

### `race!`

```rust
fn race<I: IntoIterator<Item = RaceJob<T>>, T: Send + 'static>(jobs: I) -> Result<(usize, T), NoSuccessfulJobError>
```

Waits for the fastest job to complete and returns its index and result. Remaining jobs are terminated.

To implement this feature, `race!` macro will injects a code for each semicolon-separated statement.  
Since a job may terminate after any semicolon, it must guarantee that it does proper cleanup at any point of termination.

```rust
use versust::race;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::thread;
use std::time::Duration;

let finished_count = Arc::new(AtomicU8::new(0));

let result = race![
    [let finished_count = finished_count.clone();]
    {
        thread::sleep(Duration::from_millis(100));
        finished_count.fetch_add(1, Ordering::AcqRel);
        "1st job"
    },
    [let finished_count = finished_count.clone();]
    {
        thread::sleep(Duration::from_millis(50));
        finished_count.fetch_add(1, Ordering::AcqRel);
        "2nd job"
    }
];

assert_eq!(result.unwrap(), (1, "2nd job"));
assert_eq!(finished_count.load(Ordering::Acquire), 1);

// 1st job is terminated when 2nd job is finished, so finished_count should not increase
thread::sleep(Duration::from_millis(150));
assert_eq!(finished_count.load(Ordering::Acquire), 1);
```

### `rush!`

```rust
fn rush<I: IntoIterator<Item = Job<T>>, T: Send + 'static>(jobs: I) -> Result<(usize, T), NoSuccessfulJobError>
```

Waits for the fastest job to complete and returns its index and result. Remaining jobs continue running in the background.

```rust
use versust::rush;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::thread;
use std::time::Duration;

let finished_count = Arc::new(AtomicU8::new(0));

let result = rush![
    [let finished_count = finished_count.clone();]
    {
        thread::sleep(Duration::from_millis(100));
        finished_count.fetch_add(1, Ordering::AcqRel);
        "1st job"
    },
    [let finished_count = finished_count.clone();]
    {
        thread::sleep(Duration::from_millis(50));
        finished_count.fetch_add(1, Ordering::AcqRel);
        "2nd job"
    }
];

assert_eq!(result.unwrap(), (1, "2nd job"));
assert_eq!(finished_count.load(Ordering::Acquire), 1);

// 1st job is still running in the background
thread::sleep(Duration::from_millis(150));
assert_eq!(finished_count.load(Ordering::Acquire), 2);
```

### `branch!`

```rust
fn branch<I: IntoIterator<Item = Job<T>>, T: Send + 'static>(jobs: I) -> Vec<thread::JoinHandle<T>>
```

Executes jobs concurrently and immediately returns their `JoinHandle`s.

```rust
use versust::branch;
use std::thread;
use std::time::Duration;

let handles = branch![
    {
        thread::sleep(Duration::from_millis(100));
        1
    },
    {
        thread::sleep(Duration::from_millis(50));
        2
    }
];

let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
assert_eq!(results, vec![1, 2]);
```

## License

Licensed under either of:

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
