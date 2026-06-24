use crate::model::{CollisionPolicy, Options};
use std::path::{Path, PathBuf};

/// Where a file's output should go, or a signal to skip it.
pub enum Resolved {
    Path(PathBuf),
    SkipCollision,
}

/// Compute the output path for `input`, honoring the output directory, suffix, target extension,
/// and collision policy. Never returns a path that already exists unless the policy is `Overwrite`,
/// and never returns the `input` path itself — writing there would destroy the original being
/// compressed, so the source is always protected by falling through to a numbered name.
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

    // Guard the source: e.g. an empty suffix + same folder + same extension would resolve to the
    // input itself. Protect it regardless of policy by numbering instead of overwriting.
    let is_source = candidate == input;

    if !is_source {
        if !candidate.exists() {
            return Resolved::Path(candidate);
        }
        match opts.collision {
            CollisionPolicy::Overwrite => return Resolved::Path(candidate),
            CollisionPolicy::Skip => return Resolved::SkipCollision,
            CollisionPolicy::Suffix => {}
        }
    }

    // Suffix policy, or protecting the source: first free numbered name that isn't the source.
    for n in 1..10_000 {
        let p = parent.join(format!("{base_name}-{n}.{ext}"));
        if p != input && !p.exists() {
            return Resolved::Path(p);
        }
    }
    Resolved::SkipCollision
}
