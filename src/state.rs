use super::{Borrow, Error, Event, Options, Range, Renderer, TagEnd};
use pulldown_cmark::CowStr;
use std::fmt;

/// Input accepted and output written by a successful serializer call.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Progress {
    /// Number of input events consumed by this call. Finishing consumes none.
    pub events_consumed: usize,
    /// Number of UTF-8 bytes written to the writer by this call.
    pub bytes_written: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Status {
    Active,
    Finished,
    Failed,
}

#[derive(Clone, Debug)]
pub(crate) struct BufferedEvent {
    pub event: Event<'static>,
    pub source: Option<CowStr<'static>>,
}

/// An incremental Markdown serializer that owns its configuration and retained input.
///
/// Calls may buffer an open block, or a completed block whose next sibling affects
/// its syntax. Always call [`finish`](Self::finish) after the last input event.
/// Use the same logical output stream for every call. After any error the state
/// is unusable, since a [`fmt::Write`] failure can leave partial output behind.
///
/// ```
/// use pulldown_cmark::Parser;
/// use pulldown_cmark_to_cmark::{Options, State};
///
/// let events: Vec<_> = Parser::new("a *b*").collect();
/// let mut state = State::new(Options::default());
/// let mut output = String::new();
/// let first = state.process(&events[..2], &mut output)?;
/// assert_eq!(first.events_consumed, 2);
/// state.process(&events[2..], &mut output)?;
/// state.finish(&mut output)?;
/// assert_eq!(output, "a *b*");
/// # Ok::<(), pulldown_cmark_to_cmark::Error>(())
/// ```
#[derive(Clone, Debug)]
pub struct State {
    options: Options<'static>,
    strong_token: String,
    renderer: Renderer,
    pending: Vec<BufferedEvent>,
    open: Vec<TagEnd>,
    status: Status,
    last_source_end: usize,
}

impl Default for State {
    fn default() -> Self {
        Self::new(Options::default())
    }
}

impl State {
    /// Create a serializer, copying any borrowed configuration strings.
    pub fn new(options: Options<'_>) -> Self {
        Self {
            strong_token: options.strong_token.to_owned(),
            options: Options {
                strong_token: "",
                ..options
            },
            renderer: Renderer::default(),
            pending: Vec::new(),
            open: Vec::new(),
            status: Status::Active,
            last_source_end: 0,
        }
    }

    /// Accept events and write any output whose syntax is already determined.
    ///
    /// A successful call consumes all supplied events. `bytes_written` can be zero
    /// while a construct is buffered. Input events need not outlive this call.
    pub fn process<'a, I, E, F>(&mut self, events: I, writer: F) -> Result<Progress, Error>
    where
        I: IntoIterator<Item = E>,
        E: Borrow<Event<'a>>,
        F: fmt::Write,
    {
        self.process_inner(events.into_iter().map(|event| (event, None)), None, writer)
    }

    /// Accept events with optional byte ranges into `source`.
    ///
    /// Original text spelling is retained where it safely represents the supplied
    /// events. Edited text and missing ranges use the ordinary renderer. Ranges
    /// must be in bounds and on UTF-8 character boundaries. The source and events
    /// need not outlive this call.
    ///
    /// ```
    /// use pulldown_cmark::Parser;
    /// use pulldown_cmark_to_cmark::State;
    ///
    /// let source = "a < b &amp; c";
    /// let events = Parser::new(source).into_offset_iter()
    ///     .map(|(event, range)| (event, Some(range)));
    /// let mut state = State::default();
    /// let mut output = String::new();
    /// state.process_with_source_range(events, source, &mut output)?;
    /// state.finish(&mut output)?;
    /// assert_eq!(output, source);
    /// # Ok::<(), pulldown_cmark_to_cmark::Error>(())
    /// ```
    pub fn process_with_source_range<'a, I, E, F>(
        &mut self,
        events_and_ranges: I,
        source: &str,
        writer: F,
    ) -> Result<Progress, Error>
    where
        I: IntoIterator<Item = (E, Option<Range<usize>>)>,
        E: Borrow<Event<'a>>,
        F: fmt::Write,
    {
        self.process_inner(events_and_ranges, Some(source), writer)
    }

    fn process_inner<'a, I, E, F>(&mut self, events: I, source: Option<&str>, mut writer: F) -> Result<Progress, Error>
    where
        I: IntoIterator<Item = (E, Option<Range<usize>>)>,
        E: Borrow<Event<'a>>,
        F: fmt::Write,
    {
        let result = (|| {
            self.require_active()?;
            let mut progress = Progress::default();
            for (event, range) in events {
                let event = event.borrow();
                let original = match (source, range) {
                    (Some(source), Some(range)) => {
                        let mut start = range.start;
                        source.get(range.clone()).ok_or(Error::InvalidSourceRange)?;
                        if matches!(event, Event::Text(_))
                            && start > self.last_source_end
                            && source.as_bytes().get(start - 1) == Some(&b'\\')
                        {
                            start -= 1;
                        }
                        if !matches!(event, Event::Start(_)) {
                            self.last_source_end = range.end;
                        }
                        matches!(event, Event::Text(_)).then(|| CowStr::from(&source[start..range.end]).into_static())
                    }
                    _ => None,
                };
                if self.open.is_empty() && !self.pending.is_empty() {
                    progress.bytes_written += self.flush(Some(event), &mut writer)?;
                }
                match event {
                    Event::Start(tag) => self.open.push(tag.to_end()),
                    Event::End(end) if self.open.pop() != Some(*end) => return Err(Error::UnexpectedEvent),
                    _ => {}
                }
                self.pending.push(BufferedEvent {
                    event: event.clone().into_static(),
                    source: original,
                });
                progress.events_consumed += 1;
            }
            Ok(progress)
        })();
        if result.is_err() {
            self.status = Status::Failed;
        }
        result
    }

    fn require_active(&self) -> Result<(), Error> {
        match self.status {
            Status::Active => Ok(()),
            Status::Finished => Err(Error::Finished),
            Status::Failed => Err(Error::Failed),
        }
    }

    fn flush(&mut self, next: Option<&Event<'_>>, writer: &mut impl fmt::Write) -> Result<usize, Error> {
        let options = Options {
            strong_token: &self.strong_token,
            ..self.options.clone()
        };
        let output = self.renderer.block(&self.pending, next, &options)?;
        if !output.is_empty() {
            writer.write_str(&output)?;
        }
        self.pending.clear();
        Ok(output.len())
    }

    /// Flush retained events and reference definitions, consuming no input events.
    ///
    /// Calling this again after success writes nothing and returns zero progress.
    /// Unclosed containers cause [`Error::UnexpectedEvent`].
    pub fn finish(&mut self, mut writer: impl fmt::Write) -> Result<Progress, Error> {
        if self.status == Status::Finished {
            return Ok(Progress::default());
        }
        let result = (|| {
            self.require_active()?;
            if !self.open.is_empty() {
                return Err(Error::UnexpectedEvent);
            }
            let mut bytes_written = self.flush(None, &mut writer)?;
            let output = self.renderer.finish()?;
            if !output.is_empty() {
                writer.write_str(&output)?;
            }
            bytes_written += output.len();
            Ok(Progress {
                events_consumed: 0,
                bytes_written,
            })
        })();
        self.status = if result.is_ok() {
            Status::Finished
        } else {
            Status::Failed
        };
        result
    }
}
