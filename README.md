[![Crates.io](https://img.shields.io/crates/v/pulldown-cmark-to-cmark)](https://crates.io/crates/pulldown-cmark-to-cmark)
![Rust](https://github.com/Byron/pulldown-cmark-to-cmark/workflows/Rust/badge.svg)

A utility library which translates [`Event`][pdcm-event] back to markdown.
It's the prerequisite for writing markdown filters which can work as
[mdbook-preprocessors][mdbook-prep].

Supports CommonMark and modern `pulldown-cmark` extensions, including tables,
footnotes, definition lists, math, and nested blocks. Use the same parser flags
for parsing and serialization, and validate formatting preferences to select
a supported configuration.

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
return a finished `State`. Parser flags are now mandatory in every constructor
and one-shot helper; they determine escaping and whether candidate output
reparses correctly. `State::default()` is no longer available.

The one-shot signatures are `cmark(events, writer, parser_options)` and
`cmark_with_options(events, writer, parser_options, options)`. For source ranges,
use `cmark_with_source_range(events_and_ranges, source, writer, parser_options)`
or append `options` for `cmark_with_source_range_and_options`. Pass
`pulldown_cmark::Options::empty()` for `Parser::new` input.

For incremental output, create one state and call its methods with the same
logical `std::fmt::Write` output stream:

```rust
use pulldown_cmark::Parser;
use pulldown_cmark_to_cmark::{Options, State};

let parser_options = pulldown_cmark::Options::empty();
let options = Options::default();
options.validate(parser_options).unwrap();
let events: Vec<_> = Parser::new_ext("a *b*", parser_options).collect();
let mut state = State::new(parser_options, options);
let mut output = String::new();

let first = state.process(&events[..2], &mut output).unwrap();
assert_eq!(first.events_consumed, 2);
state.process(&events[2..], &mut output).unwrap();
state.finish(&mut output).unwrap();
assert_eq!(output, "a *b*");
```

| Previous API | Replacement |
| --- | --- |
| `cmark_resume(events, writer, state)` | `State::new(parser_options, Options::default())`, then `state.process(events, writer)` |
| `cmark_resume_with_options(...)` | `State::new(parser_options, options)`, then `state.process(events, writer)` |
| `cmark_resume_with_source_range*` | `state.process_with_source_range(events_and_ranges, source, writer)` |
| `state.finalize(writer)` | `state.finish(writer)` |
| `State<'a>` and its public fields and helper types | Owned, opaque `State`; choose formatting through `Options` when constructing it |

`process`, `process_with_source_range`, and `finish` return
`Result<Progress, Error>`. On success, `events_consumed` counts the events
accepted by that call and `bytes_written` counts the UTF-8 bytes written by
that call. Processing consumes all supplied events, but may write no output
yet. Finishing consumes zero events, flushes the last block and reference
definitions, and writes nothing on subsequent successful calls.

The state normally retains one open or completed top-level block, plus reference
labels and definitions. A list can need its final item's events to determine
tightness, and its next sibling to keep indented code outside the list.
Some lists owe their structure to reference definitions that produce no events:
for example, a loose single-paragraph item or an empty nested item interrupting
text. These require a fresh, invisible definition in the output. From the first
such list, the state buffers the remaining document until `finish`, so it can
choose a label without changing other references. Input strings and borrowed
option strings can be dropped after their call returns.

Source ranges are optional byte ranges, as before. They must be in bounds and
on UTF-8 boundaries. Source spelling is checked against the supplied events
after rendering the block; edited or unsafe spelling uses ordinary escaping.
If a filter changes a shortcut label while retaining its reference ID, the
serializer can emit an explicit reference to preserve the edit and target.

Unclosed or mismatched containers are errors. A delimiter block that cannot be
represented faithfully returns `Error::Unrepresentable` instead of silently
changing its events. Input after `finish` is rejected.
After any error, discard the state: a failed writer may already have written
part of its input, so retrying cannot safely resume serialization.

Formatting can change where syntax needs more context: adjacent lists use
different markers, multiline headings use setext syntax, and fences are sized
automatically. Newline accounting also removes redundant blank lines and
trailing container padding.

### Validating options

Call `Options::validate(parser_options)` before creating a state or using a
one-shot helper. Validation is recommended and optional; applications can still
use unvalidated preferences when deliberately changing Markdown semantics.
It checks configuration, not whether arbitrary edited events are representable.

`SUPPORTED_PARSER_OPTIONS` contains twelve independently supported flags:
tables, modern footnotes, strikethrough, task lists, heading attributes, YAML
metadata, TOML metadata, math, GFM alerts, definition lists, superscript, and
subscript. Every subset is accepted. Smart punctuation, legacy footnotes,
wikilinks, and unknown flags are rejected. Their transformations or syntax
are outside the supported round-trip contract.

Validated marker choices are `*`, `-`, or `+` for bullets; `.` or `)` for ordered
lists; backticks or tildes for fences; `*` or `_` for emphasis; and `**` or `__`
for strong emphasis. Spacing and fence lengths are preferences, adjusted to
preserve syntax. Values that cannot fit a Rust string are rejected. Incremented
ordered markers stop at Markdown's nine-digit maximum.

`use_html_for_super_sub_script` now defaults to `false`. Enabling it while the
parser recognizes symbolic superscripts or subscripts is rejected: HTML would
produce different event types. Existing HTML events are preserved either way.

The finite configuration matrix has 32,768 known parser bit patterns and 192
marker/boolean combinations. **491,520 configurations are accepted; 5,799,936
are rejected** (5,505,024 for unsupported parser flags and 294,912 for the
HTML/script conflict). Numeric preferences are checked separately at boundary
and representative values; they are not part of this finite count.

### CommonMark conformance

The suite uses all **652 examples from [CommonMark 0.31.2][commonmark-spec]**.
With default formatting, every example preserves the merged parser event
sequence, including paragraph wrappers, link types, and code block kinds.
The tests cover ordinary output, source ranges, every event boundary, and
event-by-event input. Fixture extraction converts the spec's `→` notation to
tabs and retains final newlines.

Run the spec with visible totals:

```sh
cargo test --test integrate spec::commonmark_spec -- --nocapture
```

The differential suite parses each input, serializes it, and reparses the output
with the same flags. It compares exact events after merging adjacent text only;
paragraph wrappers, nesting, link types, and code block kinds must survive.
Both ordinary and source-range output are checked. The suite retains 860 review
regressions and generates 91,446 distinct inputs from delimiter runs, Unicode,
block boundaries, and a fixed random seed. Additional checks cover the repository
fixtures and 72,030 combinations of symbolic extension delimiters.

Run configuration counts, numeric and incremental checks, all 652 spec examples
across 192 formatting combinations, every accepted configuration's interaction
probes, and the generated corpus under six useful parser profiles:

```sh
cargo test --release --test differential -- --include-ignored --nocapture
```

Failures print the input, output, flags, preferences, and differing events. Set
`CMARK_DIFFERENTIAL_REPORT=/tmp/cmark-failures.tsv` to append all failing cases.
The checks compare against the original parse, so no baseline executable or
allowed failure count is needed. CI runs this exhaustive suite.

Reference-link benchmarks cover shared and distinct labels, edited source, and
nested emphasis at 1,000–8,000 paragraphs. Each workload verifies its round trip
before timing. To inspect scaling or compare Criterion baselines:

```sh
cargo bench --bench reference_links
```

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
