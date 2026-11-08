//! Locks that keep working after a panic.
//!
//! A panic while a `Mutex` is held marks it poisoned, and `lock().expect(..)` then panics too, in
//! every task that touches it: one bug would take down transfers, discovery and, on Android, the
//! whole app. The data behind these locks stays usable (each change is small and self-contained),
//! so the guard is taken back and the engine carries on.

use std::sync::{Mutex, MutexGuard};

pub trait Locked<T> {
    /// Locks, recovering the guard if an earlier holder panicked.
    fn locked(&self) -> MutexGuard<'_, T>;
}

impl<T> Locked<T> for Mutex<T> {
    fn locked(&self) -> MutexGuard<'_, T> {
        self.lock().unwrap_or_else(|poisoned| {
            tracing::warn!("a lock was held during a panic; carrying on");
            poisoned.into_inner()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn a_poisoned_lock_still_locks() {
        let m = Arc::new(Mutex::new(1));
        let m2 = m.clone();
        let _ = std::thread::spawn(move || {
            let _g = m2.lock().unwrap();
            panic!("boom");
        })
        .join();
        assert!(m.is_poisoned());
        *m.locked() += 1;
        assert_eq!(*m.locked(), 2);
    }
}
