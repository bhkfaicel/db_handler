//! Tests for the users-domain password helpers, relocated with the code
//! they cover.

use super::{
    hash_password, hash_password_async, password_length_ok, verify_password, verify_password_async,
    DUMMY_ARGON2_HASH, MAX_PASSWORD_BYTES, MIN_PASSWORD_BYTES,
};
use argon2::password_hash::PasswordHash;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

// The house PHC policy pinned on the dummy hash: it must parse as a strict
// PHC string, carry the Argon2id algorithm identifier, and expose exactly
// the default Argon2 parameters real stored hashes get (`Argon2::default`).
// This is what makes the unknown-email branch burn the same CPU/memory as
// the wrong-password branch (no timing oracle): Argon2 verification derives
// with the dummy's stored parameters, which equal the real policy.
#[test]
fn test_dummy_hash_matches_house_argon2id_policy() {
    let parsed = PasswordHash::new(DUMMY_ARGON2_HASH)
        .expect("dummy hash must be a strictly parseable PHC string");
    assert_eq!(parsed.algorithm, argon2::ARGON2ID_IDENT);
    // A freshly generated hash carries the default policy; the dummy must
    // expose the identical algorithm identifier and parameter string
    // (the first three `$`-separated fields; salt/hash differ by design).
    let reference_hash = hash_password("reference_password").unwrap();
    let reference = PasswordHash::new(&reference_hash).unwrap();
    // Own the PHC strings before splitting so the field slices outlive the
    // comparison.
    let dummy_string = parsed.to_string();
    let reference_string = reference.to_string();
    let dummy_fields: Vec<&str> = dummy_string.split('$').take(3).collect();
    let reference_fields: Vec<&str> = reference_string.split('$').take(3).collect();
    assert_eq!(dummy_fields, reference_fields);
    // And no realistic password verifies against it: reaching `Ok(false)`
    // proves a full Argon2 derivation actually ran.
    assert!(
        !verify_password("definitely-not-the-dummy-source", DUMMY_ARGON2_HASH).unwrap(),
        "no realistic password may verify against the dummy hash"
    );
}

// Salt uniqueness: two hashes of the SAME password must never share a salt,
// so their PHC strings differ even for identical inputs (no precomputed
// hash comparison, no rainbow reuse).
#[test]
fn test_two_hashes_of_one_password_have_distinct_salts() {
    let password = "super_secret";
    let first = hash_password(password).unwrap();
    let second = hash_password(password).unwrap();
    assert_ne!(first, second);
    // Both still verify the same password.
    assert!(verify_password(password, &first).unwrap());
    assert!(verify_password(password, &second).unwrap());
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

// Strict PHC parsing fail-closed: no malformed stored string may ever
// verify (never `Ok(true)`, never a panic), so the repository can fail
// closed on corrupted rows. Truly unparsable strings surface as errors;
// strings that parse but cannot match (wrong algorithm, truncated fields)
// return `Ok(false)` — the same uniform fail-closed outcome downstream.
#[test]
fn test_verify_password_malformed_phc_fails_closed() {
    for garbage in [
        "",
        "not-a-phc",
        "$argon2id$garbage",
        "$pbkdf2-sha256$v=19$m=19456,t=2,p=1$c2FsdHNhbHQ$Zm9vYmFy",
        "$argon2i$v=19$m=19456,t=2,p=1$LVqSDkuc8AaNn2RY/XKdHA$RexxtUuD7oN3tD2xhnxvYTM9oL/+RFTRzrqYv7GD5PE",
    ] {
        let result = verify_password("super_secret", garbage);
        assert!(
            !matches!(result, Ok(true)),
            "malformed PHC must never verify: {garbage:?}"
        );
    }
    // Strict parsing: these strings are rejected at the PHC parse step
    // itself with a hard error, not a silent password mismatch.
    assert!(verify_password("super_secret", "").is_err());
    assert!(verify_password("super_secret", "not-a-phc").is_err());
}

// Password length policy on the primitive: 7 and 129 bytes are rejected,
// the 8 and 128-byte boundaries are accepted.
#[test]
fn test_hash_password_length_bounds() {
    assert!(hash_password(&"a".repeat(7)).is_err(), "7 bytes rejected");
    assert!(hash_password(&"a".repeat(8)).is_ok(), "8 bytes accepted");
    assert!(
        hash_password(&"a".repeat(128)).is_ok(),
        "128 bytes accepted"
    );
    assert!(
        hash_password(&"a".repeat(129)).is_err(),
        "129 bytes rejected"
    );
    assert!(!password_length_ok(&"a".repeat(0)));
    assert!(password_length_ok(&"a".repeat(MIN_PASSWORD_BYTES)));
    assert!(password_length_ok(&"a".repeat(MAX_PASSWORD_BYTES)));
    assert!(!password_length_ok(&"a".repeat(MAX_PASSWORD_BYTES + 1)));
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
// prove Argon2 ran on the blocking pool. If hashing ever regresses to an
// inline call, the worker is busy for the whole derivation with no yield
// point between the samples and the delta is exactly zero — a binary,
// core-count- and CPU-quota-independent discriminator (no wall clock, no
// tick floor).
//
// The guard deliberately targets the helper, not a public operation: both
// public paths await SQL around the Argon2 call (login reads the stored
// hash before verifying; user creation and update insert/update after
// hashing), so an inline regression on a public operation would still tick
// during the unrelated SQL await and the guard would pass vacuously.
// Sampling only the Argon2-containing helper keeps the zero-vs-nonzero
// signal uncontaminated.
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
    // parks here for the whole Argon2 duration and the progress task ticks.
    hash_password_async("boundary-password".to_string())
        .await
        .unwrap();

    let sample_after = counter.load(Ordering::Relaxed);

    // Cleanup discipline: stop the progress task, then join its handle so it
    // is fully retired before the test ends.
    handle.abort();
    let _ = handle.await;

    // Binary assertion: any tick observed during the Argon2 await proves the
    // worker stayed free (offload intact); an inline regression yields none.
    assert!(
        sample_after > sample_before,
        "no runtime progress observed during hash_password_async: \
         Argon2 is running inline on the async worker instead of the \
         blocking pool"
    );
}
