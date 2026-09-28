use crate::{state::BufferedEvent, text_modifications::*, Error, Options};
use pulldown_cmark::{Alignment, BlockQuoteKind, CodeBlockKind, Event, LinkType, MetadataBlockKind, Tag, TagEnd};
use std::{
    borrow::Cow,
    collections::HashSet,
    fmt::{self, Write},
};

/// Prefixes are written only when their line is written. Closing a container can
/// therefore change the prefix after an HTML/code newline without adding a line.
#[derive(Clone, Debug, Default)]
struct Output {
    text: String,
    prefix: String,
    line_start: bool,
    newlines: usize,
}

impl Output {
    fn newline(&mut self) -> fmt::Result {
        self.write_char('\n')
    }

    fn separate(&mut self, lines: usize) -> fmt::Result {
        while self.newlines < lines {
            self.newline()?;
        }
        Ok(())
    }
}

impl fmt::Write for Output {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        for part in text.split_inclusive('\n') {
            if self.line_start {
                self.text.push_str(&self.prefix);
            }
            self.text.push_str(part);
            self.line_start = part.ends_with('\n');
            if part != "\n" {
                self.newlines = 0;
            }
            if self.line_start {
                self.newlines += 1;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ItemTail {
    Paragraph,
    Empty,
    Block,
}

#[derive(Clone, Copy, Debug, Default)]
enum Last {
    #[default]
    Other,
    Inline,
    ItemStart,
    ItemEnd(ItemTail),
    ListEnd(ItemTail),
    DefinitionEnd,
}

#[derive(Clone, Debug)]
enum Content {
    Plain,
    List {
        number: Option<u64>,
        token: char,
        tight: bool,
        wide: bool,
    },
    Heading {
        setext: bool,
    },
    Delimited(String),
    Link {
        label_start: usize,
        original_label: bool,
    },
    Table {
        headers: Vec<String>,
    },
    Cell {
        text: String,
    },
}

#[derive(Clone, Debug)]
struct Frame {
    tag: Tag<'static>,
    prefix: String,
    content: Content,
    start: usize,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Renderer {
    output: Output,
    frames: Vec<Frame>,
    separator: usize,
    last: Last,
    last_list: Option<(usize, bool, char)>,
    quote_end: Option<String>,
    references: Vec<(String, String, String)>,
}

impl Renderer {
    pub(crate) fn block(
        &mut self,
        events: &[BufferedEvent],
        next: Option<&Event<'_>>,
        options: &Options<'_>,
    ) -> Result<String, Error> {
        if events.iter().any(|event| event.source.is_some()) {
            let mut candidate = self.clone();
            let output = candidate.render_block(events, next, options, true)?;
            let mut with_references = output.clone();
            with_references.push_str(&candidate.clone().finish()?);
            if events_match(&with_references, events) {
                *self = candidate;
                return Ok(output);
            }
        }
        self.render_block(events, next, options, false)
    }

    fn render_block(
        &mut self,
        events: &[BufferedEvent],
        next: Option<&Event<'_>>,
        options: &Options<'_>,
        source_spelling: bool,
    ) -> Result<String, Error> {
        // Match containers once; all lookahead stays inside this buffered block.
        let mut ends = vec![events.len(); events.len()];
        let mut open = Vec::new();
        for (i, event) in events.iter().enumerate() {
            match &event.event {
                Event::Start(_) => open.push(i),
                Event::End(_) => {
                    let start = open.pop().ok_or(Error::UnexpectedEvent)?;
                    ends[start] = i;
                }
                _ => {}
            }
        }
        if !open.is_empty() {
            return Err(Error::UnexpectedEvent);
        }
        let mut i = 0;
        while i < events.len() {
            let event = &events[i].event;
            self.before(event)?;
            let previous = std::mem::take(&mut self.last);
            match event {
                Event::Start(tag) => {
                    let body = &events[i + 1..ends[i]];
                    let sibling = events.get(ends[i] + 1).map(|e| &e.event).or(next);
                    self.start(tag, body, sibling, options)?;
                    if let Tag::Link {
                        link_type: LinkType::Collapsed | LinkType::Shortcut,
                        id,
                        ..
                    }
                    | Tag::Image {
                        link_type: LinkType::Collapsed | LinkType::Shortcut,
                        id,
                        ..
                    } = tag
                    {
                        if matches!(
                            self.frames.last().map(|f| &f.content),
                            Some(Content::Link {
                                original_label: true,
                                ..
                            })
                        ) {
                            self.output.write_str(id)?;
                            self.last = Last::Inline;
                            i = ends[i] - 1;
                        }
                    }
                }
                Event::End(end) => self.end(*end, previous, options)?,
                Event::Text(text) => {
                    // Adjacent Text events have no semantic boundary. Joining them
                    // also prevents parser escape/entity splits from changing how
                    // a leading list marker or an intraword underscore is escaped.
                    let mut text = Cow::Borrowed(text.as_ref());
                    let mut source = events[i].source.as_deref().map(Cow::Borrowed);
                    while let Some(BufferedEvent {
                        event: Event::Text(part),
                        source: spelling,
                    }) = events.get(i + 1)
                    {
                        text.to_mut().push_str(part);
                        source = match (source, spelling) {
                            (Some(mut source), Some(spelling)) => {
                                source.to_mut().push_str(spelling);
                                Some(source)
                            }
                            _ => None,
                        };
                        i += 1;
                    }
                    let after = events.get(i + 1).map(|e| &e.event).or(next);
                    if let Some(Frame {
                        content: Content::Cell { text: header },
                        ..
                    }) = self
                        .frames
                        .iter_mut()
                        .rev()
                        .find(|f| matches!(f.content, Content::Cell { .. }))
                    {
                        header.push_str(&text);
                    }
                    if self.raw_text() {
                        self.output.write_str(&text)?;
                    } else if let Some(source) = source.as_deref().filter(|_| source_spelling) {
                        self.output.write_str(source)?;
                    } else {
                        let escaped = self.escape_text(&text, after, options);
                        self.output.write_str(&escaped)?;
                    }
                    self.last = Last::Inline;
                }
                Event::Code(text) => {
                    if let Some(Frame {
                        content: Content::Cell { text: header },
                        ..
                    }) = self
                        .frames
                        .iter_mut()
                        .rev()
                        .find(|f| matches!(f.content, Content::Cell { .. }))
                    {
                        write!(header, "`{text}`")?;
                    }
                    let text = if self.in_table() {
                        text.replace('|', "\\|")
                    } else {
                        text.to_string()
                    };
                    if text.chars().all(|c| c == ' ') {
                        write!(self.output, "`{text}`")?;
                    } else {
                        let ticks = Repeated('`', max_consecutive_chars(&text, '`') + 1);
                        let space = match text.as_bytes() {
                            [b'`', ..] | [.., b'`'] | [b' ', .., b' '] => " ",
                            _ => "",
                        };
                        write!(self.output, "{ticks}{space}{text}{space}{ticks}")?;
                    }
                    self.last = Last::Inline;
                }
                Event::Html(text) | Event::InlineHtml(text) => {
                    self.output.write_str(text)?;
                    if matches!(event, Event::InlineHtml(_)) {
                        self.last = Last::Inline;
                    }
                }
                Event::SoftBreak => {
                    self.output.newline()?;
                    self.last = Last::Inline;
                }
                Event::HardBreak => {
                    self.output.write_str("  \n")?;
                    self.last = Last::Inline;
                }
                Event::Rule => {
                    self.output
                        .write_str(if matches!(previous, Last::Inline) { "***" } else { "---" })?;
                    self.separator = options.newlines_after_rule.max(1);
                }
                Event::FootnoteReference(name) => {
                    write!(self.output, "[^{name}]")?;
                    self.last = Last::Inline;
                }
                Event::TaskListMarker(checked) => {
                    self.output.write_str(if *checked { "[x] " } else { "[ ] " })?;
                    self.last = Last::Inline;
                }
                Event::InlineMath(text) => {
                    write!(self.output, "${text}$")?;
                    self.last = Last::Inline;
                }
                Event::DisplayMath(text) => {
                    write!(self.output, "$${text}$$")?;
                    self.last = Last::Inline;
                }
            }
            i += 1;
        }
        Ok(std::mem::take(&mut self.output.text))
    }

    fn prefix(&mut self) {
        self.output.prefix = self.frames.iter().map(|f| f.prefix.as_str()).collect();
    }

    fn tight_boundary(&self) -> bool {
        match self.frames.as_slice() {
            [.., Frame {
                content: Content::List { tight, .. },
                ..
            }] => *tight,
            [.., Frame {
                content: Content::List { tight, .. },
                ..
            }, Frame { tag: Tag::Item, .. }] => *tight,
            _ => false,
        }
    }

    fn before(&mut self, event: &Event<'_>) -> Result<(), Error> {
        if matches!(event, Event::End(_) | Event::Html(_)) {
            return Ok(());
        }
        if !matches!(event, Event::Start(Tag::List(_))) {
            self.last_list = None;
        }
        if matches!(event, Event::Start(Tag::List(_))) && self.frames.iter().any(|f| matches!(f.tag, Tag::List(_))) {
            self.separator = self.separator.max(1);
        }
        if matches!(self.last, Last::DefinitionEnd) && matches!(event, Event::Start(Tag::DefinitionListTitle)) {
            self.separator = self.separator.max(2);
        }
        let block = matches!(event, Event::Start(tag) if is_block(tag.to_end())) || matches!(event, Event::Rule);
        if block && matches!(self.last, Last::Inline) {
            self.separator = self.separator.max(1);
        }
        if self.tight_boundary() {
            self.separator = self.separator.min(1);
            if !matches!(event, Event::Start(Tag::Item)) {
                if let Some(line) = self.quote_end.take() {
                    self.output.separate(1)?;
                    self.output.prefix.clear();
                    self.output.write_str(&line)?;
                    self.prefix();
                    self.separator = 1;
                }
            } else {
                self.quote_end = None;
            }
        } else {
            self.quote_end = None;
        }
        if let (Last::ListEnd(tail), Event::Start(tag)) = (self.last, event) {
            if needs_blank_after_list(tag, tail) {
                self.separator = self.separator.max(2);
            }
        }
        if !matches!(event, Event::SoftBreak | Event::HardBreak) {
            self.output.separate(std::mem::take(&mut self.separator))?;
        }
        Ok(())
    }

    fn start(
        &mut self,
        tag: &Tag<'static>,
        body: &[BufferedEvent],
        sibling: Option<&Event<'_>>,
        options: &Options<'_>,
    ) -> Result<(), Error> {
        let mut frame = Frame {
            tag: tag.clone(),
            prefix: String::new(),
            content: Content::Plain,
            start: 0,
        };
        let previous_list = self.last_list.take();
        match tag {
            Tag::List(number) => {
                let mut depth = 0;
                let mut tight = true;
                for item in body {
                    match &item.event {
                        Event::Start(Tag::Paragraph) if depth == 1 => {
                            tight = false;
                            depth += 1;
                        }
                        Event::Start(_) => depth += 1,
                        Event::End(_) => depth -= 1,
                        _ => {}
                    }
                }
                let mut token = if number.is_some() {
                    options.ordered_list_token
                } else {
                    options.list_token
                };
                if let Some((depth, ordered, previous)) = previous_list {
                    if depth == self.frames.len() && ordered == number.is_some() && token == previous {
                        token = if ordered {
                            if token == '.' {
                                ')'
                            } else {
                                '.'
                            }
                        } else if token == '*' {
                            '-'
                        } else {
                            '*'
                        };
                    }
                }
                frame.content = Content::List {
                    number: *number,
                    token,
                    tight,
                    wide: matches!(sibling, Some(Event::Start(Tag::CodeBlock(CodeBlockKind::Indented)))),
                };
            }
            Tag::Item => {
                let Some(Frame {
                    content: Content::List {
                        number, token, wide, ..
                    },
                    ..
                }) = self.frames.last_mut()
                else {
                    return Err(Error::UnexpectedEvent);
                };
                let leading = if *wide { "   " } else { "" };
                let space = if *wide { "    " } else { " " };
                let marker = match number {
                    Some(number) => {
                        let marker = format!("{leading}{number}{token}{space}");
                        if options.increment_ordered_list_bullets {
                            *number += 1;
                        }
                        marker
                    }
                    None => format!("{leading}{token}{space}"),
                };
                frame.prefix = " ".repeat(marker.len());
                self.output.write_str(&marker)?;
                self.last = Last::ItemStart;
            }
            Tag::Paragraph => {}
            Tag::Heading { level, .. } => {
                if self.frames.iter().any(|f| matches!(f.tag, Tag::Heading { .. })) {
                    return Err(Error::UnexpectedEvent);
                }
                let setext = body.iter().any(|e| {
                    matches!(&e.event, Event::SoftBreak | Event::HardBreak)
                        || matches!(&e.event, Event::InlineHtml(t) if t.contains('\n'))
                });
                if !setext {
                    write!(self.output, "{} ", Repeated('#', *level as usize))?;
                }
                frame.content = Content::Heading { setext };
            }
            Tag::BlockQuote(kind) => {
                if self.output.newlines == 0 {
                    self.output.newline()?;
                }
                let first = match kind {
                    Some(BlockQuoteKind::Note) => " > [!NOTE]",
                    Some(BlockQuoteKind::Tip) => " > [!TIP]",
                    Some(BlockQuoteKind::Important) => " > [!IMPORTANT]",
                    Some(BlockQuoteKind::Warning) => " > [!WARNING]",
                    Some(BlockQuoteKind::Caution) => " > [!CAUTION]",
                    None => " > ",
                };
                self.output.write_str(first)?;
                frame.prefix = " > ".into();
                self.separator = 1;
            }
            Tag::CodeBlock(CodeBlockKind::Indented) => {
                if self.output.newlines == 0 {
                    self.output.newline()?;
                }
                frame.prefix = "    ".into();
            }
            Tag::CodeBlock(CodeBlockKind::Fenced(info)) => {
                if self.output.newlines == 0 {
                    self.output.newline()?;
                }
                let token = if options.code_block_token == '`' && info.contains('`') {
                    '~'
                } else {
                    options.code_block_token
                };
                let text: String = body
                    .iter()
                    .filter_map(|e| {
                        if let Event::Text(text) = &e.event {
                            Some(text.as_ref())
                        } else {
                            None
                        }
                    })
                    .collect();
                let length = options
                    .code_block_token_count
                    .max(3)
                    .max(max_consecutive_chars(&text, token) + 1);
                let fence = Repeated(token, length).to_string();
                writeln!(
                    self.output,
                    "{fence}{}",
                    info.replace('&', "&amp;").replace('\\', "\\\\")
                )?;
                frame.content = Content::Delimited(fence);
            }
            Tag::HtmlBlock => {}
            Tag::Emphasis | Tag::Strong => {
                let mut token = if matches!(tag, Tag::Emphasis) {
                    options.emphasis_token.to_string()
                } else {
                    options.strong_token.to_owned()
                };
                if matches!(tag, Tag::Emphasis)
                    && self
                        .frames
                        .iter()
                        .rev()
                        .find(|f| matches!(f.tag, Tag::Emphasis))
                        .is_some_and(|f| matches!(&f.content, Content::Delimited(parent) if parent == &token))
                {
                    token = if token == "*" { "_".into() } else { "*".into() };
                }
                self.output.write_str(&token)?;
                frame.content = Content::Delimited(token);
            }
            Tag::Strikethrough | Tag::Superscript | Tag::Subscript => {
                let (start, end) = match tag {
                    Tag::Strikethrough => ("~~", "~~"),
                    Tag::Superscript if options.use_html_for_super_sub_script => ("<sup>", "</sup>"),
                    Tag::Subscript if options.use_html_for_super_sub_script => ("<sub>", "</sub>"),
                    Tag::Superscript => ("^", "^"),
                    _ => ("~", "~"),
                };
                self.output.write_str(start)?;
                frame.content = Content::Delimited(end.into());
            }
            Tag::Link { link_type, id, .. } | Tag::Image { link_type, id, .. } => {
                self.output.write_str(if matches!(tag, Tag::Image { .. }) {
                    "!["
                } else if matches!(link_type, LinkType::Autolink | LinkType::Email) {
                    "<"
                } else {
                    "["
                })?;
                frame.content = Content::Link {
                    label_start: self.output.text.len(),
                    original_label: matches!(link_type, LinkType::Collapsed | LinkType::Shortcut)
                        && label_matches(id, body, matches!(tag, Tag::Image { .. })),
                };
            }
            Tag::FootnoteDefinition(name) => {
                write!(self.output, "[^{name}]: ")?;
                frame.prefix = "    ".into();
            }
            Tag::Table(_) => {
                frame.content = Content::Table { headers: Vec::new() };
            }
            Tag::TableHead | Tag::TableRow => {}
            Tag::TableCell => {
                self.output.write_char('|')?;
                frame.content = Content::Cell { text: String::new() };
            }
            Tag::DefinitionList => {}
            Tag::DefinitionListTitle => {
                // A term starts a paragraph, even after an unwrapped tight item.
                if self.output.newlines == 0 && !self.output.text.is_empty() {
                    self.output.newline()?;
                }
            }
            Tag::DefinitionListDefinition => {
                if matches!(body.first().map(|e| &e.event), Some(Event::Start(Tag::Paragraph))) {
                    self.output.separate(2)?;
                }
                self.output.write_str(": ")?;
                frame.prefix = "  ".into();
            }
            Tag::MetadataBlock(kind) => {
                self.output.write_str(metadata_fence(*kind))?;
                self.output.newline()?;
            }
        }
        frame.start = self.output.text.len();
        self.frames.push(frame);
        self.prefix();
        Ok(())
    }

    fn end(&mut self, end: TagEnd, previous: Last, options: &Options<'_>) -> Result<(), Error> {
        let frame = self.frames.pop().ok_or(Error::UnexpectedEvent)?;
        if frame.tag.to_end() != end {
            return Err(Error::UnexpectedEvent);
        }
        self.prefix();
        match frame.tag {
            Tag::Paragraph => self.separator = options.newlines_after_paragraph.max(2),
            Tag::Heading {
                level,
                id,
                classes,
                attrs,
            } => {
                if id.is_some() || !classes.is_empty() || !attrs.is_empty() {
                    self.output.write_str(" {")?;
                    if let Some(id) = id {
                        write!(self.output, " #{id}")?;
                    }
                    for class in classes {
                        write!(self.output, " .{class}")?;
                    }
                    for (key, value) in attrs {
                        write!(self.output, " {key}")?;
                        if let Some(value) = value {
                            write!(self.output, "={value}")?;
                        }
                    }
                    self.output.write_str(" }")?;
                }
                if matches!(frame.content, Content::Heading { setext: true }) {
                    self.output.newline()?;
                    self.output.write_str(if level as usize == 1 { "===" } else { "---" })?;
                }
                self.separator = options.newlines_after_headline.max(1);
            }
            Tag::CodeBlock(_) => {
                if self.output.newlines == 0 {
                    self.output.newline()?;
                }
                if let Content::Delimited(fence) = frame.content {
                    self.output.write_str(&fence)?;
                }
                self.separator = options.newlines_after_codeblock.max(2);
            }
            Tag::HtmlBlock => self.separator = options.newlines_after_htmlblock.max(2),
            Tag::BlockQuote(_) => {
                self.separator = options.newlines_after_blockquote.max(2);
                self.quote_end = self
                    .frames
                    .iter()
                    .any(|f| matches!(f.tag, Tag::Item))
                    .then(|| format!("{} >", self.output.prefix));
            }
            Tag::Item => {
                self.separator = self.separator.max(options.newlines_after_rest.max(1));
                self.last = Last::ItemEnd(match previous {
                    Last::Inline => ItemTail::Paragraph,
                    Last::ItemStart => ItemTail::Empty,
                    Last::ListEnd(tail) => tail,
                    _ => ItemTail::Block,
                });
                if !self.frames.iter().any(|f| matches!(f.tag, Tag::Item)) {
                    self.quote_end = None;
                }
            }
            Tag::List(_) => {
                if let Content::List { number, token, .. } = frame.content {
                    self.last_list = Some((self.frames.len(), number.is_some(), token));
                }
                if self.frames.iter().any(|f| matches!(f.tag, Tag::List(_))) {
                    self.last = Last::ListEnd(match previous {
                        Last::ItemEnd(tail) => tail,
                        _ => ItemTail::Block,
                    });
                } else {
                    self.separator = self.separator.max(options.newlines_after_list.max(2));
                }
            }
            Tag::Emphasis | Tag::Strong | Tag::Strikethrough | Tag::Superscript | Tag::Subscript => {
                if let Content::Delimited(token) = frame.content {
                    self.output.write_str(&token)?;
                }
                self.last = Last::Inline;
            }
            Tag::Link {
                link_type,
                dest_url,
                title,
                id,
            }
            | Tag::Image {
                link_type,
                dest_url,
                title,
                id,
            } => {
                let Content::Link {
                    label_start,
                    original_label,
                } = frame.content
                else {
                    return Err(Error::UnexpectedEvent);
                };
                match link_type {
                    LinkType::Autolink | LinkType::Email => self.output.write_char('>')?,
                    LinkType::Reference => {
                        write!(self.output, "][{id}]")?;
                        self.references
                            .push((id.into_string(), dest_url.into_string(), title.into_string()));
                    }
                    LinkType::Collapsed | LinkType::Shortcut if !original_label && !id.is_empty() => {
                        // A filter edited the children but kept the old ID. An
                        // explicit reference preserves both the edit and target.
                        write!(self.output, "][{id}]")?;
                        self.references
                            .push((id.into_string(), dest_url.into_string(), title.into_string()));
                    }
                    LinkType::Collapsed | LinkType::Shortcut => {
                        let label = self.output.text[label_start..].to_owned();
                        self.references
                            .push((label, dest_url.into_string(), title.into_string()));
                        self.output
                            .write_str(if link_type == LinkType::Collapsed { "][]" } else { "]" })?;
                    }
                    _ => close_link(&dest_url, &title, &mut self.output, LinkType::Inline)?,
                }
                self.last = Last::Inline;
            }
            Tag::FootnoteDefinition(_) => {}
            Tag::Table(_) => self.separator = options.newlines_after_table.max(2),
            Tag::TableCell => {
                let Content::Cell { text } = frame.content else {
                    return Err(Error::UnexpectedEvent);
                };
                let Some(Frame {
                    content: Content::Table { headers },
                    ..
                }) = self.frames.iter_mut().rev().find(|f| matches!(f.tag, Tag::Table(_)))
                else {
                    return Err(Error::UnexpectedEvent);
                };
                headers.push(text);
            }
            Tag::TableRow | Tag::TableHead => {
                self.output.write_char('|')?;
                if matches!(frame.tag, Tag::TableHead) {
                    self.output.newline()?;
                    let Some(Frame {
                        tag: Tag::Table(alignments),
                        content: Content::Table { headers },
                        ..
                    }) = self.frames.last()
                    else {
                        return Err(Error::UnexpectedEvent);
                    };
                    for (alignment, header) in alignments.iter().zip(headers) {
                        self.output.write_char('|')?;
                        let minimum = match alignment {
                            Alignment::None => 1,
                            Alignment::Center => 3,
                            _ => 2,
                        };
                        let width = header.chars().count().max(minimum);
                        for col in 0..width {
                            self.output.write_char(
                                if col == 0 && matches!(alignment, Alignment::Left | Alignment::Center)
                                    || col == width - 1 && matches!(alignment, Alignment::Right | Alignment::Center)
                                {
                                    ':'
                                } else {
                                    '-'
                                },
                            )?;
                        }
                    }
                    self.output.write_char('|')?;
                }
                self.separator = options.newlines_after_rest.max(1);
            }
            Tag::DefinitionList => self.separator = options.newlines_after_list.max(2),
            Tag::DefinitionListTitle => self.separator = 1,
            Tag::DefinitionListDefinition => {
                // Terminate a quote before writing outside the definition. A later
                // quote marker would start a new quote instead of closing this one.
                if let Some(line) = self.quote_end.take() {
                    self.output.separate(1)?;
                    self.output.prefix.clear();
                    self.output.write_str(&line)?;
                    self.prefix();
                }
                self.output.newline()?;
                self.separator = 1;
                self.last = Last::DefinitionEnd;
            }
            Tag::MetadataBlock(kind) => {
                self.output.write_str(metadata_fence(kind))?;
                self.output.newline()?;
                self.separator = options.newlines_after_metadata + 1;
            }
        }
        Ok(())
    }

    fn raw_text(&self) -> bool {
        self.frames.iter().any(|f| {
            matches!(
                f.tag,
                Tag::CodeBlock(_)
                    | Tag::HtmlBlock
                    | Tag::MetadataBlock(_)
                    | Tag::Link {
                        link_type: LinkType::Autolink | LinkType::Email,
                        ..
                    }
            )
        })
    }

    fn in_table(&self) -> bool {
        self.frames.iter().any(|f| matches!(f.tag, Tag::Table(_)))
    }

    fn escape_text(&self, text: &str, next: Option<&Event<'_>>, options: &Options<'_>) -> String {
        let mut escaped = String::with_capacity(text.len());
        let line_start = self.output.line_start
            || self.output.text.is_empty()
            || self.frames.last().is_some_and(|f| f.start == self.output.text.len());
        let marker_end = text.as_bytes().get(1).map_or(
            matches!(next, None | Some(Event::End(_) | Event::SoftBreak | Event::HardBreak)),
            |c| c.is_ascii_whitespace() || matches!(c, b'-' | b'+' | b'='),
        );
        let heading = self.frames.iter().any(|f| matches!(f.tag, Tag::Heading { .. }));
        let link_next = matches!(
            next,
            Some(Event::Start(Tag::Link { .. } | Tag::Image { .. }) | Event::FootnoteReference(_))
        );
        for (i, c) in text.char_indices() {
            match c {
                '\n' => escaped.push_str("&#10;"),
                '\r' => escaped.push_str("&#13;"),
                '&' => escaped.push_str("&amp;"),
                ' ' | '\t'
                    if i == 0
                        && (self.output.line_start
                            || self.frames.last().is_some_and(|f| f.start == self.output.text.len())
                            || self.output.text.is_empty()) =>
                {
                    escaped.push_str(if c == ' ' { "&#32;" } else { "&#9;" })
                }
                _ => {
                    if matches!(c, '*' | '[' | ']' | '`' | '<' | '~')
                        || (c == '\\' && text[i + 1..].chars().next().map_or(true, |c| c.is_ascii_punctuation()))
                        || (c == '>' && i == 0)
                        || (c == '^' && !text[..i].chars().next_back().is_some_and(char::is_alphanumeric))
                        || (c == '_'
                            && !(text[..i].chars().next_back().is_some_and(char::is_alphanumeric)
                                && text[i + 1..].chars().next().is_some_and(char::is_alphanumeric)))
                        || (c == '|' && (i == 0 || self.in_table()))
                        || (c == '#' && (i == 0 || heading))
                        || (matches!(c, '-' | '+' | '=') && i == 0 && line_start && marker_end)
                        || (matches!(c, '.' | ')') && i > 0 && text[..i].bytes().all(|c| c.is_ascii_digit()))
                        || (c == '!' && i + 1 == text.len() && link_next)
                        || (c == options.emphasis_token && !matches!(c, '*' | '_'))
                    {
                        escaped.push('\\');
                    }
                    escaped.push(c);
                }
            }
        }
        escaped
    }

    pub(crate) fn finish(&mut self) -> Result<String, Error> {
        if !self.references.is_empty() {
            self.output.prefix.clear();
            self.output.separate(2)?;
            let mut seen = HashSet::new();
            let mut first = true;
            for reference in self.references.drain(..) {
                if !seen.insert(reference.clone()) {
                    continue;
                }
                if !first {
                    self.output.newline()?;
                }
                first = false;
                write!(self.output, "[{}", reference.0)?;
                close_link(&reference.1, &reference.2, &mut self.output, LinkType::Shortcut)?;
            }
        }
        Ok(std::mem::take(&mut self.output.text))
    }
}

fn metadata_fence(kind: MetadataBlockKind) -> &'static str {
    match kind {
        MetadataBlockKind::YamlStyle => "---",
        MetadataBlockKind::PlusesStyle => "+++",
    }
}

fn needs_blank_after_list(tag: &Tag<'_>, tail: ItemTail) -> bool {
    match tag {
        Tag::Paragraph | Tag::DefinitionList => true,
        Tag::Table(_) => tail == ItemTail::Paragraph,
        Tag::CodeBlock(CodeBlockKind::Indented) => tail != ItemTail::Block,
        _ => false,
    }
}

pub(crate) fn is_block(tag: TagEnd) -> bool {
    !matches!(
        tag,
        TagEnd::Emphasis
            | TagEnd::Strong
            | TagEnd::Strikethrough
            | TagEnd::Superscript
            | TagEnd::Subscript
            | TagEnd::Link
            | TagEnd::Image
    )
}

// A reference ID contains Markdown syntax, not just decoded text. Reuse its
// spelling only when it still describes the supplied children: event filters
// are allowed to edit the label independently of the reference ID.
fn label_matches(id: &str, body: &[BufferedEvent], image: bool) -> bool {
    use pulldown_cmark::{utils::TextMergeStream, Parser};
    let markdown = format!("{}[{id}](cmark-label)", if image { "!" } else { "" });
    let expected: Vec<_> = TextMergeStream::new(body.iter().map(|e| e.event.clone())).collect();
    [
        pulldown_cmark::Options::empty(),
        pulldown_cmark::Options::all() - pulldown_cmark::Options::ENABLE_SMART_PUNCTUATION,
    ]
    .iter()
    .any(|options| {
        let parsed: Vec<_> = TextMergeStream::new(Parser::new_ext(&markdown, *options)).collect();
        matches!(parsed.first(), Some(Event::Start(Tag::Paragraph)))
            && if image {
                matches!(parsed.get(1), Some(Event::Start(Tag::Image { .. })))
            } else {
                matches!(parsed.get(1), Some(Event::Start(Tag::Link { .. })))
            }
            && matches!(parsed.last(), Some(Event::End(TagEnd::Paragraph)))
            && parsed.len() >= 4
            && parsed[2..parsed.len() - 2] == expected
    })
}

// Source slices are spelling hints. Check them after rendering the whole block,
// because its new indentation or surrounding delimiters can change their meaning.
fn events_match(markdown: &str, expected: &[BufferedEvent]) -> bool {
    use pulldown_cmark::{utils::TextMergeStream, Parser};
    let expected: Vec<_> = TextMergeStream::new(expected.iter().map(|e| e.event.clone())).collect();
    [
        pulldown_cmark::Options::empty(),
        pulldown_cmark::Options::all() - pulldown_cmark::Options::ENABLE_SMART_PUNCTUATION,
    ]
    .iter()
    .any(|options| TextMergeStream::new(Parser::new_ext(markdown, *options)).eq(expected.iter().cloned()))
}
