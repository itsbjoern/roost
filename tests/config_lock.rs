//! Concurrent writes serialized.

mod common;

use roost::config::{Config, RoostPaths};
use std::sync::{Arc, Barrier};
use std::thread;

#[test]
fn concurrent_writes_serialized() {
    let dir = common::temp_roost_home();
    let paths = RoostPaths::for_test(dir.path());
    roost::store::ensure_dirs(&paths).unwrap();

    let barrier = Arc::new(Barrier::new(3));
    let paths1 = paths.clone();
    let paths2 = paths.clone();
    let b1 = Arc::clone(&barrier);
    let b2 = Arc::clone(&barrier);

    let t1 = thread::spawn(move || {
        b1.wait();
        let mut config = Config::default();
        config.domains.insert("a.test".to_string(), "default".to_string());
        config.save(&paths1).unwrap();
    });

    let t2 = thread::spawn(move || {
        b2.wait();
        let mut config = Config::default();
        config.domains.insert("b.test".to_string(), "default".to_string());
        config.save(&paths2).unwrap();
    });

    barrier.wait();
    t1.join().unwrap();
    t2.join().unwrap();

    let loaded = Config::load(&paths).unwrap();
    assert!(loaded.domains.len() <= 2);
    assert!(loaded.domains.len() >= 1);
}
