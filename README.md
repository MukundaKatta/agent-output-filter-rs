# agent-output-filter

A small, dependency-light Rust library for filtering and transforming LLM agent
output before it is delivered to a user or downstream system.

It lets you build an ordered chain of rules (redaction, truncation, regex
replacement, case normalization, line filtering, and more) and apply them to any
text in a single pass.

## Features

- Composable rule chain applied in insertion order via `OutputFilter`.
- Built-in `FilterRule` variants:
  - `TrimWhitespace` — strip leading/trailing whitespace.
  - `RemovePattern(String)` — delete every match of a regex.
  - `ReplacePattern { pattern, replacement }` — regex search-and-replace.
  - `TruncateChars(usize)` — keep at most N characters (Unicode-aware).
  - `TruncateWords(usize)` — keep at most N whitespace-separated words.
  - `Lowercase` / `Uppercase` — case normalization.
  - `RemoveMatchingLines(String)` — drop lines matching a regex.
  - `Prefix(String)` / `Suffix(String)` — wrap the output.
- Regex-based rules fail safe: an invalid pattern leaves the text unchanged
  rather than panicking.

## Installation

Add the crate to your `Cargo.toml`:

```toml
[dependencies]
agent-output-filter = { git = "https://github.com/MukundaKatta/agent-output-filter-rs" }
```

## Usage

```rust
use agent_output_filter::{OutputFilter, FilterRule};

let mut f = OutputFilter::new();
f.add_rule(FilterRule::TrimWhitespace);
f.add_rule(FilterRule::RemovePattern(r"\[REDACTED\]".to_string()));

let result = f.apply("  Hello [REDACTED] world  ");
assert_eq!(result, "Hello  world");
```

Rules are applied in the order they are added, so you can express
multi-stage pipelines:

```rust
use agent_output_filter::{OutputFilter, FilterRule};

let mut f = OutputFilter::new();
f.add_rule(FilterRule::TrimWhitespace);
f.add_rule(FilterRule::Uppercase);

assert_eq!(f.apply("  hello  "), "HELLO");
assert_eq!(f.rule_count(), 2);
```

## Building and testing

```sh
cargo build
cargo test
```

## Tech stack

- Rust (edition 2021)
- [`regex`](https://crates.io/crates/regex) for pattern-based rules

## License

Licensed under the MIT License.
