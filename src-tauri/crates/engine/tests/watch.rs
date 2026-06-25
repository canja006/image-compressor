use engine::watch::{should_ingest, SettleTracker};
use std::path::Path;

#[test]
fn rejects_non_images() {
    assert!(!should_ingest(Path::new("/w/readme.txt"), None));
}

#[test]
fn accepts_supported_images_case_insensitively() {
    assert!(should_ingest(Path::new("/w/photo.JPG"), None));
    assert!(should_ingest(Path::new("/w/a.png"), None));
}

#[test]
fn excludes_paths_inside_the_output_dir() {
    // Non-existent paths -> canonicalize fails -> lexical starts_with is used.
    let out = Path::new("/w/out");
    assert!(!should_ingest(Path::new("/w/out/a.jpg"), Some(out)));
    assert!(should_ingest(Path::new("/w/a.jpg"), Some(out)));
}

#[test]
fn settle_tracker_needs_consecutive_stable_sizes() {
    let mut t = SettleTracker::new(3);
    assert!(!t.observe(100)); // streak 1
    assert!(!t.observe(100)); // streak 2
    assert!(t.observe(100)); // streak 3 -> settled
}

#[test]
fn settle_tracker_resets_on_change() {
    let mut t = SettleTracker::new(2);
    assert!(!t.observe(100)); // 1
    assert!(!t.observe(200)); // changed -> streak 1 again
    assert!(t.observe(200)); // 2 -> settled
}

#[test]
fn settle_tracker_ignores_zero_size() {
    let mut t = SettleTracker::new(1);
    assert!(!t.observe(0)); // empty file not settled
    assert!(t.observe(50)); // non-zero, required_stable 1 -> settled
}
