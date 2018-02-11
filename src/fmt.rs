use std::fmt;
use std::borrow::Borrow;
use std::borrow::Cow;
use pulldown_cmark::{Alignment as TableAlignment, Event};

/// Similar to [Pulldown-Cmark-Alignment][pd-alignment], but with required
/// traits for comparison to allow testing.
///
/// [pd-alignment]: https://docs.rs/pulldown-cmark/*/pulldown_cmark/enum.Alignment.html
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

/// The state of the `cmark` function.
/// This does not only allow introspection, but enables the user
/// to halt the serialization at any time, and resume it later.
#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct State<'a> {
    /// The amount of newlines to insert after `Event::Start(...)`
    pub newlines_before_start: usize,
    /// The lists and their types for which we have seen a `Event::Start(List(...))` tag
    pub list_stack: Vec<Option<usize>>,
    /// The computed padding and prefix to print after each newline.
    /// This changes with the level of `BlockQuote` and `List` events.
    pub padding: Vec<Cow<'a, str>>,
    /// Keeps the current table alignments, if we are currently serializing a table.
    pub table_alignments: Vec<Alignment>,
    /// Keeps the current table headers, if we are currently serializing a table.
    pub table_headers: Vec<String>,
}

/// Configuration for the `cmark` function.
/// The defaults should provide decent spacing and most importantly, will
/// provide a faithful rendering of your markdown document particularly when
/// rendering it to HTML.
///
/// It's best used with its `Options::default()` implementation.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Options {
    pub newlines_after_headline: usize,
    pub newlines_after_paragraph: usize,
    pub newlines_after_codeblock: usize,
    pub newlines_after_table: usize,
    pub newlines_after_html: usize,
    pub newlines_after_rule: usize,
    pub newlines_after_list: usize,
    pub newlines_after_rest: usize,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            newlines_after_headline: 2,
            newlines_after_paragraph: 2,
            newlines_after_codeblock: 2,
            newlines_after_table: 2,
            newlines_after_html: 1,
            newlines_after_rule: 2,
            newlines_after_list: 2,
            newlines_after_rest: 1,
        }
    }
}

/// Serialize a stream of [pulldown-cmark-Events][pd-event] into a string-backed buffer.
///
/// 1. **events**
///   * An iterator over [`Events`][pd-event], for example as returned by the [`Parser`][pd-parser]
/// 1. **formatter**
///   * A format writer, can be a `String`.
/// 1. **state**
///   * The optional initial state of the serialization.
/// 1. **options**
///   * Customize the appearance of the serialization. All otherwise magic values are contained
///     here.
///
/// *Returns* the `State` of the serialization on success. You can use it as initial state in the
/// next call if you are halting event serialization.
/// *Errors* are only happening if the underlying buffer fails, which is unlikely.
///
/// [pd-event]: https://docs.rs/pulldown-cmark/*/pulldown_cmark/enum.Event.html
/// [pd-parser]: https://docs.rs/pulldown-cmark/*/pulldown_cmark/struct.Parser.html
pub fn cmark_with_options<'a, I, E, F>(
    events: I,
    mut formatter: F,
    state: Option<State<'static>>,
    options: Options,
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
    fn consume_newlines<F>(f: &mut F, s: &mut State) -> fmt::Result
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

    fn print_text<'a, F>(t: &str, f: &mut F, p: &[Cow<'a, str>]) -> fmt::Result
    where
        F: fmt::Write,
    {
        if t.contains('\n') {
            let ntokens = t.split('\n').count();
            for (tid, token) in t.split('\n').enumerate() {
                f.write_str(token).and(if tid + 1 == ntokens {
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

    fn padding_of(l: Option<usize>) -> Cow<'static, str> {
        match l {
            None => "  ".into(),
            Some(n) => format!("{}. ", n)
                .chars()
                .map(|_| ' ')
                .collect::<String>()
                .into(),
        }
    }

    for event in events {
        use pulldown_cmark::Event::*;
        use pulldown_cmark::Tag::*;
        match *event.borrow() {
            Start(ref tag) => {
                match *tag {
                    BlockQuote => state.padding.push(" > ".into()),
                    List(ref list_type) => {
                        state.list_stack.push(list_type.clone());
                        if state.list_stack.len() > 1 {
                            if state.newlines_before_start < options.newlines_after_rest {
                                state.newlines_before_start = options.newlines_after_rest;
                            }
                        }
                    }
                    _ => {}
                }
                let left_on_padded_newlines = state.newlines_before_start != 0;
                consume_newlines(&mut formatter, &mut state)?;
                match *tag {
                    Item => match state.list_stack.last() {
                        Some(inner) => {
                            state.padding.push(padding_of(*inner));
                            match inner {
                                &Some(n) => write!(formatter, "{}. ", n),
                                &None => formatter.write_str("* "),
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
                    TableCell => formatter.write_char('|'),
                    Link(_, _) => formatter.write_char('['),
                    Image(_, _) => formatter.write_str("!["),
                    Emphasis => formatter.write_char('*'),
                    Strong => formatter.write_str("**"),
                    Code => formatter.write_char('`'),
                    FootnoteDefinition(ref name) => write!(formatter, "[^{}]: ", name),
                    Paragraph => Ok(()),
                    Rule => formatter.write_str("---"),
                    Header(n) => {
                        for _ in 0..n {
                            formatter.write_char('#')?;
                        }
                        formatter.write_char(' ')
                    }
                    BlockQuote => if !left_on_padded_newlines {
                        padding(&mut formatter, &state.padding)
                    } else {
                        Ok(())
                    },
                    CodeBlock(ref info) => formatter.write_str("```")
                        .and(formatter.write_str(info))
                        .and(formatter.write_char('\n'))
                        .and(padding(&mut formatter, &state.padding)),
                    List(_) => Ok(()),
                }
            }
            End(ref tag) => match *tag {
                Image(ref uri, ref title) | Link(ref uri, ref title) => {
                    if title.is_empty() {
                        write!(formatter, "]({})", uri)
                    } else {
                        write!(formatter, "]({uri} \"{title}\")", uri = uri, title = title)
                    }
                }
                Code => formatter.write_char('`'),
                Emphasis => formatter.write_char('*'),
                Strong => formatter.write_str("**"),
                Header(_) => {
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
                    formatter.write_str("```")
                }
                Rule => {
                    if state.newlines_before_start < options.newlines_after_rule {
                        state.newlines_before_start = options.newlines_after_rule;
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
                TableCell => Ok(()),
                ref t @ TableRow | ref t @ TableHead => {
                    if state.newlines_before_start < options.newlines_after_rest {
                        state.newlines_before_start = options.newlines_after_rest;
                    }
                    formatter.write_char('|')?;

                    if let &TableHead = t {
                        formatter.write_char('\n').and(padding(&mut formatter, &state.padding))?;
                        for (alignment, name) in state
                            .table_alignments
                            .iter()
                            .zip(state.table_headers.iter())
                        {
                            formatter.write_char('|')?;
                            let last_minus_one = name.len() - 1;
                            for c in 0..name.len() {
                                formatter.write_char(if (c == 0
                                    && (alignment == &Alignment::Center
                                        || alignment == &Alignment::Left))
                                    || (c == last_minus_one
                                        && (alignment == &Alignment::Center
                                            || alignment == &Alignment::Right))
                                {
                                    ':'
                                } else {
                                    '-'
                                })?;
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
                    if state.list_stack.len() == 0
                        && state.newlines_before_start < options.newlines_after_list
                    {
                        state.newlines_before_start = options.newlines_after_list;
                    }
                    Ok(())
                }
                BlockQuote => {
                    state.padding.pop();
                    Ok(())
                }
                FootnoteDefinition(_) => Ok(()),
            },
            HardBreak => formatter.write_str("  \n").and(padding(&mut formatter, &state.padding)),
            SoftBreak => formatter.write_char('\n').and(padding(&mut formatter, &state.padding)),
            Text(ref text) => {
                if state.table_alignments.len() != state.table_headers.len() {
                    state.table_headers.push(text.clone().into());
                }
                consume_newlines(&mut formatter, &mut state)?;
                print_text(text, &mut formatter, &state.padding)
            }
            Html(ref text) => {
                consume_newlines(&mut formatter, &mut state)?;
                print_text(text, &mut formatter, &state.padding)?;

                if state.newlines_before_start < options.newlines_after_html {
                    state.newlines_before_start = options.newlines_after_html;
                }
                Ok(())
            }
            InlineHtml(ref name) => formatter.write_str(name),
            FootnoteReference(ref name) => write!(formatter, "[^{}]", name),
        }?
    }
    Ok(state)
}

/// As `cmark_with_options`, but with default `Options`.
pub fn cmark<'a, I, E, F>(
    events: I,
    formatter: F,
    state: Option<State<'static>>,
) -> Result<State<'static>, fmt::Error>
where
    I: Iterator<Item = E>,
    E: Borrow<Event<'a>>,
    F: fmt::Write,
{
    cmark_with_options(events, formatter, state, Options::default())
}
