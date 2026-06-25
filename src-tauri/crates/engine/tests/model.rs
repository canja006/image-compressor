//! The serde contract with the TypeScript frontend: camelCase fields and tagged `kind` outcomes.

use engine::model::{
    Anchor, CollisionPolicy, FileResult, Options, Outcome, OutputFormat, ResizeMode,
};

#[test]
fn options_use_camel_case_and_round_trip() {
    let opts = Options {
        cap_bytes: 512_000,
        output_format: OutputFormat::Jpeg,
        ..Options::default()
    };
    let json = serde_json::to_string(&opts).unwrap();
    assert!(json.contains("\"capBytes\":512000"), "json: {json}");
    assert!(json.contains("\"outputFormat\":\"jpeg\""), "json: {json}");
    assert!(json.contains("\"skipIfUnderCap\":true"), "json: {json}");

    let back: Options = serde_json::from_str(&json).unwrap();
    assert_eq!(back.cap_bytes, 512_000);
    assert_eq!(back.output_format, OutputFormat::Jpeg);
}

#[test]
fn outcome_is_tagged_by_kind() {
    let json = serde_json::to_string(&Outcome::Compressed {
        final_bytes: 100,
        quality: Some(80),
        width: 10,
        height: 20,
        downscaled: true,
    })
    .unwrap();
    assert!(json.contains("\"kind\":\"compressed\""), "json: {json}");
    assert!(json.contains("\"finalBytes\":100"), "json: {json}");

    let unreachable = serde_json::to_string(&Outcome::Unreachable {
        reason: "too small".to_string(),
    })
    .unwrap();
    assert!(
        unreachable.contains("\"kind\":\"unreachable\""),
        "{unreachable}"
    );
}

#[test]
fn resize_mode_is_tagged_and_camel_case() {
    // Fit (the default) carries an optional longest-edge cap.
    let fit = serde_json::to_string(&ResizeMode::Fit {
        max_dimension: Some(1920),
    })
    .unwrap();
    assert!(fit.contains("\"mode\":\"fit\""), "json: {fit}");
    assert!(fit.contains("\"maxDimension\":1920"), "json: {fit}");

    // Exact locks an output size with an anchor and an upscale flag.
    let exact = serde_json::to_string(&ResizeMode::Exact {
        width: 1920,
        height: 1080,
        anchor: Anchor::Center,
        allow_upscale: true,
    })
    .unwrap();
    assert!(exact.contains("\"mode\":\"exact\""), "json: {exact}");
    assert!(exact.contains("\"width\":1920"), "json: {exact}");
    assert!(exact.contains("\"height\":1080"), "json: {exact}");
    assert!(exact.contains("\"anchor\":\"center\""), "json: {exact}");
    assert!(exact.contains("\"allowUpscale\":true"), "json: {exact}");

    // The frontend sends this exact shape; confirm it round-trips back into the enum.
    let back: ResizeMode = serde_json::from_str(&exact).unwrap();
    assert_eq!(
        back,
        ResizeMode::Exact {
            width: 1920,
            height: 1080,
            anchor: Anchor::Center,
            allow_upscale: true,
        }
    );

    // Options nests the resize object under "resize".
    let opts = serde_json::to_string(&Options::default()).unwrap();
    assert!(
        opts.contains("\"resize\":{\"mode\":\"fit\""),
        "json: {opts}"
    );
}

#[test]
fn collision_policy_serializes_lowercase() {
    assert_eq!(
        serde_json::to_string(&CollisionPolicy::Suffix).unwrap(),
        "\"suffix\""
    );
    assert_eq!(
        serde_json::to_string(&CollisionPolicy::Overwrite).unwrap(),
        "\"overwrite\""
    );
}

#[test]
fn file_result_round_trips() {
    let fr = FileResult {
        input: "/a.jpg".into(),
        output: Some("/a-c.jpg".into()),
        original_bytes: 999,
        outcome: Outcome::SkippedUnderCap { bytes: 999 },
    };
    let json = serde_json::to_string(&fr).unwrap();
    let back: FileResult = serde_json::from_str(&json).unwrap();
    assert_eq!(back.original_bytes, 999);
    assert!(matches!(
        back.outcome,
        Outcome::SkippedUnderCap { bytes: 999 }
    ));
}
