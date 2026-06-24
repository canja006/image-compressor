//! The serde contract with the TypeScript frontend: camelCase fields and tagged `kind` outcomes.

use engine::model::{CollisionPolicy, FileResult, Options, Outcome, OutputFormat};

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
