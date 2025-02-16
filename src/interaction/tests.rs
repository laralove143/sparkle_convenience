use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[test]
fn atomic_preserved() {
    let responded = Arc::new(AtomicBool::new(false));
    let responded_clone = Arc::clone(&responded);

    responded.store(true, Ordering::Release);

    assert!(responded_clone.load(Ordering::Acquire));
}
