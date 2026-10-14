//! The spawn lock (D18S P2.8b-1): a std spawn through
//! `chief_of_staff_spawn_isolation::spawn` waits while the lock is held,
//! which is what keeps it out of the Windows applier's inheritable span.

use chief_of_staff_spawn_isolation::{spawn, spawn_lock};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// This test binary itself, asked only to list its tests: a child that
/// exists on every platform and exits at once.
fn quick_child() -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .arg("--list")
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

#[test]
fn a_spawn_waits_while_the_lock_is_held() {
    let spawned = Arc::new(AtomicBool::new(false));
    let held = spawn_lock();
    let worker = {
        let spawned = Arc::clone(&spawned);
        std::thread::spawn(move || {
            let mut child = spawn(&mut quick_child()).unwrap();
            spawned.store(true, Ordering::SeqCst);
            child.wait().unwrap()
        })
    };
    // Ample time for an unblocked spawn to have happened.
    std::thread::sleep(Duration::from_millis(500));
    assert!(
        !spawned.load(Ordering::SeqCst),
        "spawned while the lock was held"
    );
    drop(held);
    assert!(worker.join().unwrap().success());
    assert!(spawned.load(Ordering::SeqCst));
}

#[test]
fn the_lock_is_free_again_after_a_spawn() {
    spawn(&mut quick_child()).unwrap().wait().unwrap();
    drop(spawn_lock());
}
