//! Filename token expansion for bulk rename (pure Rust, no deps). Expands a user pattern like
//! `{name}-{seq:000}` into a filesystem-safe base filename (no extension). The caller supplies the
//! date string so this stays deterministic and clock-free.

/// Context for expanding a rename pattern into a base filename (no extension).
pub struct NameContext<'a> {
    /// The original file stem (filename without extension).
    pub stem: &'a str,
    /// 1-based sequence number of this file within the batch.
    pub seq: usize,
    /// Output image width in pixels.
    pub width: u32,
    /// Output image height in pixels.
    pub height: u32,
    /// Preformatted date string, e.g. "2026-06-25" (supplied by the caller).
    pub date: &'a str,
}

/// Expand a rename pattern into a filesystem-safe base filename (NO extension). Supported tokens:
/// `{name}`, `{seq}`, `{seq:000}` (zero-pad to the count of `0`s), `{date}`, `{w}`, `{h}`. Unknown
/// tokens are left literal (braces included); text outside braces is copied verbatim.
pub fn expand_name(pattern: &str, ctx: &NameContext) -> String {
    let chars: Vec<char> = pattern.chars().collect();
    let mut out = String::with_capacity(pattern.len() + 16);
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '{' {
            match (i + 1..chars.len()).find(|&j| chars[j] == '}') {
                Some(close) => {
                    let token: String = chars[i + 1..close].iter().collect();
                    match expand_token(&token, ctx) {
                        Some(rep) => out.push_str(&rep),
                        // Unknown token: keep it literal, braces and all.
                        None => {
                            out.push('{');
                            out.push_str(&token);
                            out.push('}');
                        }
                    }
                    i = close + 1;
                    continue;
                }
                // No closing brace: the rest of the pattern is literal text.
                None => {
                    out.extend(chars[i..].iter());
                    break;
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }

    let sanitized = sanitize_filename(&out);
    if !sanitized.is_empty() {
        return sanitized;
    }
    let from_stem = sanitize_filename(ctx.stem);
    if from_stem.is_empty() {
        "image".to_string()
    } else {
        from_stem
    }
}

/// Replacement for a single token, or `None` if the token is unrecognized.
fn expand_token(token: &str, ctx: &NameContext) -> Option<String> {
    match token {
        "name" => Some(ctx.stem.to_string()),
        "date" => Some(ctx.date.to_string()),
        "w" => Some(ctx.width.to_string()),
        "h" => Some(ctx.height.to_string()),
        "seq" => Some(ctx.seq.to_string()),
        _ => {
            // `{seq:000}` — the argument is a run of '0's whose length is the pad width.
            let pad = token.strip_prefix("seq:")?;
            if !pad.is_empty() && pad.bytes().all(|b| b == b'0') {
                Some(format!("{:0width$}", ctx.seq, width = pad.len()))
            } else {
                None
            }
        }
    }
}

/// Make a string safe as a filename: replace path separators and Windows-illegal characters (and
/// control chars) with '_', then trim leading/trailing spaces and dots (invalid on Windows).
fn sanitize_filename(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if (c as u32) < 0x20 => '_',
            c => c,
        })
        .collect();
    cleaned.trim_matches(|c| c == ' ' || c == '.').to_string()
}
