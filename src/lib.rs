#![deny(rust_2018_idioms)]

use std::{
    borrow::{Borrow, Cow},
    collections::HashSet,
    fmt,
    iter::FromIterator,
};

use pulldown_cmark::{Alignment as TableAlignment, Event, HeadingLevel, LinkType};

/// Similar to [Pulldown-Cmark-Alignment][Alignment], but with required
/// traits for comparison to allow testing.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Alignment {
    None,
    Left,
    Center,
    Right,
}

impl<'a> From<&'a TableAlignment> for Alignment {
    fn from(s: &'a TableAlignment) -> Self {
        match *s {
            TableAlignment::None => Alignment::None,
            TableAlignment::Left => Alignment::Left,
            TableAlignment::Center => Alignment::Center,
            TableAlignment::Right => Alignment::Right,
        }
    }
}

/// The state of the [`cmark_resume()`] and [`cmark_resume_with_options()`] functions.
/// This does not only allow introspection, but enables the user
/// to halt the serialization at any time, and resume it later.
#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct State<'a> {
    /// The amount of newlines to insert after `Event::Start(...)`
    pub newlines_before_start: usize,
    /// The lists and their types for which we have seen a `Event::Start(List(...))` tag
    pub list_stack: Vec<Option<u64>>,
    /// The computed padding and prefix to print after each newline.
    /// This changes with the level of `BlockQuote` and `List` events.
    pub padding: Vec<Cow<'a, str>>,
    /// Keeps the current table alignments, if we are currently serializing a table.
    pub table_alignments: Vec<Alignment>,
    /// Keeps the current table headers, if we are currently serializing a table.
    pub table_headers: Vec<String>,
    /// The last seen text when serializing a header
    pub text_for_header: Option<String>,
    /// Is set while we are handling text in a code block
    pub is_in_code_block: bool,
    /// True if the last event was html. Used to inject additional newlines to support markdown inside of HTML tags.
    pub last_was_html: bool,
    /// True if the last event was text and the text does not have trailing newline. Used to inject additional newlines before code block end fence.
    pub last_was_text_without_trailing_newline: bool,

    /// Keeps track of the last seen shortcut/link
    pub current_shortcut_text: Option<String>,
    /// A list of shortcuts seen so far for later emission
    pub shortcuts: Vec<(String, String, String)>,
}

/// Configuration for the [`cmark_with_options()`] and [`cmark_resume_with_options()`] functions.
/// The defaults should provide decent spacing and most importantly, will
/// provide a faithful rendering of your markdown document particularly when
/// rendering it to HTML.
///
/// It's best used with its `Options::default()` implementation.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Options<'a> {
    pub newlines_after_headline: usize,
    pub newlines_after_paragraph: usize,
    pub newlines_after_codeblock: usize,
    pub newlines_after_table: usize,
    pub newlines_after_rule: usize,
    pub newlines_after_list: usize,
    pub newlines_after_blockquote: usize,
    pub newlines_after_rest: usize,
    pub code_block_token_count: usize,
    pub code_block_token: char,
    pub list_token: char,
    pub emphasis_token: char,
    pub strong_token: &'a str,
}

const DEFAULT_OPTIONS: Options<'_> = Options {
    newlines_after_headline: 2,
    newlines_after_paragraph: 2,
    newlines_after_codeblock: 2,
    newlines_after_table: 2,
    newlines_after_rule: 2,
    newlines_after_list: 2,
    newlines_after_blockquote: 2,
    newlines_after_rest: 1,
    code_block_token_count: 4,
    code_block_token: '`',
    list_token: '*',
    emphasis_token: '*',
    strong_token: "**",
};

impl<'a> Default for Options<'a> {
    fn default() -> Self {
        DEFAULT_OPTIONS
    }
}

impl<'a> Options<'a> {
    pub fn special_characters(&self) -> Cow<'static, str> {
        // These always need to be escaped, even if reconfigured.
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

/// Serialize a stream of [pulldown-cmark-Events][Event] into a string-backed buffer.
///
/// 1. **events**
///   * An iterator over [`Events`][Event], for example as returned by the [`Parser`][pulldown_cmark::Parser]
/// 1. **formatter**
///   * A format writer, can be a `String`.
/// 1. **state**
///   * The optional initial state of the serialization.
/// 1. **options**
///   * Customize the appearance of the serialization. All otherwise magic values are contained
///     here.
///
/// *Returns* the [`State`] of the serialization on success. You can use it as initial state in the
/// next call if you are halting event serialization.
/// *Errors* are only happening if the underlying buffer fails, which is unlikely.
pub fn cmark_resume_with_options<'a, I, E, F>(
    events: I,
    mut formatter: F,
    state: Option<State<'static>>,
    options: Options<'_>,
) -> Result<State<'static>, fmt::Error>
where
    I: Iterator<Item = E>,
    E: Borrow<Event<'a>>,
    F: fmt::Write,
{
    let mut state = state.unwrap_or_default();
    fn padding<'a, F>(f: &mut F, p: &[Cow<'a, str>]) -> fmt::Result
    where
        F: fmt::Write,
    {
        for padding in p {
            write!(f, "{}", padding)?;
        }
        Ok(())
    }
    fn consume_newlines<F>(f: &mut F, s: &mut State<'_>) -> fmt::Result
    where
        F: fmt::Write,
    {
        while s.newlines_before_start != 0 {
            s.newlines_before_start -= 1;
            f.write_char('\n')?;
            padding(f, &s.padding)?;
        }
        Ok(())
    }

    fn escape_leading_special_characters<'a>(
        t: &'a str,
        is_in_block_quote: bool,
        options: &Options<'a>,
    ) -> Cow<'a, str> {
        if is_in_block_quote || t.is_empty() {
            return Cow::Borrowed(t);
        }

        let first = t.chars().next().expect("at least one char");
        if options.special_characters().contains(first) {
            let mut s = String::with_capacity(t.len() + 1);
            s.push('\\');
            s.push(first);
            s.push_str(&t[1..]);
            Cow::Owned(s)
        } else {
            Cow::Borrowed(t)
        }
    }

    fn print_text_without_trailing_newline<'a, F>(t: &str, f: &mut F, p: &[Cow<'a, str>]) -> fmt::Result
    where
        F: fmt::Write,
    {
        if t.contains('\n') {
            let line_count = t.split('\n').count();
            for (tid, token) in t.split('\n').enumerate() {
                f.write_str(token).and(if tid + 1 == line_count {
                    Ok(())
                } else {
                    f.write_char('\n').and(padding(f, p))
                })?;
            }
            Ok(())
        } else {
            f.write_str(t)
        }
    }

    fn padding_of(l: Option<u64>) -> Cow<'static, str> {
        match l {
            None => "  ".into(),
            Some(n) => format!("{}. ", n).chars().map(|_| ' ').collect::<String>().into(),
        }
    }

    for event in events {
        use pulldown_cmark::{CodeBlockKind, Event::*, Tag::*};

        let event = event.borrow();

        // Markdown allows for HTML elements, into which further markdown formatting is nested.
        // However only if the HTML element is spaced by an additional newline.
        //
        // Relevant spec: https://spec.commonmark.org/0.28/#html-blocks
        if state.last_was_html {
            match event {
                Html(_) => { /* no newlines if HTML continues */ }
                Text(_) => { /* no newlines for inline HTML */ }
                End(_) => { /* no newlines if ending a previous opened tag */ }
                SoftBreak => { /* SoftBreak will result in a newline later */ }
                _ => {
                    // Ensure next Markdown block is rendered properly
                    // by adding a newline after an HTML element.
                    formatter.write_char('\n')?;
                }
            }
        }

        state.last_was_html = false;
        let last_was_text_without_trailing_newline = state.last_was_text_without_trailing_newline;
        state.last_was_text_without_trailing_newline = false;
        match *event {
            Rule => {
                consume_newlines(&mut formatter, &mut state)?;
                if state.newlines_before_start < options.newlines_after_rule {
                    state.newlines_before_start = options.newlines_after_rule;
                }
                formatter.write_str("---")
            }
            Code(ref text) => {
                if let Some(shortcut_text) = state.current_shortcut_text.as_mut() {
                    shortcut_text.push_str(&format!("`{}`", text));
                }
                if let Some(text_for_header) = state.text_for_header.as_mut() {
                    let code = format!("{}{}{}", options.code_block_token, text, options.code_block_token);
                    text_for_header.push_str(&code);
                }
                let (start, end) = if text.contains(options.code_block_token) {
                    (
                        String::from_iter([options.code_block_token, options.code_block_token, ' ']),
                        String::from_iter([' ', options.code_block_token, options.code_block_token]),
                    )
                } else {
                    (
                        String::from(options.code_block_token),
                        String::from(options.code_block_token),
                    )
                };
                formatter
                    .write_str(&start)
                    .and_then(|_| formatter.write_str(text))
                    .and_then(|_| formatter.write_str(&end))
            }
            Start(ref tag) => {
                if let List(ref list_type) = *tag {
                    state.list_stack.push(*list_type);
                    if state.list_stack.len() > 1 && state.newlines_before_start < options.newlines_after_rest {
                        state.newlines_before_start = options.newlines_after_rest;
                    }
                }
                let consumed_newlines = state.newlines_before_start != 0;
                consume_newlines(&mut formatter, &mut state)?;
                match tag {
                    Item => match state.list_stack.last() {
                        Some(inner) => {
                            state.padding.push(padding_of(*inner));
                            match inner {
                                Some(n) => write!(formatter, "{}. ", n),
                                None => write!(formatter, "{} ", options.list_token),
                            }
                        }
                        None => Ok(()),
                    },
                    Table(ref alignments) => {
                        state.table_alignments = alignments.iter().map(From::from).collect();
                        Ok(())
                    }
                    TableHead => Ok(()),
                    TableRow => Ok(()),
                    TableCell => {
                        state.text_for_header = Some(String::new());
                        formatter.write_char('|')
                    }
                    Link(LinkType::Autolink | LinkType::Email, ..) => formatter.write_char('<'),
                    Link(LinkType::Shortcut, ..) => {
                        state.current_shortcut_text = Some(String::new());
                        formatter.write_char('[')
                    }
                    Link(..) => formatter.write_char('['),
                    Image(..) => formatter.write_str("!["),
                    Emphasis => formatter.write_char(options.emphasis_token),
                    Strong => formatter.write_str(options.strong_token),
                    FootnoteDefinition(ref name) => write!(formatter, "[^{}]: ", name),
                    Paragraph => Ok(()),
                    Heading(level, _, _) => {
                        match level {
                            HeadingLevel::H1 => formatter.write_str("#"),
                            HeadingLevel::H2 => formatter.write_str("##"),
                            HeadingLevel::H3 => formatter.write_str("###"),
                            HeadingLevel::H4 => formatter.write_str("####"),
                            HeadingLevel::H5 => formatter.write_str("#####"),
                            HeadingLevel::H6 => formatter.write_str("######"),
                        }?;
                        formatter.write_char(' ')
                    }
                    BlockQuote => {
                        state.padding.push(" > ".into());
                        state.newlines_before_start = 1;

                        // if we consumed some newlines, we know that we can just write out the next
                        // level in our blockquote. This should work regardless if we have other
                        // padding or if we're in a list
                        if consumed_newlines {
                            formatter.write_str(" > ")
                        } else {
                            formatter.write_char('\n').and(padding(&mut formatter, &state.padding))
                        }
                    }
                    CodeBlock(CodeBlockKind::Indented) => {
                        state.is_in_code_block = true;
                        for _ in 0..options.code_block_token_count {
                            formatter.write_char(options.code_block_token)?;
                        }
                        formatter.write_char('\n').and(padding(&mut formatter, &state.padding))
                    }
                    CodeBlock(CodeBlockKind::Fenced(ref info)) => {
                        state.is_in_code_block = true;
                        let s = if !consumed_newlines {
                            formatter
                                .write_char('\n')
                                .and_then(|_| padding(&mut formatter, &state.padding))
                        } else {
                            Ok(())
                        };

                        s.and_then(|_| {
                            for _ in 0..options.code_block_token_count {
                                formatter.write_char(options.code_block_token)?;
                            }
                            Ok(())
                        })
                        .and_then(|_| formatter.write_str(info))
                        .and_then(|_| formatter.write_char('\n'))
                        .and_then(|_| padding(&mut formatter, &state.padding))
                    }
                    List(_) => Ok(()),
                    Strikethrough => formatter.write_str("~~"),
                }
            }
            End(ref tag) => match tag {
                Link(LinkType::Autolink | LinkType::Email, ..) => formatter.write_char('>'),
                Link(LinkType::Shortcut, ref uri, ref title) => {
                    if let Some(shortcut_text) = state.current_shortcut_text.take() {
                        state
                            .shortcuts
                            .push((shortcut_text, uri.to_string(), title.to_string()));
                    }
                    formatter.write_char(']')
                }
                Image(_, ref uri, ref title) | Link(_, ref uri, ref title) => {
                    close_link(uri, title, &mut formatter, LinkType::Inline)
                }
                Emphasis => formatter.write_char(options.emphasis_token),
                Strong => formatter.write_str(options.strong_token),
                Heading(_, id, classes) => {
                    let emit_braces = id.is_some() || !classes.is_empty();
                    if emit_braces {
                        formatter.write_str(" {")?;
                    }
                    if let Some(id_str) = id {
                        formatter.write_char('#')?;
                        formatter.write_str(id_str)?;
                        if !classes.is_empty() {
                            formatter.write_char(' ')?;
                        }
                    }
                    for (idx, class) in classes.iter().enumerate() {
                        formatter.write_char('.')?;
                        formatter.write_str(class)?;
                        if idx < classes.len() - 1 {
                            formatter.write_char(' ')?;
                        }
                    }
                    if emit_braces {
                        formatter.write_char('}')?;
                    }
                    if state.newlines_before_start < options.newlines_after_headline {
                        state.newlines_before_start = options.newlines_after_headline;
                    }
                    Ok(())
                }
                Paragraph => {
                    if state.newlines_before_start < options.newlines_after_paragraph {
                        state.newlines_before_start = options.newlines_after_paragraph;
                    }
                    Ok(())
                }
                CodeBlock(_) => {
                    if state.newlines_before_start < options.newlines_after_codeblock {
                        state.newlines_before_start = options.newlines_after_codeblock;
                    }
                    state.is_in_code_block = false;
                    if last_was_text_without_trailing_newline {
                        formatter.write_char('\n')?;
                    }
                    for _ in 0..options.code_block_token_count {
                        formatter.write_char(options.code_block_token)?;
                    }
                    Ok(())
                }
                Table(_) => {
                    if state.newlines_before_start < options.newlines_after_table {
                        state.newlines_before_start = options.newlines_after_table;
                    }
                    state.table_alignments.clear();
                    state.table_headers.clear();
                    Ok(())
                }
                TableCell => {
                    state.table_headers.push(
                        state
                            .text_for_header
                            .take()
                            .filter(|s| !s.is_empty())
                            .unwrap_or_else(|| "  ".into()),
                    );
                    Ok(())
                }
                ref t @ TableRow | ref t @ TableHead => {
                    if state.newlines_before_start < options.newlines_after_rest {
                        state.newlines_before_start = options.newlines_after_rest;
                    }
                    formatter.write_char('|')?;

                    if let TableHead = t {
                        formatter
                            .write_char('\n')
                            .and(padding(&mut formatter, &state.padding))?;
                        for (alignment, name) in state.table_alignments.iter().zip(state.table_headers.iter()) {
                            formatter.write_char('|')?;
                            // NOTE: For perfect counting, count grapheme clusters.
                            // The reason this is not done is to avoid the dependency.
                            let last_minus_one = name.chars().count().saturating_sub(1);
                            for c in 0..name.len() {
                                formatter.write_char(
                                    if (c == 0 && (alignment == &Alignment::Center || alignment == &Alignment::Left))
                                        || (c == last_minus_one
                                            && (alignment == &Alignment::Center || alignment == &Alignment::Right))
                                    {
                                        ':'
                                    } else {
                                        '-'
                                    },
                                )?;
                            }
                        }
                        formatter.write_char('|')?;
                    }
                    Ok(())
                }
                Item => {
                    state.padding.pop();
                    if state.newlines_before_start < options.newlines_after_rest {
                        state.newlines_before_start = options.newlines_after_rest;
                    }
                    Ok(())
                }
                List(_) => {
                    state.list_stack.pop();
                    if state.list_stack.is_empty() && state.newlines_before_start < options.newlines_after_list {
                        state.newlines_before_start = options.newlines_after_list;
                    }
                    Ok(())
                }
                BlockQuote => {
                    state.padding.pop();

                    if state.newlines_before_start < options.newlines_after_blockquote {
                        state.newlines_before_start = options.newlines_after_blockquote;
                    }

                    Ok(())
                }
                FootnoteDefinition(_) => Ok(()),
                Strikethrough => formatter.write_str("~~"),
            },
            HardBreak => formatter.write_str("  \n").and(padding(&mut formatter, &state.padding)),
            SoftBreak => formatter.write_char('\n').and(padding(&mut formatter, &state.padding)),
            Text(ref text) => {
                if let Some(shortcut_text) = state.current_shortcut_text.as_mut() {
                    shortcut_text.push_str(text);
                }
                if let Some(text_for_header) = state.text_for_header.as_mut() {
                    text_for_header.push_str(text)
                }
                consume_newlines(&mut formatter, &mut state)?;
                state.last_was_text_without_trailing_newline = !text.ends_with('\n');
                print_text_without_trailing_newline(
                    &escape_leading_special_characters(text, state.is_in_code_block, &options),
                    &mut formatter,
                    &state.padding,
                )
            }
            Html(ref text) => {
                state.last_was_html = true;
                consume_newlines(&mut formatter, &mut state)?;
                print_text_without_trailing_newline(text, &mut formatter, &state.padding)
            }
            FootnoteReference(ref name) => write!(formatter, "[^{}]", name),
            TaskListMarker(checked) => {
                let check = if checked { "x" } else { " " };
                write!(formatter, "[{}] ", check)
            }
        }?
    }
    Ok(state)
}

/// As [`cmark_resume_with_options()`], but with default [`Options`].
pub fn cmark_resume<'a, I, E, F>(
    events: I,
    formatter: F,
    state: Option<State<'static>>,
) -> Result<State<'static>, fmt::Error>
where
    I: Iterator<Item = E>,
    E: Borrow<Event<'a>>,
    F: fmt::Write,
{
    cmark_resume_with_options(events, formatter, state, Options::default())
}

fn close_link<F>(uri: &str, title: &str, f: &mut F, link_type: LinkType) -> fmt::Result
where
    F: fmt::Write,
{
    let separator = match link_type {
        LinkType::Shortcut => ": ",
        _ => "(",
    };

    if uri.contains(' ') {
        write!(f, "]{}<{uri}>", separator, uri = uri)?;
    } else {
        write!(f, "]{}{uri}", separator, uri = uri)?;
    }
    if !title.is_empty() {
        write!(f, " \"{title}\"", title = title)?;
    }
    if link_type != LinkType::Shortcut {
        f.write_char(')')?;
    }

    Ok(())
}

impl<'a> State<'a> {
    pub fn finalize<F>(mut self, mut formatter: F) -> Result<Self, fmt::Error>
    where
        F: fmt::Write,
    {
        if self.shortcuts.is_empty() {
            return Ok(self);
        }

        formatter.write_str("\n")?;
        let mut written_shortcuts = HashSet::new();
        for shortcut in self.shortcuts.drain(..) {
            if written_shortcuts.contains(&shortcut) {
                continue;
            }
            write!(formatter, "\n[{}", shortcut.0)?;
            close_link(&shortcut.1, &shortcut.2, &mut formatter, LinkType::Shortcut)?;
            written_shortcuts.insert(shortcut);
        }
        Ok(self)
    }
}

/// As [`cmark_resume_with_options()`], but with the [`State`] finalized.
pub fn cmark_with_options<'a, I, E, F>(
    events: I,
    mut formatter: F,
    options: Options<'_>,
) -> Result<State<'static>, fmt::Error>
where
    I: Iterator<Item = E>,
    E: Borrow<Event<'a>>,
    F: fmt::Write,
{
    let state = cmark_resume_with_options(events, &mut formatter, Default::default(), options)?;
    state.finalize(formatter)
}

/// As [`cmark_with_options()`], but with default [`Options`].
pub fn cmark<'a, I, E, F>(events: I, mut formatter: F) -> Result<State<'static>, fmt::Error>
where
    I: Iterator<Item = E>,
    E: Borrow<Event<'a>>,
    F: fmt::Write,
{
    cmark_with_options(events, &mut formatter, Default::default())
}
