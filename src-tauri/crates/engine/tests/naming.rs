//! Output path resolution and the three collision policies.

use engine::model::{CollisionPolicy, Options};
use engine::naming::{resolve_output_path, Resolved};
use std::path::PathBuf;

fn unique_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ic_naming_{}_{}", tag, std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn default_is_next_to_source_with_suffix() {
    let dir = unique_dir("default");
    let input = dir.join("photo.png");
    std::fs::write(&input, b"x").unwrap();
    let opts = Options {
        output_dir: None,
        suffix: "-compressed".to_string(),
        ..Options::default()
    };
    match resolve_output_path(&input, &opts, "jpg") {
        Resolved::Path(p) => assert_eq!(p, dir.join("photo-compressed.jpg")),
        Resolved::SkipCollision => panic!("unexpected skip"),
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn output_dir_override_is_used() {
    let src_dir = unique_dir("srcdir");
    let out_dir = unique_dir("outdir");
    let input = src_dir.join("a.tiff");
    std::fs::write(&input, b"x").unwrap();
    let opts = Options {
        output_dir: Some(out_dir.clone()),
        suffix: "-c".to_string(),
        ..Options::default()
    };
    match resolve_output_path(&input, &opts, "jpg") {
        Resolved::Path(p) => assert_eq!(p, out_dir.join("a-c.jpg")),
        Resolved::SkipCollision => panic!("unexpected skip"),
    }
    std::fs::remove_dir_all(&src_dir).ok();
    std::fs::remove_dir_all(&out_dir).ok();
}

#[test]
fn collision_policies_resolve_correctly() {
    let dir = unique_dir("collision");
    let input = dir.join("a.jpg");
    std::fs::write(&input, b"x").unwrap();
    // Pre-create the default target so every policy hits a collision.
    let taken = dir.join("a-c.jpg");
    std::fs::write(&taken, b"y").unwrap();
    let base = Options {
        output_dir: Some(dir.clone()),
        suffix: "-c".to_string(),
        ..Options::default()
    };

    let suffixed = Options {
        collision: CollisionPolicy::Suffix,
        ..base.clone()
    };
    match resolve_output_path(&input, &suffixed, "jpg") {
        Resolved::Path(p) => assert_eq!(p, dir.join("a-c-1.jpg")),
        Resolved::SkipCollision => panic!("suffix policy should number, not skip"),
    }

    let overwrite = Options {
        collision: CollisionPolicy::Overwrite,
        ..base.clone()
    };
    match resolve_output_path(&input, &overwrite, "jpg") {
        Resolved::Path(p) => assert_eq!(p, taken),
        Resolved::SkipCollision => panic!("overwrite should reuse the path"),
    }

    let skip = Options {
        collision: CollisionPolicy::Skip,
        ..base
    };
    assert!(matches!(
        resolve_output_path(&input, &skip, "jpg"),
        Resolved::SkipCollision
    ));

    std::fs::remove_dir_all(&dir).ok();
}
