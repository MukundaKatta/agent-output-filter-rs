/*!
`agent-output-filter`: filter and transform LLM agent output before delivery.

This crate provides a small, dependency-light pipeline for cleaning up text
produced by language-model agents before it is shown to a user, logged, or
forwarded to another system. You compose an ordered list of [`FilterRule`]s on
an [`OutputFilter`] and call [`OutputFilter::apply`] to run them in sequence.

# Quick start

```rust
use agent_output_filter::{OutputFilter, FilterRule};

let mut f = OutputFilter::new();
f.add_rule(FilterRule::TrimWhitespace);
f.add_rule(FilterRule::RemovePattern(r"\[REDACTED\]".to_string()));
let result = f.apply("  Hello [REDACTED] world  ");
assert_eq!(result, "Hello  world");
```

# Builder style

[`OutputFilter::add_rule`] returns `&mut Self`, so rules can be chained:

```rust
use agent_output_filter::{OutputFilter, FilterRule};

let mut f = OutputFilter::new();
f.add_rule(FilterRule::TrimWhitespace)
 .add_rule(FilterRule::CollapseWhitespace)
 .add_rule(FilterRule::Uppercase);
assert_eq!(f.apply("  hello   world  "), "HELLO WORLD");
```

# Validating regex rules

Rules that take a regex ([`FilterRule::RemovePattern`],
[`FilterRule::ReplacePattern`], [`FilterRule::RemoveMatchingLines`]) silently
pass text through unchanged when the pattern fails to compile. Call
[`OutputFilter::validate`] up front to surface a bad pattern instead of having
it silently do nothing:

```rust
use agent_output_filter::{OutputFilter, FilterRule};

let mut f = OutputFilter::new();
f.add_rule(FilterRule::RemovePattern("(".to_string())); // unbalanced paren
assert!(f.validate().is_err());
```
*/

use regex::Regex;

/// A rule for transforming or filtering output text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterRule {
    /// Remove leading/trailing whitespace.
    TrimWhitespace,
    /// Collapse every run of whitespace down to a single space.
    ///
    /// Newlines, tabs, and repeated spaces are all normalized to one space.
    /// Leading and trailing whitespace is also removed.
    CollapseWhitespace,
    /// Replace a regex pattern with the empty string.
    ///
    /// If the pattern does not compile the text is returned unchanged. Use
    /// [`OutputFilter::validate`] to detect invalid patterns ahead of time.
    RemovePattern(String),
    /// Replace a regex pattern with a replacement string.
    ///
    /// The replacement may reference capture groups (for example `$1`). If the
    /// pattern does not compile the text is returned unchanged.
    ReplacePattern {
        /// The regular expression to match.
        pattern: String,
        /// The replacement string (supports `$N` capture references).
        replacement: String,
    },
    /// Truncate to at most `n` characters (Unicode scalar values).
    TruncateChars(usize),
    /// Truncate to at most `n` whitespace-separated words.
    TruncateWords(usize),
    /// Convert to lowercase.
    Lowercase,
    /// Convert to uppercase.
    Uppercase,
    /// Remove every line that matches the given regex pattern.
    ///
    /// If the pattern does not compile the text is returned unchanged.
    RemoveMatchingLines(String),
    /// Prepend a prefix string.
    Prefix(String),
    /// Append a suffix string.
    Suffix(String),
}

impl FilterRule {
    /// Return the regex pattern this rule would compile, if any.
    ///
    /// Rules that do not use a regex return `None`.
    fn pattern(&self) -> Option<&str> {
        match self {
            FilterRule::RemovePattern(p)
            | FilterRule::RemoveMatchingLines(p)
            | FilterRule::ReplacePattern { pattern: p, .. } => Some(p),
            _ => None,
        }
    }
}

/// An error describing a rule whose regex pattern failed to compile.
#[derive(Debug, Clone)]
pub struct InvalidRuleError {
    /// Index of the offending rule within the filter's rule list.
    pub index: usize,
    /// The pattern string that failed to compile.
    pub pattern: String,
    /// A human-readable description of why compilation failed.
    pub message: String,
}

impl std::fmt::Display for InvalidRuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "rule {} has an invalid regex pattern {:?}: {}",
            self.index, self.pattern, self.message
        )
    }
}

impl std::error::Error for InvalidRuleError {}

/// Applies a chain of [`FilterRule`]s to output text, in order.
///
/// Create a filter with [`OutputFilter::new`] (or [`OutputFilter::default`]),
/// add rules with [`OutputFilter::add_rule`], then transform text with
/// [`OutputFilter::apply`].
#[derive(Debug, Default, Clone)]
pub struct OutputFilter {
    rules: Vec<FilterRule>,
}

impl OutputFilter {
    /// Create a new, empty filter with no rules.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a filter pre-populated with the given rules.
    pub fn with_rules(rules: Vec<FilterRule>) -> Self {
        Self { rules }
    }

    /// Append a rule to the end of the pipeline.
    ///
    /// Returns `&mut Self` so calls can be chained.
    pub fn add_rule(&mut self, rule: FilterRule) -> &mut Self {
        self.rules.push(rule);
        self
    }

    /// Apply all rules in order and return the resulting string.
    ///
    /// Rules whose regex fails to compile are skipped (the text passes through
    /// unchanged for that rule). Call [`OutputFilter::validate`] first if you
    /// want compilation errors surfaced instead.
    pub fn apply(&self, text: &str) -> String {
        let mut s = text.to_string();
        for rule in &self.rules {
            s = apply_rule(s, rule);
        }
        s
    }

    /// Check that every regex-based rule compiles successfully.
    ///
    /// Returns the first [`InvalidRuleError`] encountered, or `Ok(())` if all
    /// rules are valid. Rules that do not use a regex are always valid.
    pub fn validate(&self) -> Result<(), InvalidRuleError> {
        for (index, rule) in self.rules.iter().enumerate() {
            if let Some(pattern) = rule.pattern() {
                if let Err(e) = Regex::new(pattern) {
                    return Err(InvalidRuleError {
                        index,
                        pattern: pattern.to_string(),
                        message: e.to_string(),
                    });
                }
            }
        }
        Ok(())
    }

    /// Validate all rules, then apply them.
    ///
    /// Unlike [`OutputFilter::apply`], a rule with an invalid regex causes an
    /// error to be returned rather than being silently skipped.
    pub fn try_apply(&self, text: &str) -> Result<String, InvalidRuleError> {
        self.validate()?;
        Ok(self.apply(text))
    }

    /// Number of rules currently in the pipeline.
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Whether the filter has no rules.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Borrow the current list of rules.
    pub fn rules(&self) -> &[FilterRule] {
        &self.rules
    }

    /// Remove all rules, leaving an empty filter.
    pub fn clear(&mut self) {
        self.rules.clear();
    }
}

impl From<Vec<FilterRule>> for OutputFilter {
    fn from(rules: Vec<FilterRule>) -> Self {
        Self::with_rules(rules)
    }
}

impl Extend<FilterRule> for OutputFilter {
    fn extend<T: IntoIterator<Item = FilterRule>>(&mut self, iter: T) {
        self.rules.extend(iter);
    }
}

impl FromIterator<FilterRule> for OutputFilter {
    fn from_iter<T: IntoIterator<Item = FilterRule>>(iter: T) -> Self {
        Self {
            rules: iter.into_iter().collect(),
        }
    }
}

fn apply_rule(text: String, rule: &FilterRule) -> String {
    match rule {
        FilterRule::TrimWhitespace => text.trim().to_string(),
        FilterRule::CollapseWhitespace => text.split_whitespace().collect::<Vec<_>>().join(" "),
        FilterRule::RemovePattern(pat) => match Regex::new(pat) {
            Ok(re) => re.replace_all(&text, "").to_string(),
            Err(_) => text,
        },
        FilterRule::ReplacePattern {
            pattern,
            replacement,
        } => match Regex::new(pattern) {
            Ok(re) => re.replace_all(&text, replacement.as_str()).to_string(),
            Err(_) => text,
        },
        FilterRule::TruncateChars(n) => text.chars().take(*n).collect(),
        FilterRule::TruncateWords(n) => text
            .split_whitespace()
            .take(*n)
            .collect::<Vec<_>>()
            .join(" "),
        FilterRule::Lowercase => text.to_lowercase(),
        FilterRule::Uppercase => text.to_uppercase(),
        FilterRule::RemoveMatchingLines(pat) => match Regex::new(pat) {
            Ok(re) => text
                .lines()
                .filter(|line| !re.is_match(line))
                .collect::<Vec<_>>()
                .join("\n"),
            Err(_) => text,
        },
        FilterRule::Prefix(p) => format!("{}{}", p, text),
        FilterRule::Suffix(s) => format!("{}{}", text, s),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trim_whitespace() {
        let mut f = OutputFilter::new();
        f.add_rule(FilterRule::TrimWhitespace);
        assert_eq!(f.apply("  hello  "), "hello");
    }

    #[test]
    fn collapse_whitespace() {
        let mut f = OutputFilter::new();
        f.add_rule(FilterRule::CollapseWhitespace);
        assert_eq!(f.apply("  hello \t\n  world  "), "hello world");
    }

    #[test]
    fn collapse_whitespace_empty() {
        let mut f = OutputFilter::new();
        f.add_rule(FilterRule::CollapseWhitespace);
        assert_eq!(f.apply("   \n\t  "), "");
    }

    #[test]
    fn remove_pattern() {
        let mut f = OutputFilter::new();
        f.add_rule(FilterRule::RemovePattern(r"\d+".to_string()));
        assert_eq!(f.apply("abc123def"), "abcdef");
    }

    #[test]
    fn replace_pattern() {
        let mut f = OutputFilter::new();
        f.add_rule(FilterRule::ReplacePattern {
            pattern: r"foo".to_string(),
            replacement: "bar".to_string(),
        });
        assert_eq!(f.apply("foo baz foo"), "bar baz bar");
    }

    #[test]
    fn replace_pattern_with_capture_group() {
        let mut f = OutputFilter::new();
        f.add_rule(FilterRule::ReplacePattern {
            pattern: r"(\w+)@(\w+)".to_string(),
            replacement: "$1 at $2".to_string(),
        });
        assert_eq!(f.apply("user@host"), "user at host");
    }

    #[test]
    fn truncate_chars() {
        let mut f = OutputFilter::new();
        f.add_rule(FilterRule::TruncateChars(5));
        assert_eq!(f.apply("hello world"), "hello");
    }

    #[test]
    fn truncate_words() {
        let mut f = OutputFilter::new();
        f.add_rule(FilterRule::TruncateWords(2));
        assert_eq!(f.apply("one two three four"), "one two");
    }

    #[test]
    fn lowercase() {
        let mut f = OutputFilter::new();
        f.add_rule(FilterRule::Lowercase);
        assert_eq!(f.apply("HELLO"), "hello");
    }

    #[test]
    fn uppercase() {
        let mut f = OutputFilter::new();
        f.add_rule(FilterRule::Uppercase);
        assert_eq!(f.apply("hello"), "HELLO");
    }

    #[test]
    fn remove_matching_lines() {
        let mut f = OutputFilter::new();
        f.add_rule(FilterRule::RemoveMatchingLines(r"^#".to_string()));
        let input = "# comment\nhello\n# another";
        assert_eq!(f.apply(input), "hello");
    }

    #[test]
    fn prefix_and_suffix() {
        let mut f = OutputFilter::new();
        f.add_rule(FilterRule::Prefix("<<".to_string()));
        f.add_rule(FilterRule::Suffix(">>".to_string()));
        assert_eq!(f.apply("hello"), "<<hello>>");
    }

    #[test]
    fn rules_applied_in_order() {
        let mut f = OutputFilter::new();
        f.add_rule(FilterRule::TrimWhitespace);
        f.add_rule(FilterRule::Uppercase);
        assert_eq!(f.apply("  hello  "), "HELLO");
    }

    #[test]
    fn no_rules_passthrough() {
        let f = OutputFilter::new();
        assert_eq!(f.apply("unchanged"), "unchanged");
    }

    #[test]
    fn rule_count() {
        let mut f = OutputFilter::new();
        f.add_rule(FilterRule::Lowercase);
        f.add_rule(FilterRule::TrimWhitespace);
        assert_eq!(f.rule_count(), 2);
    }

    #[test]
    fn truncate_words_fewer_than_n() {
        let mut f = OutputFilter::new();
        f.add_rule(FilterRule::TruncateWords(10));
        assert_eq!(f.apply("one two"), "one two");
    }

    #[test]
    fn unicode_truncate_chars() {
        let mut f = OutputFilter::new();
        f.add_rule(FilterRule::TruncateChars(3));
        let r = f.apply("日本語テスト");
        assert_eq!(r.chars().count(), 3);
    }

    #[test]
    fn add_rule_is_chainable() {
        let mut f = OutputFilter::new();
        f.add_rule(FilterRule::TrimWhitespace)
            .add_rule(FilterRule::CollapseWhitespace)
            .add_rule(FilterRule::Uppercase);
        assert_eq!(f.rule_count(), 3);
        assert_eq!(f.apply("  hello   world  "), "HELLO WORLD");
    }

    #[test]
    fn with_rules_constructor() {
        let f =
            OutputFilter::with_rules(vec![FilterRule::Uppercase, FilterRule::Suffix("!".into())]);
        assert_eq!(f.rule_count(), 2);
        assert_eq!(f.apply("hi"), "HI!");
    }

    #[test]
    fn from_vec_and_iter() {
        let f = OutputFilter::from(vec![FilterRule::Uppercase]);
        assert_eq!(f.apply("a"), "A");

        let f2: OutputFilter = [FilterRule::Lowercase].into_iter().collect();
        assert_eq!(f2.apply("A"), "a");
    }

    #[test]
    fn extend_adds_rules() {
        let mut f = OutputFilter::new();
        f.extend(vec![FilterRule::Lowercase, FilterRule::TrimWhitespace]);
        assert_eq!(f.rule_count(), 2);
    }

    #[test]
    fn is_empty_and_clear() {
        let mut f = OutputFilter::new();
        assert!(f.is_empty());
        f.add_rule(FilterRule::Lowercase);
        assert!(!f.is_empty());
        f.clear();
        assert!(f.is_empty());
        assert_eq!(f.apply("Unchanged"), "Unchanged");
    }

    #[test]
    fn rules_accessor() {
        let f = OutputFilter::with_rules(vec![FilterRule::Lowercase]);
        assert_eq!(f.rules(), &[FilterRule::Lowercase]);
    }

    #[test]
    fn invalid_pattern_passes_through_with_apply() {
        let mut f = OutputFilter::new();
        f.add_rule(FilterRule::RemovePattern("(".to_string()));
        // apply() silently leaves text unchanged for an uncompilable pattern.
        assert_eq!(f.apply("(keep me)"), "(keep me)");
    }

    #[test]
    fn validate_detects_invalid_pattern() {
        let mut f = OutputFilter::new();
        f.add_rule(FilterRule::TrimWhitespace);
        f.add_rule(FilterRule::RemovePattern("(".to_string()));
        let err = f.validate().expect_err("expected invalid pattern error");
        assert_eq!(err.index, 1);
        assert_eq!(err.pattern, "(");
        assert!(!err.message.is_empty());
    }

    #[test]
    fn validate_accepts_valid_rules() {
        let mut f = OutputFilter::new();
        f.add_rule(FilterRule::RemovePattern(r"\d+".to_string()));
        f.add_rule(FilterRule::ReplacePattern {
            pattern: r"a".to_string(),
            replacement: "b".to_string(),
        });
        f.add_rule(FilterRule::RemoveMatchingLines(r"^#".to_string()));
        assert!(f.validate().is_ok());
    }

    #[test]
    fn try_apply_ok_and_err() {
        let mut f = OutputFilter::new();
        f.add_rule(FilterRule::Uppercase);
        assert_eq!(f.try_apply("hi").unwrap(), "HI");

        let mut bad = OutputFilter::new();
        bad.add_rule(FilterRule::ReplacePattern {
            pattern: "[".to_string(),
            replacement: "x".to_string(),
        });
        assert!(bad.try_apply("anything").is_err());
    }

    #[test]
    fn invalid_rule_error_display_mentions_index() {
        let mut f = OutputFilter::new();
        f.add_rule(FilterRule::RemoveMatchingLines("[".to_string()));
        let err = f.validate().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("rule 0"));
        assert!(msg.contains('['));
    }
}
