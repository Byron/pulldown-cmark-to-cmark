use crate::{state::BufferedEvent, text_modifications::*, Error, Options};
use pulldown_cmark::{
    Alignment, BlockQuoteKind, CodeBlockKind, CowStr, Event, LinkType, MetadataBlockKind, Options as ParserOptions,
    Parser, Tag, TagEnd,
};
use std::{
    borrow::Cow,
    collections::HashSet,
    fmt::{self, Write},
};
use unicase::UniCase;

/// Prefixes are written only when their line is written. Closing a container can
/// therefore change the prefix after an HTML/code newline without adding a line.
#[derive(Clone, Debug, Default)]
struct Output {
    text: String,
    prefix: String,
    line_start: bool,
    newlines: usize,
    marker: Option<u8>,
    marker_runs: Vec<usize>,
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
            for byte in part.bytes() {
                if matches!(byte, b'*' | b'_') {
                    if self.marker == Some(byte) {
                        *self.marker_runs.last_mut().expect("active marker run") += 1;
                    } else {
                        self.marker_runs.push(1);
                        self.marker = Some(byte);
                    }
                } else {
                    self.marker = None;
                }
            }
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
    BlockEnd(TagEnd),
}

#[derive(Clone, Debug)]
enum Content {
    Plain,
    List {
        number: Option<u64>,
        token: char,
        tight: bool,
        wide: bool,
        placeholder: bool,
    },
    Heading {
        setext: bool,
    },
    Html {
        blank_terminated: bool,
        closed: bool,
    },
    Definition {
        tight: bool,
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
    opening_run: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Layout {
    Preferred,
    Runs,
    Uniform,
    Shared { ends: bool, adjacent: bool, split: bool },
    RuleThree,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum TextSpelling {
    Escaped,
    Literal,
    Entities,
    LiteralEntities,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Delimiters {
    layout: Layout,
    text: TextSpelling,
}

impl Delimiters {
    const PREFERRED: Self = Self::new(Layout::Preferred, TextSpelling::Escaped);
    const RUNS: Self = Self::new(Layout::Runs, TextSpelling::Escaped);

    // A fixed number of passes bounds work on adversarial delimiter sequences.
    // Literal runs can participate in the rule of three; character references
    // can make word boundaries safe for underscores without changing the text.
    const FALLBACKS: [Self; 14] = [
        Self::new(Layout::RuleThree, TextSpelling::Entities),
        Self::new(Layout::RuleThree, TextSpelling::LiteralEntities),
        Self::new(Layout::Runs, TextSpelling::Literal),
        Self::new(Layout::Uniform, TextSpelling::Escaped),
        Self::new(Layout::Uniform, TextSpelling::Literal),
        Self::shared(false, false, false, TextSpelling::Escaped),
        Self::shared(true, false, false, TextSpelling::Escaped),
        Self::shared(false, true, false, TextSpelling::Escaped),
        Self::shared(true, true, false, TextSpelling::Escaped),
        Self::shared(false, false, false, TextSpelling::Entities),
        Self::shared(false, false, false, TextSpelling::Literal),
        Self::shared(true, false, false, TextSpelling::Literal),
        Self::shared(false, false, true, TextSpelling::Escaped),
        Self::shared(true, false, true, TextSpelling::Escaped),
    ];

    const fn new(layout: Layout, text: TextSpelling) -> Self {
        Self { layout, text }
    }

    const fn shared(ends: bool, adjacent: bool, split: bool, text: TextSpelling) -> Self {
        Self::new(Layout::Shared { ends, adjacent, split }, text)
    }

    fn shared_runs(self) -> bool {
        matches!(self.layout, Layout::Shared { .. } | Layout::RuleThree)
    }

    fn literal_runs(self) -> bool {
        matches!(self.text, TextSpelling::Literal | TextSpelling::LiteralEntities)
    }

    fn encode_edges(self) -> bool {
        matches!(self.text, TextSpelling::Entities | TextSpelling::LiteralEntities)
    }
}

#[derive(Clone, Copy)]
struct DelimiterContext {
    plan: Delimiters,
    literal_token: Option<char>,
    underscore_allowed: bool,
    follows_close: bool,
    opening_width: usize,
    split_shared: bool,
    starts_with_punctuation: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct Renderer {
    parser_options: ParserOptions,
    at_start: bool,
    output: Output,
    frames: Vec<Frame>,
    separator: usize,
    last: Last,
    last_list: Option<(usize, bool, char)>,
    quote_end: Option<String>,
    raw_eof: bool,
    open_html: bool,
    references: Vec<(String, String, String)>,
    reference_labels: HashSet<UniCase<CowStr<'static>>>,
    literal_labels: HashSet<UniCase<CowStr<'static>>>,
    list_label: Option<String>,
}

impl Renderer {
    pub(crate) fn new(parser_options: ParserOptions) -> Self {
        Self {
            parser_options,
            at_start: true,
            output: Output::default(),
            frames: Vec::new(),
            separator: 0,
            last: Last::Other,
            last_list: None,
            quote_end: None,
            raw_eof: false,
            open_html: false,
            references: Vec::new(),
            reference_labels: HashSet::new(),
            literal_labels: HashSet::new(),
            list_label: None,
        }
    }

    pub(crate) fn block(
        &mut self,
        events: &[BufferedEvent],
        next: Option<&Event<'_>>,
        options: &Options<'_>,
    ) -> Result<String, Error> {
        // Trial rendering must only copy and finalize this block's references.
        // Keep document history out of the snapshot, including on fallback.
        let mut references = std::mem::take(&mut self.references);
        let mut reference_labels = std::mem::take(&mut self.reference_labels);
        let mut literal_labels = std::mem::take(&mut self.literal_labels);
        let terminal_raw = next.is_none()
            && events
                .iter()
                .any(|event| matches!(event.event, Event::Start(Tag::CodeBlock(_) | Tag::HtmlBlock)));
        let result: Result<(String, String), Error> = (|| {
            let has_source = events.iter().any(|event| event.source.is_some());
            let has_delimiters = events.iter().any(|event| {
                matches!(
                    event.event,
                    Event::Start(Tag::Emphasis | Tag::Strong | Tag::Strikethrough | Tag::Superscript | Tag::Subscript)
                        | Event::InlineMath(_)
                        | Event::DisplayMath(_)
                )
            });
            if !matches!(options.emphasis_token, '*' | '_') || !matches!(options.strong_token, "**" | "__") {
                // Unvalidated custom tokens are intentionally allowed to change semantics.
                let output = self.render_block(events, next, options, false, Delimiters::PREFERRED)?;
                return Ok((output, self.reference_definitions()?));
            }
            if has_source {
                for runs in [Delimiters::PREFERRED, Delimiters::RUNS] {
                    if runs == Delimiters::RUNS && !has_delimiters {
                        break;
                    }
                    if let Some((renderer, output, definitions)) =
                        self.checked_block(events, next, options, true, runs, &reference_labels)?
                    {
                        *self = renderer;
                        return Ok((output, definitions));
                    }
                }
            }
            if !has_delimiters {
                let output = self.render_block(events, next, options, false, Delimiters::PREFERRED)?;
                return Ok((output, self.reference_definitions()?));
            }

            let mut original = self.clone();
            let output = original.render_block(events, next, options, false, Delimiters::PREFERRED)?;
            let definitions = original.reference_definitions()?;
            if events_match(
                &output,
                &definitions,
                events,
                &reference_labels,
                self.validation_options(),
            ) {
                *self = original;
                return Ok((output, definitions));
            }

            // Validate complete runs: changing one span can affect another span's
            // flanking or the rule of three. Keep the requested markers when safe.
            let alternate_emphasis = if options.emphasis_token == '*' { '_' } else { '*' };
            let alternate_strong = if options.strong_token == "**" { "__" } else { "**" };
            for (emphasis_token, strong_token) in [
                (options.emphasis_token, options.strong_token),
                (alternate_emphasis, options.strong_token),
                (options.emphasis_token, alternate_strong),
                (alternate_emphasis, alternate_strong),
            ] {
                let options = Options {
                    emphasis_token,
                    strong_token,
                    ..options.clone()
                };
                if let Some((renderer, output, definitions)) =
                    self.checked_block(events, next, &options, false, Delimiters::RUNS, &reference_labels)?
                {
                    *self = renderer;
                    return Ok((output, definitions));
                }
            }
            for mode in Delimiters::FALLBACKS {
                for emphasis_token in ['*', '_'] {
                    for strong_token in ["**", "__"] {
                        let options = Options {
                            emphasis_token,
                            strong_token,
                            ..options.clone()
                        };
                        if let Some((renderer, output, definitions)) =
                            self.checked_block(events, next, &options, false, mode, &reference_labels)?
                        {
                            *self = renderer;
                            return Ok((output, definitions));
                        }
                    }
                }
            }
            if options.validate(self.parser_options).is_err() {
                *self = original;
                Ok((output, definitions))
            } else {
                Err(Error::Unrepresentable)
            }
        })();
        if let Ok((_, definitions)) = &result {
            if !definitions.is_empty() {
                // Let the parser normalize whitespace and escapes in labels.
                let parser = Parser::new_ext(definitions, self.parser_options);
                reference_labels.extend(
                    parser
                        .reference_definitions()
                        .iter()
                        .map(|(label, _)| UniCase::new(CowStr::from(label).into_static())),
                );
            }
            references.append(&mut self.references);
        }
        self.references = references;
        self.reference_labels = reference_labels;
        let (mut output, _) = result?;
        if terminal_raw && !self.references.is_empty() {
            // Definitions appended after an EOF-terminated raw block become part
            // of that block. Emit all definitions before this final top-level block.
            let definitions = self.reference_definitions()?;
            let leading_newlines = output.len() - output.trim_start_matches('\n').len();
            output.insert_str(
                leading_newlines,
                &format!("{}\n\n", definitions.trim_start_matches('\n')),
            );
            self.references.clear();
        }
        if output.contains('[') {
            let callback = |link: pulldown_cmark::BrokenLink<'_>| {
                literal_labels.insert(UniCase::new(link.reference.into_static()));
                None
            };
            Parser::new_with_broken_link_callback(&output, self.parser_options, Some(callback)).for_each(drop);
        }
        self.literal_labels = literal_labels;
        self.at_start = false;
        Ok(output)
    }

    pub(crate) fn reserve_list_label(&mut self, events: &[BufferedEvent]) {
        let mut used = self.literal_labels.clone();
        used.extend(self.reference_labels.iter().cloned());
        for event in events {
            if let Event::Start(Tag::Link { id, .. } | Tag::Image { id, .. }) = &event.event {
                used.insert(UniCase::new(id.clone()));
            }
        }
        let mut number = 1;
        loop {
            let label = format!("cmark-loose-{number}");
            if !used.contains(&UniCase::new(CowStr::from(label.as_str()))) {
                self.reference_labels.insert(UniCase::new(CowStr::from(label.clone())));
                self.list_label = Some(label);
                return;
            }
            number += 1;
        }
    }

    fn checked_block(
        &self,
        events: &[BufferedEvent],
        next: Option<&Event<'_>>,
        options: &Options<'_>,
        source_spelling: bool,
        delimiters: Delimiters,
        previous_labels: &HashSet<UniCase<CowStr<'static>>>,
    ) -> Result<Option<(Self, String, String)>, Error> {
        let mut candidate = self.clone();
        let output = candidate.render_block(events, next, options, source_spelling, delimiters)?;
        let definitions = candidate.reference_definitions()?;
        if events_match(
            &output,
            &definitions,
            events,
            previous_labels,
            self.validation_options(),
        ) {
            Ok(Some((candidate, output, definitions)))
        } else {
            Ok(None)
        }
    }

    fn validation_options(&self) -> ParserOptions {
        if self.at_start {
            self.parser_options
        } else {
            self.parser_options.difference(
                ParserOptions::ENABLE_YAML_STYLE_METADATA_BLOCKS
                    | ParserOptions::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS,
            )
        }
    }

    fn render_block(
        &mut self,
        events: &[BufferedEvent],
        next: Option<&Event<'_>>,
        options: &Options<'_>,
        source_spelling: bool,
        delimiters: Delimiters,
    ) -> Result<String, Error> {
        let run_delimiters = delimiters != Delimiters::PREFERRED;
        // Match containers once; all lookahead stays inside this buffered block.
        let mut ends = vec![events.len(); events.len()];
        let mut delimiter_runs = if run_delimiters {
            delimiter_runs(events)
        } else {
            Vec::new()
        };
        let mut prefer_underscore = vec![false; events.len()];
        let mut open = Vec::new();
        let mut emphasis = Vec::new();
        for (i, event) in events.iter().enumerate() {
            match &event.event {
                Event::Start(tag) => {
                    open.push(i);
                    if matches!(tag, Tag::Emphasis) || (run_delimiters && matches!(tag, Tag::Strong)) {
                        emphasis.push(i);
                    }
                }
                Event::End(end) => {
                    let start = open.pop().ok_or(Error::UnexpectedEvent)?;
                    ends[start] = i;
                    if *end == TagEnd::Emphasis || (run_delimiters && *end == TagEnd::Strong) {
                        emphasis.pop();
                        let (before, first, after) = if run_delimiters {
                            (
                                delimiter_runs[start].before,
                                delimiter_runs[start].after,
                                delimiter_runs[i].after,
                            )
                        } else {
                            (
                                inline_edge(events[..start].last().map(|event| &event.event), false),
                                inline_edge(events.get(start + 1).map(|event| &event.event), true),
                                inline_edge(events.get(i + 1).map(|event| &event.event), true),
                            )
                        };
                        let needs_asterisk = !underscore_boundary(before) || !underscore_boundary(after);
                        let can_close = before.is_some_and(|c| !c.is_whitespace())
                            && (!underscore_boundary(before) || underscore_boundary(first));
                        if prefer_underscore[start] || (needs_asterisk && can_close) {
                            if let Some(&parent) = emphasis.last() {
                                prefer_underscore[parent] = true;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        if !open.is_empty() {
            return Err(Error::UnexpectedEvent);
        }
        if run_delimiters {
            for i in (0..events.len()).rev() {
                let width = match events[i].event {
                    Event::Start(Tag::Emphasis) => 1,
                    Event::Start(Tag::Strong) => 2,
                    _ => continue,
                };
                delimiter_runs[i].opening_width = width;
                if i + 1 < events.len()
                    && delimiter_runs[i + 1].opening_width > 0
                    && !(ends[i] == ends[i + 1] + 1 && matches!(events[i + 1].event, Event::Start(Tag::Emphasis)))
                {
                    delimiter_runs[i].opening_width += delimiter_runs[i + 1].opening_width;
                }
            }
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
                    let run_context = run_delimiters.then(|| DelimiterContext {
                        plan: delimiters,
                        literal_token: (delimiters.text == TextSpelling::LiteralEntities)
                            .then_some(
                                delimiter_runs[i]
                                    .literal_before
                                    .or(delimiter_runs[ends[i]].literal_after),
                            )
                            .flatten(),
                        underscore_allowed: underscore_boundary(delimiter_runs[i].before)
                            && underscore_boundary(delimiter_runs[ends[i]].after),
                        opening_width: delimiter_runs[i].opening_width,
                        split_shared: matches!(delimiters.layout, Layout::Shared { split: true, .. })
                            && delimiter_runs[i].width % 3 != 0
                            && (delimiter_runs[i].width + delimiter_runs[ends[i]].width) % 3 == 0
                            && delimiter_runs[ends[i]]
                                .before
                                .is_some_and(|c| !c.is_whitespace() && underscore_boundary(Some(c))),
                        follows_close: matches!(
                            events[..i].last().map(|event| &event.event),
                            Some(Event::End(TagEnd::Emphasis | TagEnd::Strong))
                        ),
                        starts_with_punctuation: delimiter_runs[i]
                            .after
                            .is_some_and(|c| !c.is_whitespace() && underscore_boundary(Some(c))),
                    });
                    self.start(tag, body, sibling, options, prefer_underscore[i], run_context)?;
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
                    let encode_start = delimiters.encode_edges()
                        && matches!(
                            events[..i].last().map(|e| &e.event),
                            Some(Event::End(TagEnd::Emphasis | TagEnd::Strong))
                        );
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
                        let encode_end = delimiters.encode_edges()
                            && matches!(after, Some(Event::Start(Tag::Emphasis | Tag::Strong)));
                        let escaped = self.escape_text(
                            &text,
                            after,
                            options,
                            (encode_start, encode_end),
                            delimiters.literal_runs(),
                        );
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
                    let hyphen_item = matches!(previous, Last::ItemStart)
                        && self.frames.iter().rev().any(|frame| {
                            matches!(
                                frame.content,
                                Content::List {
                                    number: None,
                                    token: '-',
                                    ..
                                }
                            )
                        });
                    self.output
                        .write_str(if matches!(previous, Last::Inline) || hyphen_item {
                            "***"
                        } else {
                            "---"
                        })?;
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
                Event::InlineMath(text) | Event::DisplayMath(text) => {
                    let fence = if matches!(event, Event::InlineMath(_)) {
                        "$"
                    } else {
                        "$$"
                    };
                    let text = if self.in_table() {
                        Cow::Owned(text.replace('|', "\\|"))
                    } else {
                        Cow::Borrowed(text.as_ref())
                    };
                    write!(self.output, "{fence}{text}{fence}")?;
                    self.last = Last::Inline;
                }
            }
            i += 1;
        }
        self.output.marker_runs.clear();
        self.output.marker = None;
        Ok(std::mem::take(&mut self.output.text))
    }

    fn prefix(&mut self) {
        self.output.prefix = self.frames.iter().map(|f| f.prefix.as_str()).collect();
    }

    fn tight_boundary(&self) -> bool {
        match self.frames.as_slice() {
            [.., Frame {
                content: Content::Definition { tight },
                ..
            }] => *tight,
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
        self.raw_eof = false;
        if !matches!(event, Event::Start(Tag::List(_))) {
            self.last_list = None;
        }
        if matches!(event, Event::Start(Tag::List(_)))
            && !matches!(self.last, Last::ItemStart)
            && self.frames.iter().any(|f| matches!(f.tag, Tag::List(_)))
        {
            self.separator = self.separator.max(1);
        }
        if matches!(self.last, Last::DefinitionEnd) && matches!(event, Event::Start(Tag::DefinitionListTitle)) {
            self.separator = self.separator.max(2);
        }
        let block = matches!(event, Event::Start(tag) if is_block(tag.to_end())) || matches!(event, Event::Rule);
        if block && matches!(self.last, Last::Inline) {
            self.separator = self.separator.max(1);
        }
        // Tightness only limits optional spacing. These blocks can absorb their
        // next sibling without a blank line, even when the list itself is tight.
        let paragraph_start = !block
            || matches!(
                event,
                Event::Start(
                    Tag::Paragraph | Tag::DefinitionList | Tag::Table(_) | Tag::CodeBlock(CodeBlockKind::Indented)
                )
            );
        let minimum = match (self.last, event) {
            (Last::BlockEnd(TagEnd::HtmlBlock), _) => 2,
            (Last::BlockEnd(TagEnd::DefinitionList | TagEnd::FootnoteDefinition | TagEnd::Table), _)
                if paragraph_start =>
            {
                2
            }
            (Last::BlockEnd(TagEnd::BlockQuote(_)), Event::Start(Tag::BlockQuote(_))) => 2,
            _ => 0,
        };
        if self.open_html {
            self.quote_end = None;
        } else if self.tight_boundary() {
            self.separator = self.separator.min(1);
            if minimum < 2 && !matches!(event, Event::Start(Tag::Item)) {
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
        self.separator = self.separator.max(minimum);
        if let (Last::ListEnd(tail), Event::Start(tag)) = (self.last, event) {
            if needs_blank_after_list(tag, tail) {
                self.separator = self.separator.max(2);
            }
        }
        if matches!(event, Event::Start(Tag::Item))
            && matches!(
                self.frames.last(),
                Some(Frame {
                    content: Content::List { tight: false, .. },
                    ..
                })
            )
            && matches!(self.last, Last::ItemEnd(_))
        {
            self.separator = self.separator.max(2);
        }
        if self.open_html {
            // An unterminated raw HTML block ended by dedentation. A blank
            // line still belongs to that block, even outside its container.
            self.separator = 1;
            self.open_html = false;
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
        prefer_underscore: bool,
        run_context: Option<DelimiterContext>,
    ) -> Result<(), Error> {
        let mut frame = Frame {
            tag: tag.clone(),
            prefix: String::new(),
            content: Content::Plain,
            start: 0,
            opening_run: None,
        };
        let previous_list = self.last_list.take();
        let at_item_start = self
            .frames
            .iter()
            .rev()
            .take_while(|frame| frame.start == self.output.text.len())
            .any(|frame| matches!(frame.tag, Tag::Item));
        match tag {
            Tag::List(number) => {
                let mut depth = 0;
                // Only unwrapped inline content proves a list is tight. A list
                // containing only blocks can still need blank block separators.
                let mut tight = false;
                let mut paragraphs = 0;
                let mut blocks = 0;
                let mut items = 0;
                for item in body {
                    match &item.event {
                        Event::Start(tag) => {
                            if depth == 0 && matches!(tag, Tag::Item) {
                                items += 1;
                            }
                            if depth == 1 && is_block(tag.to_end()) {
                                blocks += 1;
                                if matches!(tag, Tag::Paragraph) {
                                    paragraphs += 1;
                                }
                            }
                            if depth == 1 && !is_block(tag.to_end()) {
                                tight = true;
                            }
                            depth += 1;
                        }
                        Event::End(_) => depth -= 1,
                        Event::Rule => {}
                        _ if depth == 1 => tight = true,
                        _ => {}
                    }
                }
                let mut token = if number.is_some() {
                    options.ordered_list_token
                } else {
                    options.list_token
                };
                if at_item_start
                    && number.is_none()
                    && self.frames.iter().rev().find_map(|frame| match frame.content {
                        Content::List {
                            number: None, token, ..
                        } => Some(token),
                        _ => None,
                    }) == Some(token)
                {
                    token = if token == '*' { '-' } else { '*' };
                }
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
                    tight: tight && paragraphs == 0,
                    wide: matches!(sibling, Some(Event::Start(Tag::CodeBlock(CodeBlockKind::Indented)))),
                    placeholder: items == 1 && paragraphs == 1 && blocks == 1,
                };
            }
            Tag::Item => {
                let empty_nested = body.is_empty()
                    && self
                        .frames
                        .iter()
                        .filter(|frame| matches!(frame.tag, Tag::List(_)))
                        .count()
                        > 1;
                let Some(Frame {
                    content:
                        Content::List {
                            number,
                            token,
                            wide,
                            placeholder,
                            ..
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
                            *number = number.saturating_add(1).min(999_999_999);
                        }
                        marker
                    }
                    None => format!("{leading}{token}{space}"),
                };
                frame.prefix = " ".repeat(marker.len());
                self.output.write_str(&marker)?;
                if *placeholder || empty_nested {
                    if let Some(label) = &self.list_label {
                        write!(self.output, "[{label}]: <>")?;
                        self.separator = if empty_nested { 1 } else { 2 };
                    }
                }
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
                if self.output.newlines == 0 && !at_item_start {
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
                self.output
                    .write_str(if at_item_start { first.trim_start() } else { first })?;
                frame.prefix = " > ".into();
                self.separator = usize::from(!at_item_start || kind.is_some());
            }
            Tag::CodeBlock(CodeBlockKind::Indented) => {
                if at_item_start {
                    self.output.write_str("    ")?;
                } else if self.output.newlines == 0 {
                    self.output.newline()?;
                }
                frame.prefix = "    ".into();
            }
            Tag::CodeBlock(CodeBlockKind::Fenced(info)) => {
                if self.output.newlines == 0 && !at_item_start {
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
            Tag::HtmlBlock => {
                frame.content = Content::Html {
                    blank_terminated: html_block_needs_blank(body),
                    closed: html_block_closed(body),
                };
            }
            Tag::Emphasis | Tag::Strong => {
                let mut token = if matches!(tag, Tag::Emphasis) {
                    options.emphasis_token.to_string()
                } else {
                    options.strong_token.to_owned()
                };
                if run_context.is_some_and(|context| context.plan.layout == Layout::Uniform) {
                    self.output.write_str(&token)?;
                    frame.content = Content::Delimited(token);
                    frame.start = self.output.text.len();
                    frame.opening_run = self.output.marker.map(|_| self.output.marker_runs.len() - 1);
                    self.frames.push(frame);
                    return Ok(());
                }
                if run_context.is_some_and(|context| context.plan.shared_runs())
                    && matches!(token.as_str(), "*" | "_" | "**" | "__")
                {
                    let parent = self
                        .frames
                        .iter()
                        .rev()
                        .find(|f| matches!(f.tag, Tag::Emphasis | Tag::Strong));
                    let marker = if let Some(marker) = run_context.and_then(|context| context.literal_token) {
                        marker
                    } else if run_context.is_some_and(|context| context.follows_close) {
                        let previous = self.output.text.chars().next_back().unwrap();
                        let closing_width = self.output.marker_runs.last().copied().unwrap_or(0);
                        if run_context.is_some_and(|context| {
                            matches!(context.plan.layout, Layout::Shared { adjacent: true, .. })
                                || (context.plan.layout == Layout::RuleThree
                                    && (closing_width % 3 != context.opening_width % 3 || closing_width % 3 == 0))
                        }) {
                            previous
                        } else if previous == '*' {
                            '_'
                        } else {
                            '*'
                        }
                    } else if let Some(Frame {
                        content: Content::Delimited(parent_token),
                        start,
                        opening_run,
                        tag: parent_tag,
                        ..
                    }) = parent
                    {
                        let same_start = *start == self.output.text.len();
                        let same_end = matches!(sibling, Some(Event::End(end)) if *end == parent_tag.to_end());
                        let parent_marker = parent_token.chars().next().unwrap();
                        let parent_width = opening_run.map_or(0, |run| self.output.marker_runs[run]);
                        let same = same_start
                            && (same_end && matches!(tag, Tag::Strong)
                                || !same_end && !run_context.is_some_and(|context| context.split_shared))
                            || !same_start
                                && run_context.is_some_and(|context| {
                                    context.plan.layout == Layout::RuleThree
                                        && parent_width % 3 != 0
                                        && (parent_width + context.opening_width) % 3 == 0
                                })
                            || same_end
                                && !same_start
                                && run_context.is_some_and(|context| {
                                    matches!(context.plan.layout, Layout::Shared { ends: true, .. })
                                });
                        if same {
                            parent_marker
                        } else if parent_marker == '*' {
                            '_'
                        } else {
                            '*'
                        }
                    } else {
                        token.chars().next().unwrap()
                    };
                    token = marker.to_string().repeat(token.len());
                    self.output.write_str(&token)?;
                    frame.content = Content::Delimited(token);
                    frame.start = self.output.text.len();
                    frame.opening_run = self.output.marker.map(|_| self.output.marker_runs.len() - 1);
                    self.frames.push(frame);
                    return Ok(());
                }
                let planned = run_context.is_some();
                let underscore_allowed = run_context.map_or_else(
                    || {
                        underscore_boundary(self.output.text.chars().next_back())
                            && underscore_boundary(inline_edge(sibling, true))
                    },
                    |context| context.underscore_allowed,
                );
                let can_change = if planned {
                    matches!(token.as_str(), "*" | "_" | "**" | "__")
                } else {
                    matches!(tag, Tag::Emphasis)
                };
                let width = if matches!(tag, Tag::Emphasis) { 1 } else { token.len() };
                if planned && can_change && !underscore_allowed {
                    token = "*".repeat(width);
                } else if can_change && run_context.is_some_and(|context| context.follows_close) {
                    // Adjacent spans need distinct closing and opening runs.
                    if self.output.text.ends_with('_') {
                        token = "*".repeat(width);
                    } else if underscore_allowed {
                        token = "_".repeat(width);
                    }
                } else if can_change && prefer_underscore && underscore_allowed {
                    token = "_".repeat(width);
                } else if can_change
                    && self
                        .frames
                        .iter()
                        .rev()
                        .find(|frame| {
                            matches!(frame.tag, Tag::Emphasis) || (planned && matches!(frame.tag, Tag::Strong))
                        })
                        .is_some_and(|frame| match &frame.content {
                            Content::Delimited(parent) if planned => {
                                let same_start = frame.start == self.output.text.len();
                                let same_end = matches!(sibling, Some(Event::End(end)) if *end == frame.tag.to_end());
                                // Keep a shared opening or closing run intact.
                                // Sharing both would merge emphasis into strong.
                                parent.as_bytes().first() == token.as_bytes().first()
                                    && (same_start == same_end
                                        || run_context.is_some_and(|context| context.starts_with_punctuation))
                            }
                            Content::Delimited(parent) => parent == &token,
                            _ => false,
                        })
                    && (!token.starts_with('*') || underscore_allowed)
                {
                    token = if token.starts_with('*') { "_" } else { "*" }.repeat(width);
                }
                self.output.write_str(&token)?;
                frame.content = Content::Delimited(token);
            }
            Tag::Strikethrough | Tag::Superscript | Tag::Subscript => {
                let single_strike = !self.parser_options.contains(ParserOptions::ENABLE_SUBSCRIPT)
                    && run_context.is_some()
                    && self
                        .frames
                        .iter()
                        .rev()
                        .find(|frame| matches!(frame.tag, Tag::Strikethrough))
                        .is_some_and(|frame| matches!(&frame.content, Content::Delimited(token) if token == "~~"));
                let (start, end) = match tag {
                    Tag::Strikethrough if single_strike => ("~", "~"),
                    Tag::Strikethrough => ("~~", "~~"),
                    Tag::Superscript if options.use_html_for_super_sub_script => ("<sup>", "</sup>"),
                    Tag::Subscript if options.use_html_for_super_sub_script => ("<sub>", "</sub>"),
                    Tag::Superscript if run_context.is_some_and(|context| context.plan.layout == Layout::Uniform) => {
                        ("^^", "^^")
                    }
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
                        && label_matches(id, body, matches!(tag, Tag::Image { .. }), self.parser_options),
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
                if self.output.newlines == 0 && !self.output.text.is_empty() && !at_item_start {
                    self.output.newline()?;
                }
            }
            Tag::DefinitionListDefinition => {
                let mut depth = 0;
                let mut paragraphs = false;
                for event in body {
                    match event.event {
                        Event::Start(ref tag) => {
                            paragraphs |= depth == 0 && matches!(tag, Tag::Paragraph);
                            depth += 1;
                        }
                        Event::End(_) => depth -= 1,
                        _ => {}
                    }
                }
                frame.content = Content::Definition { tight: !paragraphs };
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
        if matches!(tag, Tag::Emphasis | Tag::Strong) {
            frame.opening_run = self.output.marker.map(|_| self.output.marker_runs.len() - 1);
        }
        let changes_prefix = !frame.prefix.is_empty();
        self.frames.push(frame);
        if changes_prefix {
            self.prefix();
        }
        Ok(())
    }

    fn end(&mut self, end: TagEnd, previous: Last, options: &Options<'_>) -> Result<(), Error> {
        let frame = self.frames.pop().ok_or(Error::UnexpectedEvent)?;
        if frame.tag.to_end() != end {
            return Err(Error::UnexpectedEvent);
        }
        if !frame.prefix.is_empty() {
            self.prefix();
        }
        if is_block(end) {
            self.last = Last::BlockEnd(end);
        }
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
                self.raw_eof = self.output.newlines == 0;
                if !self.raw_eof {
                    if let Content::Delimited(fence) = frame.content {
                        self.output.write_str(&fence)?;
                    }
                }
                self.separator = options.newlines_after_codeblock.max(2);
            }
            Tag::HtmlBlock => {
                self.raw_eof = self.output.newlines == 0;
                self.open_html = matches!(
                    frame.content,
                    Content::Html {
                        blank_terminated: false,
                        closed: false
                    }
                );
                self.separator = options.newlines_after_htmlblock.max(2);
                if matches!(
                    frame.content,
                    Content::Html {
                        blank_terminated: false,
                        ..
                    }
                ) {
                    self.last = Last::Other;
                }
            }
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
                // A blank line terminates a table, regardless of preferred spacing.
                self.separator = 1;
            }
            Tag::DefinitionList => self.separator = options.newlines_after_list.max(2),
            Tag::DefinitionListTitle => self.separator = 1,
            Tag::DefinitionListDefinition => {
                // Terminate a quote before writing outside the definition. A later
                // quote marker would start a new quote instead of closing this one.
                // Open raw HTML exits by dedentation; an empty quote line adds HTML.
                if let Some(line) = self.quote_end.take().filter(|_| !self.open_html) {
                    self.output.separate(1)?;
                    self.output.prefix.clear();
                    self.output.write_str(&line)?;
                    self.prefix();
                }
                if !self.raw_eof {
                    self.output.separate(1)?;
                }
                self.separator = 1;
                self.last = Last::DefinitionEnd;
            }
            Tag::MetadataBlock(kind) => {
                self.output.write_str(metadata_fence(kind))?;
                self.output.newline()?;
                self.separator = options.newlines_after_metadata.saturating_add(1);
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

    fn escape_text(
        &self,
        text: &str,
        next: Option<&Event<'_>>,
        options: &Options<'_>,
        encode_edges: (bool, bool),
        literal_runs: bool,
    ) -> String {
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
            if literal_runs && matches!(c, '*' | '_' | '~' | '^' | '$') {
                escaped.push(c);
                continue;
            }
            if (i == 0 && encode_edges.0 || i + c.len_utf8() == text.len() && encode_edges.1) && !c.is_whitespace() {
                write!(escaped, "&#{};", c as u32).expect("String writes cannot fail");
                continue;
            }
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
                        || (c == '^'
                            && (self.parser_options.contains(ParserOptions::ENABLE_SUPERSCRIPT)
                                || !text[..i].chars().next_back().is_some_and(char::is_alphanumeric)))
                        || (c == '$' && self.parser_options.contains(ParserOptions::ENABLE_MATH))
                        || (c == ':'
                            && i == 0
                            && line_start
                            && marker_end
                            && self.parser_options.contains(ParserOptions::ENABLE_DEFINITION_LIST))
                        || (c == '{'
                            && heading
                            && self.parser_options.contains(ParserOptions::ENABLE_HEADING_ATTRIBUTES))
                        || (c == '_'
                            && !(text[..i].chars().next_back().is_some_and(char::is_alphanumeric)
                                && text[i + 1..].chars().next().is_some_and(char::is_alphanumeric)))
                        || (c == '|'
                            && (i == 0
                                || self.in_table()
                                || self.parser_options.contains(ParserOptions::ENABLE_TABLES)))
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

    fn reference_definitions(&self) -> Result<String, Error> {
        let mut output = self.output.clone();
        if !self.references.is_empty() {
            output.prefix.clear();
            output.separate(2)?;
            let mut seen = HashSet::new();
            let mut first = true;
            for reference in &self.references {
                if !seen.insert(reference) {
                    continue;
                }
                if !first {
                    output.newline()?;
                }
                first = false;
                write!(output, "[{}", reference.0)?;
                close_link(&reference.1, &reference.2, &mut output, LinkType::Shortcut)?;
            }
        }
        Ok(output.text)
    }

    pub(crate) fn finish(&mut self) -> Result<String, Error> {
        let output = self.reference_definitions()?;
        self.output.text.clear();
        self.references.clear();
        self.reference_labels.clear();
        Ok(output)
    }
}

// A single paragraph in a single item cannot make a list loose by itself.
// The input had an invisible reference definition, so retain that structure
// with a fresh definition whose label is chosen after all remaining input.
pub(crate) fn needs_list_placeholder(events: &[BufferedEvent]) -> bool {
    let mut open = Vec::new();
    for (end, event) in events.iter().enumerate() {
        match event.event {
            Event::Start(_) => open.push(end),
            Event::End(tag) => {
                let Some(start) = open.pop() else { return false };
                if tag == TagEnd::Item
                    && end == start + 1
                    && start >= 2
                    && matches!(events[start - 1].event, Event::Start(Tag::List(_)))
                    && match &events[start - 2].event {
                        Event::End(tag) => !is_block(*tag),
                        Event::Start(_) | Event::Rule | Event::Html(_) => false,
                        _ => true,
                    }
                {
                    return true;
                }
                if tag == TagEnd::Paragraph
                    && start >= 2
                    && matches!(events[start - 2].event, Event::Start(Tag::List(_)))
                    && matches!(events[start - 1].event, Event::Start(Tag::Item))
                    && matches!(events.get(end + 1).map(|e| &e.event), Some(Event::End(TagEnd::Item)))
                    && matches!(events.get(end + 2).map(|e| &e.event), Some(Event::End(TagEnd::List(_))))
                {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

fn metadata_fence(kind: MetadataBlockKind) -> &'static str {
    match kind {
        MetadataBlockKind::YamlStyle => "---",
        MetadataBlockKind::PlusesStyle => "+++",
    }
}

fn underscore_boundary(character: Option<char>) -> bool {
    character.map_or(true, |c| {
        if c.is_ascii() {
            c.is_ascii_whitespace() || c.is_ascii_punctuation()
        } else {
            // Use the parser's Unicode punctuation rules on a constant-size
            // probe: an underscore cannot open after (or close before) a word.
            pulldown_cmark::Parser::new(&format!("{c}_x_")).any(|event| matches!(event, Event::Start(Tag::Emphasis)))
        }
    })
}

fn html_block_needs_blank(body: &[BufferedEvent]) -> bool {
    let Some(html) = body.iter().find_map(|event| match &event.event {
        Event::Html(html) => Some(html),
        _ => None,
    }) else {
        return false;
    };
    let html = html.trim_start();
    if html.starts_with("<!") || html.starts_with("<?") {
        return false;
    }
    let tag = html
        .strip_prefix('<')
        .unwrap_or(html)
        .split(|c: char| c.is_ascii_whitespace() || c == '>')
        .next()
        .unwrap_or_default();
    !["pre", "script", "style", "textarea"]
        .iter()
        .any(|name| tag.eq_ignore_ascii_case(name))
}

fn html_block_closed(body: &[BufferedEvent]) -> bool {
    let text: String = body
        .iter()
        .filter_map(|event| match &event.event {
            Event::Html(text) => Some(text.as_ref()),
            _ => None,
        })
        .collect();
    let text = text.trim_start();
    let terminator = if text.starts_with("<!--") {
        "-->"
    } else if text.starts_with("<?") {
        "?>"
    } else if text.starts_with("<![CDATA[") {
        "]]>"
    } else if text.starts_with("<!") {
        ">"
    } else {
        let text = text.to_ascii_lowercase();
        return ["pre", "script", "style", "textarea"]
            .iter()
            .any(|tag| text.contains(&format!("</{tag}>")));
    };
    text.contains(terminator)
}

#[derive(Clone, Copy, Default)]
struct DelimiterRun {
    before: Option<char>,
    after: Option<char>,
    literal_before: Option<char>,
    literal_after: Option<char>,
    width: usize,
    opening_width: usize,
}

fn delimiter_runs(events: &[BufferedEvent]) -> Vec<DelimiterRun> {
    let mut runs = vec![DelimiterRun::default(); events.len()];
    let is_delimiter = |event: &Event<'_>| {
        matches!(
            event,
            Event::Start(Tag::Emphasis | Tag::Strong) | Event::End(TagEnd::Emphasis | TagEnd::Strong)
        )
    };
    let mut start = 0;
    while start < events.len() {
        if !is_delimiter(&events[start].event) {
            start += 1;
            continue;
        }
        // Flanking depends on the characters outside the whole run. Keep
        // opening and closing runs separate so adjacent spans can alternate.
        let mut end = start + 1;
        while end < events.len()
            && is_delimiter(&events[end].event)
            && matches!(events[start].event, Event::Start(_)) == matches!(events[end].event, Event::Start(_))
        {
            end += 1;
        }
        let before = inline_edge(events[..start].last().map(|event| &event.event), false);
        let after = inline_edge(events.get(end).map(|event| &event.event), true);
        let before_literal = match events[..start].last().map(|e| &e.event) {
            Some(Event::Text(text)) => text.chars().next_back().filter(|c| matches!(c, '*' | '_')),
            _ => None,
        };
        let after_literal = match events.get(end).map(|e| &e.event) {
            Some(Event::Text(text)) => text.chars().next().filter(|c| matches!(c, '*' | '_')),
            _ => None,
        };
        let width = events[start..end]
            .iter()
            .map(|event| match event.event {
                Event::Start(Tag::Strong) | Event::End(TagEnd::Strong) => 2,
                _ => 1,
            })
            .sum();
        runs[start..end].fill(DelimiterRun {
            before,
            after,
            literal_before: before_literal,
            literal_after: after_literal,
            width,
            opening_width: 0,
        });
        start = end;
    }
    runs
}

fn inline_edge(event: Option<&Event<'_>>, first: bool) -> Option<char> {
    let text = match event? {
        Event::Text(text) | Event::InlineHtml(text) => text,
        Event::SoftBreak | Event::HardBreak => return Some('\n'),
        Event::Start(tag) if is_block(tag.to_end()) => return None,
        Event::End(tag) if is_block(*tag) => return None,
        // Inline markup starts and ends with punctuation.
        _ => return Some('*'),
    };
    if first {
        text.chars().next()
    } else {
        text.chars().next_back()
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
fn label_matches(id: &str, body: &[BufferedEvent], image: bool, parser_options: ParserOptions) -> bool {
    use pulldown_cmark::utils::TextMergeStream;
    let markdown = format!("{}[{id}](cmark-label)", if image { "!" } else { "" });
    let expected: Vec<_> = TextMergeStream::new(body.iter().map(|e| e.event.clone())).collect();
    {
        let parsed: Vec<_> = TextMergeStream::new(Parser::new_ext(&markdown, parser_options)).collect();
        matches!(parsed.first(), Some(Event::Start(Tag::Paragraph)))
            && if image {
                matches!(parsed.get(1), Some(Event::Start(Tag::Image { .. })))
            } else {
                matches!(parsed.get(1), Some(Event::Start(Tag::Link { .. })))
            }
            && matches!(parsed.last(), Some(Event::End(TagEnd::Paragraph)))
            && parsed.len() >= 4
            && parsed[2..parsed.len() - 2] == expected
    }
}

// Check source spelling and delimiter choices after rendering the whole block,
// because new indentation or surrounding delimiters can change their meaning.
fn events_match(
    output: &str,
    definitions: &str,
    expected: &[BufferedEvent],
    previous_labels: &HashSet<UniCase<CowStr<'static>>>,
    parser_options: ParserOptions,
) -> bool {
    use pulldown_cmark::{utils::TextMergeStream, BrokenLink};
    // Leading blank lines disable metadata recognition, so only prefix a
    // separator when this block actually has reference definitions.
    let markdown = if definitions.is_empty() {
        Cow::Borrowed(output)
    } else {
        Cow::Owned(format!("{definitions}\n\n{output}"))
    };
    let mut expected: Vec<_> = TextMergeStream::new(expected.iter().map(|e| e.event.clone())).collect();
    if !expected.is_empty()
        && expected.iter().all(|event| match event {
            Event::Start(tag) => !is_block(tag.to_end()),
            Event::End(tag) => !is_block(*tag),
            Event::Rule | Event::Html(_) => false,
            _ => true,
        })
    {
        // The API also accepts inline fragments, whose surrounding paragraph
        // belongs to the caller rather than the supplied event stream.
        expected.insert(0, Event::Start(Tag::Paragraph));
        expected.push(Event::End(TagEnd::Paragraph));
    }
    {
        // Definitions needed by this block are in `markdown`. An otherwise
        // unresolved label can still become a link through an earlier block.
        let mut resolves_earlier = false;
        let callback = |link: BrokenLink<'_>| {
            resolves_earlier |= previous_labels.contains(&UniCase::new(link.reference));
            None
        };
        let parser = Parser::new_with_broken_link_callback(&markdown, parser_options, Some(callback));
        let matches = TextMergeStream::new(parser).eq(expected.iter().cloned());
        matches && !resolves_earlier
    }
}
