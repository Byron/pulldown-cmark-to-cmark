//! Convert `pulldown-cmark` `Event`s back to the string they were parsed from.
//!
//! This crate provides functions to serialize markdown events back into markdown text format.
//!
//! # Examples
//!
//! ```rust
//! use pulldown_cmark::Parser;
//! use pulldown_cmark_to_cmark::cmark;
//!
//! let input_markdown = "# Hello\n\nWorld!";
//! let events = Parser::new(input_markdown);
//! let mut output_markdown = String::new();
//! cmark(events, &mut output_markdown, pulldown_cmark::Options::empty()).unwrap();
//! assert_eq!(output_markdown, input_markdown);
//! ```

#![deny(rust_2018_idioms)]
#![deny(missing_docs)]

use std::{
    borrow::{Borrow, Cow},
    fmt,
    ops::Range,
};

use pulldown_cmark::{Event, Options as ParserOptions, Tag, TagEnd};

mod source_range {
    use super::{fmt, Borrow, Error, Event, Options, ParserOptions, Range, State};

    /// Serialize events with source ranges, returning a finished [`State`].
    ///
    /// `parser_options` must match the parser that produced the events.
    /// Calling [`Options::validate`] first is recommended.
    /// See [`State::process_with_source_range`] for source spelling and range handling.
    pub fn cmark_with_source_range_and_options<'a, I, E, F>(
        event_and_ranges: I,
        source: &str,
        mut formatter: F,
        parser_options: ParserOptions,
        options: Options<'_>,
    ) -> Result<State, Error>
    where
        I: IntoIterator<Item = (E, Option<Range<usize>>)>,
        E: Borrow<Event<'a>>,
        F: fmt::Write,
    {
        let mut state = State::new(parser_options, options);
        state.process_with_source_range(event_and_ranges, source, &mut formatter)?;
        state.finish(formatter)?;
        Ok(state)
    }

    /// As [`cmark_with_source_range_and_options`], but with default [`Options`].
    pub fn cmark_with_source_range<'a, I, E, F>(
        event_and_ranges: I,
        source: &str,
        mut formatter: F,
        parser_options: ParserOptions,
    ) -> Result<State, Error>
    where
        I: IntoIterator<Item = (E, Option<Range<usize>>)>,
        E: Borrow<Event<'a>>,
        F: fmt::Write,
    {
        cmark_with_source_range_and_options(
            event_and_ranges,
            source,
            &mut formatter,
            parser_options,
            Default::default(),
        )
    }
}
pub use source_range::{cmark_with_source_range, cmark_with_source_range_and_options};

mod state;
pub use state::{Progress, State};
mod render;
use render::Renderer;
mod text_modifications;

/// The minimum number of tokens in a fenced code block.
pub const DEFAULT_CODE_BLOCK_TOKEN_COUNT: usize = 3;

/// Parser extensions supported by validated formatting configurations.
///
/// Every subset is supported. Smart punctuation, old footnotes, wikilinks, and
/// unknown flags are rejected by [`Options::validate`]. Use the same flags for
/// parsing the input, serializing it, and parsing the output.
pub const SUPPORTED_PARSER_OPTIONS: ParserOptions = ParserOptions::from_bits_retain(
    ParserOptions::ENABLE_TABLES.bits()
        | ParserOptions::ENABLE_FOOTNOTES.bits()
        | ParserOptions::ENABLE_STRIKETHROUGH.bits()
        | ParserOptions::ENABLE_TASKLISTS.bits()
        | ParserOptions::ENABLE_HEADING_ATTRIBUTES.bits()
        | ParserOptions::ENABLE_YAML_STYLE_METADATA_BLOCKS.bits()
        | ParserOptions::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS.bits()
        | ParserOptions::ENABLE_MATH.bits()
        | ParserOptions::ENABLE_GFM.bits()
        | ParserOptions::ENABLE_DEFINITION_LIST.bits()
        | ParserOptions::ENABLE_SUPERSCRIPT.bits()
        | ParserOptions::ENABLE_SUBSCRIPT.bits(),
);

/// Formatting preferences for [`State`] and [`cmark_with_options()`].
/// The defaults should provide decent spacing and most importantly, will
/// provide a faithful rendering of your markdown document particularly when
/// rendering it to HTML.
///
/// It's best used with its `Options::default()` implementation.
/// Separators and delimiters may be adjusted when needed to preserve Markdown syntax.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Options<'a> {
    /// The number of newlines to insert after a headline
    pub newlines_after_headline: usize,
    /// The number of newlines to insert after a paragraph
    pub newlines_after_paragraph: usize,
    /// The number of newlines to insert after a code block
    pub newlines_after_codeblock: usize,
    /// The number of newlines to insert after an HTML block
    pub newlines_after_htmlblock: usize,
    /// The number of newlines to insert after a table
    pub newlines_after_table: usize,
    /// The number of newlines to insert after a horizontal rule
    pub newlines_after_rule: usize,
    /// The number of newlines to insert after a list
    pub newlines_after_list: usize,
    /// The number of newlines to insert after a block quote
    pub newlines_after_blockquote: usize,
    /// The number of newlines to insert after other elements
    pub newlines_after_rest: usize,
    /// The amount of newlines placed after TOML or YAML metadata blocks at the beginning of a document.
    pub newlines_after_metadata: usize,
    /// Preferred token count for fenced code blocks, defaulting to `4`.
    /// The serializer increases this to at least three and beyond any matching
    /// token run in the block's contents. Precomputing a count with
    /// [`calculate_code_block_token_count()`] is optional.
    pub code_block_token_count: usize,
    /// The character to use for code block fences (backtick or tilde)
    pub code_block_token: char,
    /// The character to use for unordered list items
    pub list_token: char,
    /// The character to use after ordered list numbers (e.g., '.' for `1.`)
    pub ordered_list_token: char,
    /// Whether to increment the number for each ordered list item
    pub increment_ordered_list_bullets: bool,
    /// The character to use for emphasis (italic)
    pub emphasis_token: char,
    /// The string to use for strong emphasis (bold)
    pub strong_token: &'a str,
    /// Use HTML tags for superscript/subscript events instead of `^` and `~`.
    /// Defaults to `false` to preserve the parser's symbolic event types.
    /// [`Self::validate`] rejects HTML output with either symbolic parser extension.
    /// Original HTML events are written unchanged regardless of this preference.
    pub use_html_for_super_sub_script: bool,
}

const DEFAULT_OPTIONS: Options<'_> = Options {
    newlines_after_headline: 2,
    newlines_after_paragraph: 2,
    newlines_after_codeblock: 2,
    newlines_after_htmlblock: 1,
    newlines_after_table: 2,
    newlines_after_rule: 2,
    newlines_after_list: 2,
    newlines_after_blockquote: 2,
    newlines_after_rest: 1,
    newlines_after_metadata: 1,
    code_block_token_count: 4,
    code_block_token: '`',
    list_token: '*',
    ordered_list_token: '.',
    increment_ordered_list_bullets: false,
    emphasis_token: '*',
    strong_token: "**",
    use_html_for_super_sub_script: false,
};

impl Default for Options<'_> {
    fn default() -> Self {
        DEFAULT_OPTIONS
    }
}

impl Options<'_> {
    /// Check that parser flags and formatting preferences support exact event round trips.
    ///
    /// Calling this before creating a [`State`] or calling a `cmark*` helper is
    /// recommended, but optional. Unvalidated configurations remain available
    /// for applications that deliberately change Markdown semantics.
    /// This checks configuration, not whether arbitrary edited events are representable.
    /// Spacing and fence lengths are preferences, clamped to syntactic minima.
    pub fn validate(&self, parser_options: ParserOptions) -> Result<(), OptionsError> {
        let unsupported = parser_options.difference(SUPPORTED_PARSER_OPTIONS);
        if !unsupported.is_empty() {
            return Err(OptionsError::UnsupportedParserOptions(unsupported));
        }
        for (field, valid) in [
            ("code_block_token", matches!(self.code_block_token, '`' | '~')),
            ("list_token", matches!(self.list_token, '*' | '-' | '+')),
            ("ordered_list_token", matches!(self.ordered_list_token, '.' | ')')),
            ("emphasis_token", matches!(self.emphasis_token, '*' | '_')),
            ("strong_token", matches!(self.strong_token, "**" | "__")),
        ] {
            if !valid {
                return Err(OptionsError::InvalidToken(field));
            }
        }
        if self.use_html_for_super_sub_script
            && parser_options.intersects(ParserOptions::ENABLE_SUPERSCRIPT | ParserOptions::ENABLE_SUBSCRIPT)
        {
            return Err(OptionsError::HtmlSuperSubscript);
        }
        for (field, value) in [
            ("newlines_after_headline", self.newlines_after_headline),
            ("newlines_after_paragraph", self.newlines_after_paragraph),
            ("newlines_after_codeblock", self.newlines_after_codeblock),
            ("newlines_after_htmlblock", self.newlines_after_htmlblock),
            ("newlines_after_table", self.newlines_after_table),
            ("newlines_after_rule", self.newlines_after_rule),
            ("newlines_after_list", self.newlines_after_list),
            ("newlines_after_blockquote", self.newlines_after_blockquote),
            ("newlines_after_rest", self.newlines_after_rest),
            ("newlines_after_metadata", self.newlines_after_metadata),
            ("code_block_token_count", self.code_block_token_count),
        ] {
            if value >= isize::MAX as usize {
                return Err(OptionsError::SizeOverflow(field));
            }
        }
        Ok(())
    }

    /// Returns the baseline punctuation considered for backslash escaping.
    /// Context can require escaping other characters or writing character references.
    pub fn special_characters(&self) -> Cow<'static, str> {
        // Reconfiguring delimiter preferences does not remove their Markdown meaning.
        const BASE: &str = "#\\_*<>`|[]";
        if DEFAULT_OPTIONS.code_block_token == self.code_block_token
            && DEFAULT_OPTIONS.list_token == self.list_token
            && DEFAULT_OPTIONS.emphasis_token == self.emphasis_token
            && DEFAULT_OPTIONS.strong_token == self.strong_token
        {
            BASE.into()
        } else {
            let mut s = String::from(BASE);
            s.push(self.code_block_token);
            s.push(self.list_token);
            s.push(self.emphasis_token);
            s.push_str(self.strong_token);
            s.into()
        }
    }
}

/// An unsupported parser/formatting configuration, reported by [`Options::validate`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OptionsError {
    /// Flags outside [`SUPPORTED_PARSER_OPTIONS`], including unknown bits.
    UnsupportedParserOptions(ParserOptions),
    /// The named formatting field uses a nonstandard Markdown marker.
    InvalidToken(&'static str),
    /// HTML output would replace symbolic superscript/subscript events with HTML events.
    HtmlSuperSubscript,
    /// The named field cannot fit its output and required separator in a Rust string.
    SizeOverflow(&'static str),
}

impl fmt::Display for OptionsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedParserOptions(flags) => write!(
                f,
                "unsupported parser flags: {flags:?}; use a subset of SUPPORTED_PARSER_OPTIONS"
            ),
            Self::InvalidToken(field) => write!(f, "{field} must use a standard Markdown delimiter"),
            Self::HtmlSuperSubscript => f.write_str(
                "set use_html_for_super_sub_script to false when parsing symbolic superscripts or subscripts",
            ),
            Self::SizeOverflow(field) => write!(f, "{field} is too large for a Markdown output string"),
        }
    }
}

impl std::error::Error for OptionsError {}

/// An error encountered while serializing Markdown.
#[derive(Debug)]
pub enum Error {
    /// Formatting to the output writer failed
    FormatFailed(fmt::Error),
    /// An event was encountered that cannot be produced by valid markdown
    UnexpectedEvent,
    /// No lossless Markdown spelling was found for the supplied event block.
    Unrepresentable,
    /// A source range is out of bounds or is not on UTF-8 character boundaries.
    InvalidSourceRange,
    /// More input was supplied after finishing the serializer.
    Finished,
    /// An earlier call failed; this serializer can no longer be used.
    Failed,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FormatFailed(e) => e.fmt(f),
            Self::UnexpectedEvent => f.write_str("Unexpected event while reconstructing Markdown"),
            Self::Unrepresentable => f.write_str("Cannot preserve this event block with the supplied parser options"),
            Self::InvalidSourceRange => f.write_str("Invalid Markdown source range"),
            Self::Finished => f.write_str("Markdown serializer has already finished"),
            Self::Failed => f.write_str("Markdown serializer cannot be used after an error"),
        }
    }
}

impl std::error::Error for Error {}

impl From<fmt::Error> for Error {
    fn from(e: fmt::Error) -> Self {
        Self::FormatFailed(e)
    }
}

/// Serialize Markdown with default [`Options`], returning a finished [`State`].
/// `parser_options` must match the parser that produced the events.
pub fn cmark<'a, I, E, F>(events: I, formatter: F, parser_options: ParserOptions) -> Result<State, Error>
where
    I: IntoIterator<Item = E>,
    E: Borrow<Event<'a>>,
    F: fmt::Write,
{
    cmark_with_options(events, formatter, parser_options, Options::default())
}

/// Serialize a stream of Markdown events using `options`.
///
/// `parser_options` must match the parser that produced the events.
/// Calling [`Options::validate`] first is recommended.
/// The returned [`State`] is finished, including any reference definitions.
/// For incremental serialization use [`State::process`] and [`State::finish`].
/// Errors report invalid event streams or a failure of the [`fmt::Write`] writer.
pub fn cmark_with_options<'a, I, E, F>(
    events: I,
    mut formatter: F,
    parser_options: ParserOptions,
    options: Options<'_>,
) -> Result<State, Error>
where
    I: IntoIterator<Item = E>,
    E: Borrow<Event<'a>>,
    F: fmt::Write,
{
    let mut state = State::new(parser_options, options);
    state.process(events, &mut formatter)?;
    state.finish(formatter)?;
    Ok(state)
}

/// Return the `<seen amount of consecutive fenced code-block tokens> + 1` that occur *within* a
/// fenced code-block `events`.
///
/// This can set a preferred `code_block_token_count` in [`Options`]. The serializer
/// also sizes each block's fence automatically, so this calculation is optional.
///
/// Returns `None` if `events` didn't include any code-block, or the code-block didn't contain
/// a nested block. In that case, the correct amount of fenced code-block tokens is
/// [`DEFAULT_CODE_BLOCK_TOKEN_COUNT`].
///
/// ```rust
/// use pulldown_cmark::Event;
/// use pulldown_cmark_to_cmark::*;
///
/// let events = &[Event::Text("text".into())];
/// let code_block_token_count = calculate_code_block_token_count(events).unwrap_or(DEFAULT_CODE_BLOCK_TOKEN_COUNT);
/// let options = Options {
///     code_block_token_count,
///     ..Default::default()
/// };
/// let mut buf = String::new();
/// cmark_with_options(events.iter(), &mut buf, pulldown_cmark::Options::empty(), options)?;
/// # Ok::<(), pulldown_cmark_to_cmark::Error>(())
/// ```
pub fn calculate_code_block_token_count<'a, I, E>(events: I) -> Option<usize>
where
    I: IntoIterator<Item = E>,
    E: Borrow<Event<'a>>,
{
    let mut in_codeblock = false;
    let mut max_token_count = 0;

    // token_count should be taken over Text events
    // because a continuous text may be splitted to some Text events.
    let mut token_count = 0;
    let mut prev_token_char = None;
    for event in events {
        match event.borrow() {
            Event::Start(Tag::CodeBlock(_)) => {
                in_codeblock = true;
            }
            Event::End(TagEnd::CodeBlock) => {
                in_codeblock = false;
                prev_token_char = None;
            }
            Event::Text(x) if in_codeblock => {
                for c in x.chars() {
                    let prev_token = prev_token_char.take();
                    if c == '`' || c == '~' {
                        prev_token_char = Some(c);
                        if Some(c) == prev_token {
                            token_count += 1;
                        } else {
                            max_token_count = max_token_count.max(token_count);
                            token_count = 1;
                        }
                    }
                }
            }
            _ => prev_token_char = None,
        }
    }

    max_token_count = max_token_count.max(token_count);
    (max_token_count >= 3).then_some(max_token_count + 1)
}
