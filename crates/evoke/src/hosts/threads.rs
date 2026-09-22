//! A few threads over a list, for the batches that call the adapter once per input — `test`, the thief test at
//! `add` — where one call at a time takes a minute. In: the items and the work on one. Out: every result in the
//! items' order, or the first error in that order, after which no more work starts.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};
use std::thread;

/// How many run at once: Jev takes 1 200 requests a minute, and four kept-alive connections send about a dozen a
/// second.
const WORKERS: usize = 4;

pub fn try_each<I: Sync, T: Send, E: Send>(
    items: &[I],
    work: impl Fn(&I) -> Result<T, E> + Sync,
) -> Result<Vec<T>, E> {
    let next = AtomicUsize::new(0);
    let stopped = AtomicBool::new(false);
    let results: Mutex<Vec<Option<Result<T, E>>>> =
        Mutex::new((0..items.len()).map(|_| None).collect());
    thread::scope(|scope| {
        for _ in 0..WORKERS.min(items.len()) {
            scope.spawn(|| {
                while !stopped.load(Ordering::Relaxed) {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(item) = items.get(i) else { break };
                    let result = work(item);
                    if result.is_err() {
                        stopped.store(true, Ordering::Relaxed);
                    }
                    results.lock().unwrap_or_else(PoisonError::into_inner)[i] = Some(result);
                }
            });
        }
    });
    let mut values = Vec::with_capacity(items.len());
    for result in results.into_inner().unwrap_or_else(PoisonError::into_inner) {
        match result {
            Some(Ok(value)) => values.push(value),
            Some(Err(error)) => return Err(error),
            // Items are taken in order, so everything before the first error was done.
            None => unreachable!("an item left undone before the first error"),
        }
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_result_comes_back_in_order() {
        let items: Vec<u64> = (0..50).collect();
        let doubled = try_each(&items, |n| Ok::<u64, ()>(n * 2)).unwrap();
        assert_eq!(doubled, items.iter().map(|n| n * 2).collect::<Vec<_>>());
        let none: &[u64] = &[];
        assert!(try_each(none, |n| Ok::<u64, ()>(*n)).unwrap().is_empty());
    }

    #[test]
    fn the_first_error_in_order_ends_the_batch() {
        let items: Vec<u64> = (0..50).collect();
        let error = try_each(
            &items,
            |n| if *n == 7 || *n == 30 { Err(*n) } else { Ok(()) },
        );
        assert_eq!(error, Err(7));
    }
}
