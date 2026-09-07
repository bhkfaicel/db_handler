//! Test-only guard around the process-global `CONFIG_FILE` environment
//! variable.
//!
//! Tests that point the config loader at a temporary file mutate a
//! process-global variable; two such tests running in parallel (default
//! test threads) could otherwise overwrite or restore each other's value
//! mid-run and nondeterministically make one of them read the wrong
//! configuration. This module serializes every such mutation behind a
//! single process-wide mutex and restores the variable's previous value
//! automatically when the guard is dropped, even on early returns.
//!
//! Test-only by construction: the module is compiled out of any
//! non-test build (`#[cfg(test)]` on the module declaration in the crate
//! root), so production code is untouched.

use std::sync::{Mutex, MutexGuard};

/// Process-wide lock serializing every `CONFIG_FILE` mutation in tests.
static CONFIG_FILE_LOCK: Mutex<()> = Mutex::new(());

/// Holds the environment lock plus the value `CONFIG_FILE` had before the
/// test mutated it; restoring it on `Drop` guarantees the variable can
/// never leak past a test, whatever the test's control flow.
pub struct ConfigFileGuard {
    _lock: MutexGuard<'static, ()>,
    original: Option<String>,
}

impl ConfigFileGuard {
    /// Acquires the global `CONFIG_FILE` lock and remembers the current
    /// value so it can be restored on drop.
    pub fn acquire() -> Self {
        // Ignore poisoning: another test panicked while holding the lock;
        // its test binary is already failing, and a poisoned lock must not
        // cascade the panic into unrelated tests.
        let _lock = CONFIG_FILE_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        Self {
            _lock,
            original: std::env::var("CONFIG_FILE").ok(),
        }
    }

    /// Points `CONFIG_FILE` at `value` (`None` unsets it) while holding
    /// the global lock; the pre-acquisition value is restored on drop.
    pub fn set(&self, value: Option<&str>) {
        match value {
            Some(v) => std::env::set_var("CONFIG_FILE", v),
            None => std::env::remove_var("CONFIG_FILE"),
        }
    }
}

impl Drop for ConfigFileGuard {
    /// Restores the value `CONFIG_FILE` had when the guard was acquired.
    fn drop(&mut self) {
        match &self.original {
            Some(v) => std::env::set_var("CONFIG_FILE", v),
            None => std::env::remove_var("CONFIG_FILE"),
        }
    }
}
