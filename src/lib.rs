/*!
agent-output-filter: filter and transform LLM output before delivery.

```rust
use agent_output_filter::{OutputFilter, FilterRule};

let mut f = OutputFilter::new();
f.add_rule(FilterRule::TrimWhitespace);
f.add_rule(FilterRule::RemovePattern(r"\[REDACTED\]".to_string()));
let result = f.apply("  Hello [REDACTED] world  ");
assert_eq!(result, "Hello  world");
```
*/

use regex::Regex;

/// A rule for transforming or filtering output text.
#[derive(Debug, Clone)]
pub enum FilterRule {
    /// Remove leading/trailing whitespace.
    TrimWhitespace,
    /// Replace a regex pattern with empty string.
    RemovePattern(String),
    /// Replace a regex pattern with a replacement string.
    ReplacePattern { pattern: String, replacement: String },
    /// Truncate to max character count.
    TruncateChars(usize),
    /// Truncate to max words.
    TruncateWords(usize),
    /// Convert to lowercase.
    Lowercase,
    /// Convert to uppercase.
    Uppercase,
    /// Remove lines matching a pattern.
    RemoveMatchingLines(String),
    /// Add a prefix.
    Prefix(String),
    /// Add a suffix.
    Suffix(String),
}

/// Applies a chain of filter rules to output text.
#[derive(Default)]
pub struct OutputFilter {
    rules: Vec<FilterRule>,
}

impl OutputFilter {
    pub fn new() -> Self { Self::default() }

    pub fn add_rule(&mut self, rule: FilterRule) {
        self.rules.push(rule);
    }

    /// Apply all rules in order, return the final string.
    pub fn apply(&self, text: &str) -> String {
        let mut s = text.to_string();
        for rule in &self.rules {
            s = apply_rule(s, rule);
        }
        s
    }

    pub fn rule_count(&self) -> usize { self.rules.len() }
}

fn apply_rule(text: String, rule: &FilterRule) -> String {
    match rule {
        FilterRule::TrimWhitespace => text.trim().to_string(),
        FilterRule::RemovePattern(pat) => {
            if let Ok(re) = Regex::new(pat) {
                re.replace_all(&text, "").to_string()
            } else {
                text
            }
        }
        FilterRule::ReplacePattern { pattern, replacement } => {
            if let Ok(re) = Regex::new(pattern) {
                re.replace_all(&text, replacement.as_str()).to_string()
            } else {
                text
            }
        }
        FilterRule::TruncateChars(n) => text.chars().take(*n).collect(),
        FilterRule::TruncateWords(n) => {
            text.split_whitespace().take(*n).collect::<Vec<_>>().join(" ")
        }
        FilterRule::Lowercase => text.to_lowercase(),
        FilterRule::Uppercase => text.to_uppercase(),
        FilterRule::RemoveMatchingLines(pat) => {
            if let Ok(re) = Regex::new(pat) {
                text.lines()
                    .filter(|line| !re.is_match(line))
                    .collect::<Vec<_>>()
                    .join("\n")
            } else {
                text
            }
        }
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
    fn remove_pattern() {
        let mut f = OutputFilter::new();
        f.add_rule(FilterRule::RemovePattern(r"\d+".to_string()));
        assert_eq!(f.apply("abc123def"), "abcdef");
    }

    #[test]
    fn replace_pattern() {
        let mut f = OutputFilter::new();
        f.add_rule(FilterRule::ReplacePattern { pattern: r"foo".to_string(), replacement: "bar".to_string() });
        assert_eq!(f.apply("foo baz foo"), "bar baz bar");
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
}
