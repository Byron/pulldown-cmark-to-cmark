[![Crates.io](https://img.shields.io/crates/v/pulldown-cmark-to-cmark)](https://crates.io/crates/pulldown-cmark-to-cmark)
![Rust](https://github.com/Byron/pulldown-cmark-to-cmark/workflows/Rust/badge.svg)

A utility library which translates [`Event`][pdcm-event] back to markdown.
It's the prerequisite for writing markdown filters which can work as
[mdbook-preprocessors][mdbook-prep].

This library takes great pride in supporting **everything that `pulldown-cmark`** supports,
including *tables* and *footnotes* and *codeblocks in codeblocks*,
while assuring *quality* with a powerful test suite.

[pdcm-event]: https://docs.rs/pulldown-cmark/latest/pulldown_cmark/enum.Event.html
[mdbook-prep]: https://rust-lang.github.io/mdBook/for_developers/preprocessors.html

### How to use

Please have a look at the [`stupicat`-example][sc-example] for a complete tour
of the API, or have a look at the [api-docs][api].

It's easiest to get this library into your `Cargo.toml` using `cargo-add`:
```
cargo add pulldown-cmark-to-cmark
```

[sc-example]: examples/stupicat.rs
[api]: https://docs.rs/crate/pulldown-cmark-to-cmark

### Migrating to the owning State API

`State` now owns its configuration and buffered events. It has no lifetime
parameter or public fields. The one-shot `cmark*` helpers remain available and
return a finished `State`.

For incremental output, create one state and call its methods with the same
logical `std::fmt::Write` output stream:

```rust
use pulldown_cmark::Parser;
use pulldown_cmark_to_cmark::{Options, State};

let events: Vec<_> = Parser::new("a *b*").collect();
let mut state = State::new(Options::default());
let mut output = String::new();

let first = state.process(&events[..2], &mut output).unwrap();
assert_eq!(first.events_consumed, 2);
state.process(&events[2..], &mut output).unwrap();
state.finish(&mut output).unwrap();
assert_eq!(output, "a *b*");
```

| Previous API | Replacement |
| --- | --- |
| `cmark_resume(events, writer, state)` | `State::default()`, then `state.process(events, writer)` |
| `cmark_resume_with_options(...)` | `State::new(options)`, then `state.process(events, writer)` |
| `cmark_resume_with_source_range*` | `state.process_with_source_range(events_and_ranges, source, writer)` |
| `state.finalize(writer)` | `state.finish(writer)` |
| `State<'a>` and its public fields and helper types | Owned, opaque `State`; choose formatting through `Options` when constructing it |

`process`, `process_with_source_range`, and `finish` return
`Result<Progress, Error>`. On success, `events_consumed` counts the events
accepted by that call and `bytes_written` counts the UTF-8 bytes written by
that call. Processing consumes all supplied events, but may write no output
yet. Finishing consumes zero events, flushes the last block and reference
definitions, and writes nothing on subsequent successful calls.

The state retains one open or completed top-level block, plus reference
definitions. A list can need its final item's events to determine tightness,
and its next sibling to keep indented code outside the list. Input strings and
borrowed option strings can be dropped after their call returns.

Source ranges are optional byte ranges, as before. They must be in bounds and
on UTF-8 boundaries. Source spelling is checked against the supplied events
after rendering the block; edited or unsafe spelling uses ordinary escaping.
If a filter changes a shortcut label while retaining its reference ID, the
serializer can emit an explicit reference to preserve the edit and target.

Unclosed or mismatched containers are errors. Input after `finish` is rejected.
After any error, discard the state: a failed writer may already have written
part of its input, so retrying cannot safely resume serialization.

Formatting can change where syntax needs more context: adjacent lists use
different markers, multiline headings use setext syntax, and fences are sized
automatically. Newline accounting also removes redundant blank lines and
trailing container padding.

### CommonMark conformance

The suite uses all **652 examples from [CommonMark 0.31.2][commonmark-spec]**.
With default formatting, every example preserves the merged parser event
sequence, including paragraph wrappers, link types, and code block kinds.
The tests cover ordinary output, source ranges, every event boundary, and
event-by-event input. Fixture extraction converts the spec's `→` notation to
tabs and retains final newlines.

[commonmark-spec]: https://spec.commonmark.org/0.31.2/

### Supported Rust Versions

`pulldown-cmark-to-cmark` follows the MSRV (minimum supported rust version) policy of [`pulldown-cmark`]. The current MSRV is 1.71.1.

[`pulldown-cmark`]: https://github.com/pulldown-cmark/pulldown-cmark

### Friends of this project

 * [**termbook**](https://github.com/Byron/termbook)
   * A runner for `mdbooks` to keep your documentation tested.  
 * [**Share Secrets Safely**](https://github.com/Byron/share-secrets-safely)
   * share secrets within teams to avoid plain-text secrets from day one 

### Maintenance Guide

#### Making a new release

 * **Assure all documentation is up-to-date and tests are green**
 * update the `version` in `Cargo.toml` and `git commit`
 * run `cargo release --no-dev-version`
