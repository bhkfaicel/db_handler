//! Tests for the users-domain password helpers, relocated with the code
//! they cover.

use super::{
    hash_password, hash_password_async, verify_password, verify_password_async, DUMMY_BCRYPT_HASH,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

// The login lookup verifies unknown emails against DUMMY_BCRYPT_HASH to
// equalize bcrypt CPU cost across branches (no timing oracle). This test
// pins the two properties that make that work: the constant is a real
// cost-12 bcrypt hash (same `$2b$12$` cost policy as stored password
// hashes), and verifying a wrong password against it returns Ok(false) —
// which bcrypt can only answer after actually running the key-derivation
// over the dummy hash. So whenever the repository's unknown-email branch
// completes, real bcrypt work has provably happened.
#[test]
fn test_dummy_hash_is_valid_at_default_cost_and_rejects_passwords() {
    assert!(
        DUMMY_BCRYPT_HASH.starts_with("$2b$12$"),
        "dummy hash must carry the same cost policy as stored hashes"
    );
    assert!(
        !verify_password("definitely-not-the-dummy-source", DUMMY_BCRYPT_HASH).unwrap(),
        "no realistic password may verify against the dummy hash"
    );
}

#[test]
fn test_hash_password() {
    let password = "super_secret";
    let hashed = hash_password(password).unwrap();
    assert!(verify_password(password, &hashed).unwrap());
}

#[test]
fn test_verify_password_incorrect() {
    let password = "super_secret";
    let hashed = hash_password(password).unwrap();
    assert!(!verify_password("wrong_password", &hashed).unwrap());
}

#[test]
fn test_verify_password_correct() {
    let password = "super_secret";
    let hashed = hash_password(password).unwrap();
    assert!(verify_password(password, &hashed).unwrap());
}

#[tokio::test]
async fn test_hash_password_async_round_trip() {
    // The async wrapper must produce a hash the async verifier accepts,
    // with inputs moved in and results moved out unchanged.
    let password = "super_secret";
    let hashed = hash_password_async(password.to_string()).await.unwrap();
    assert!(verify_password_async(password.to_string(), hashed.clone())
        .await
        .unwrap());
}

#[tokio::test]
async fn test_verify_password_async_rejects_wrong_password() {
    let password = "super_secret";
    let hashed = hash_password_async(password.to_string()).await.unwrap();
    assert!(!verify_password_async("wrong_password".to_string(), hashed)
        .await
        .unwrap());
}

// Regression guard: hashing must stay off the async worker. A progress task
// ticks a shared counter in a yield loop; because the runtime below is
// single-threaded (`current_thread`), those ticks can only be observed while
// this test's own task is parked. We sample the counter around ONE call to
// the helper and require strictly positive progress: the helper's body holds
// exactly one await (the `spawn_blocking` join), so ticks during that await
// prove bcrypt ran on the blocking pool. If hashing ever regresses to an
// inline call, the worker is busy for the whole hash with no yield point
// between the samples and the delta is exactly zero — a binary,
// core-count- and CPU-quota-independent discriminator (no wall clock, no
// tick floor).
//
// The guard deliberately targets the helper, not a public operation: both
// public paths await SQL around the bcrypt call (login reads the stored hash
// before verifying; user creation and update insert/update after hashing),
// so an inline regression on a public operation would still tick during the
// unrelated SQL await and the guard would pass vacuously. Sampling only the
// bcrypt-containing helper keeps the zero-vs-nonzero signal uncontaminated.
#[tokio::test(flavor = "current_thread")]
async fn test_hash_password_async_keeps_worker_free() {
    // Counter incremented by the progress task on every scheduling round.
    let counter = Arc::new(AtomicUsize::new(0));
    let progress_counter = Arc::clone(&counter);

    // Progress task: tick, then yield, forever — it runs only while the
    // worker is free to schedule other tasks.
    let handle = tokio::spawn(async move {
        loop {
            progress_counter.fetch_add(1, Ordering::Relaxed);
            tokio::task::yield_now().await;
        }
    });

    // Warm-up: a spawned task on a current-thread runtime only executes when
    // this test's task yields, so spin until at least one tick is observed
    // before taking the baseline sample.
    while counter.load(Ordering::Relaxed) == 0 {
        tokio::task::yield_now().await;
    }
    let sample_before = counter.load(Ordering::Relaxed);

    // The single await under observation: with the offload intact the worker
    // parks here for the whole bcrypt duration and the progress task ticks.
    hash_password_async("x".to_string()).await.unwrap();

    let sample_after = counter.load(Ordering::Relaxed);

    // Cleanup discipline: stop the progress task, then join its handle so it
    // is fully retired before the test ends.
    handle.abort();
    let _ = handle.await;

    // Binary assertion: any tick observed during the bcrypt await proves the
    // worker stayed free (offload intact); an inline regression yields none.
    assert!(
        sample_after > sample_before,
        "no runtime progress observed during hash_password_async: \
         bcrypt is running inline on the async worker instead of the \
         blocking pool"
    );
}
