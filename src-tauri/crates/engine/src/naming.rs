use crate::model::{CollisionPolicy, Options};
use std::path::{Path, PathBuf};

/// Where a file's output should go, or a signal to skip it.
pub enum Resolved {
    Path(PathBuf),
    SkipCollision,
}

/// Compute the output path for `input`, honoring the output directory, suffix, target extension,
/// and collision policy. Never returns a path that already exists unless the policy is
/// `Overwrite`.
pub fn resolve_output_path(input: &Path, opts: &Options, ext: &str) -> Resolved {
    let parent = opts
        .output_dir
        .clone()
        .or_else(|| input.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."));

    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("image");
    let base_name = format!("{stem}{}", opts.suffix);
    let candidate = parent.join(format!("{base_name}.{ext}"));

    if !candidate.exists() {
        return Resolved::Path(candidate);
    }

    match opts.collision {
        CollisionPolicy::Overwrite => Resolved::Path(candidate),
        CollisionPolicy::Skip => Resolved::SkipCollision,
        CollisionPolicy::Suffix => {
            for n in 1..10_000 {
                let p = parent.join(format!("{base_name}-{n}.{ext}"));
                if !p.exists() {
                    return Resolved::Path(p);
                }
            }
            Resolved::SkipCollision
        }
    }
}
