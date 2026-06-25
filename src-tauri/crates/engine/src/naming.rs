use crate::model::{CollisionPolicy, Options};
use crate::rename::{expand_name, NameContext};
use std::path::{Path, PathBuf};

/// Where a file's output should go, or a signal to skip it.
pub enum Resolved {
    Path(PathBuf),
    SkipCollision,
}

/// Dynamic naming context the batch supplies for rename-token expansion (the parts not derivable
/// from the input path alone: the batch sequence number, output dimensions, and the date).
pub struct NameInfo<'a> {
    pub seq: usize,
    pub width: u32,
    pub height: u32,
    pub date: &'a str,
}

/// The output base filename (no extension): the rename pattern expanded if one is set, otherwise the
/// classic `stem + suffix`.
pub fn output_base_name(input: &Path, opts: &Options, info: &NameInfo) -> String {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("image");
    match &opts.rename_pattern {
        Some(pattern) if !pattern.trim().is_empty() => {
            let ctx = NameContext {
                stem,
                seq: info.seq,
                width: info.width,
                height: info.height,
                date: info.date,
            };
            expand_name(pattern, &ctx)
        }
        _ => format!("{stem}{}", opts.suffix),
    }
}

/// Compute the output path for `input` using the default `stem + suffix` base name. Kept for callers
/// (and tests) that don't supply a rename context; delegates to [`resolve_output_path_with_base`].
pub fn resolve_output_path(input: &Path, opts: &Options, ext: &str) -> Resolved {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("image");
    let base = format!("{stem}{}", opts.suffix);
    resolve_output_path_with_base(input, opts, ext, &base)
}

/// Compute the output path for `input` given an already-resolved `base_name` (no extension),
/// honoring the output directory, target extension, and collision policy. Never returns a path that
/// already exists unless the policy is `Overwrite`, and never returns the `input` path itself —
/// writing there would destroy the original, so the source is protected by falling through to a
/// numbered name.
pub fn resolve_output_path_with_base(
    input: &Path,
    opts: &Options,
    ext: &str,
    base_name: &str,
) -> Resolved {
    let parent = opts
        .output_dir
        .clone()
        .or_else(|| input.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."));

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
