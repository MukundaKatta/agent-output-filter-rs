//! Integration tests that exercise `agent-output-filter` through its public API,
//! mirroring how a downstream crate would compose a filtering pipeline.

use agent_output_filter::{FilterRule, OutputFilter};

/// A realistic pipeline: strip a leaked secret, drop debug log lines, collapse
/// whitespace, then cap the length of what we hand back to the user.
#[test]
fn end_to_end_cleanup_pipeline() {
    let mut filter = OutputFilter::new();
    filter
        .add_rule(FilterRule::RemoveMatchingLines(r"^\s*DEBUG:".to_string()))
        .add_rule(FilterRule::ReplacePattern {
            pattern: r"sk-[A-Za-z0-9]+".to_string(),
            replacement: "[REDACTED]".to_string(),
        })
        .add_rule(FilterRule::CollapseWhitespace)
        .add_rule(FilterRule::TruncateWords(8));

    let raw = "DEBUG: connecting to api\n\
               Here   is your key sk-ABC123 and some\n\
               extra trailing words that should be dropped";

    let cleaned = filter.apply(raw);
    assert_eq!(cleaned, "Here is your key [REDACTED] and some extra");
}

#[test]
fn validate_then_apply_round_trip() {
    let filter = OutputFilter::from(vec![
        FilterRule::TrimWhitespace,
        FilterRule::RemovePattern(r"\d+".to_string()),
    ]);
    assert!(filter.validate().is_ok());
    assert_eq!(filter.try_apply("  a1b2c3  ").unwrap(), "abc");
}

#[test]
fn invalid_regex_is_reported_by_try_apply() {
    let mut filter = OutputFilter::new();
    filter.add_rule(FilterRule::Uppercase);
    filter.add_rule(FilterRule::RemovePattern("(".to_string()));

    let err = filter.try_apply("hello").expect_err("expected an error");
    assert_eq!(err.index, 1);
    assert!(err.to_string().contains("invalid regex"));
}

#[test]
fn empty_filter_is_identity() {
    let filter = OutputFilter::new();
    assert!(filter.is_empty());
    assert_eq!(filter.apply("anything at all"), "anything at all");
}

#[test]
fn collect_from_iterator_builds_pipeline() {
    let filter: OutputFilter = [
        FilterRule::Prefix(">> ".to_string()),
        FilterRule::Suffix(" <<".to_string()),
    ]
    .into_iter()
    .collect();

    assert_eq!(filter.rule_count(), 2);
    assert_eq!(filter.apply("note"), ">> note <<");
}
