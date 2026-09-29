use std::fmt;

use pulldown_cmark::{utils::TextMergeStream, Event, LinkType, Parser, Tag, TagEnd};
use pulldown_cmark_to_cmark::{cmark, cmark_with_source_range, Error, Options, Progress, State};

#[test]
fn owns_options_and_events_and_reports_utf8_bytes() {
    let mut state = {
        let token = String::from("__");
        State::new(
            pulldown_cmark::Options::empty(),
            Options {
                strong_token: &token,
                ..Options::default()
            },
        )
    };
    let mut output = String::new();
    {
        let input = String::from("**é🌻**");
        let events: Vec<_> = Parser::new(&input).collect();
        assert_eq!(
            state.process(&events, &mut output).unwrap(),
            Progress {
                events_consumed: events.len(),
                bytes_written: 0
            }
        );
    }
    assert!(output.is_empty());
    assert_eq!(
        state.finish(&mut output).unwrap(),
        Progress {
            events_consumed: 0,
            bytes_written: "__é🌻__".len()
        }
    );
    assert_eq!(output, "__é🌻__");
    assert_eq!(state.finish(&mut output).unwrap(), Progress::default());
}

#[test]
fn completed_blocks_are_written_before_the_document_finishes() {
    let mut state = State::new(pulldown_cmark::Options::empty(), Default::default());
    let mut output = String::new();
    state.process(Parser::new("first"), &mut output).unwrap();
    let progress = state.process([Event::Start(Tag::Paragraph)], &mut output).unwrap();
    assert_eq!(
        progress,
        Progress {
            events_consumed: 1,
            bytes_written: 5
        }
    );
    assert_eq!(output, "first");
    state
        .process(
            [Event::Text("second".into()), Event::End(TagEnd::Paragraph)],
            &mut output,
        )
        .unwrap();
    state.finish(&mut output).unwrap();
    assert_eq!(output, "first\n\nsecond");
}

#[test]
fn reference_definitions_are_flushed_once() {
    let mut state = State::new(pulldown_cmark::Options::empty(), Default::default());
    let mut output = String::new();
    state.process(Parser::new("[é] [é]\n\n[é]: /url"), &mut output).unwrap();
    state.process(Parser::new("last"), &mut output).unwrap();
    assert_eq!(output, "[é] [é]");
    let before = output.len();
    assert_eq!(
        state.finish(&mut output).unwrap(),
        Progress {
            events_consumed: 0,
            bytes_written: "\n\nlast\n\n[é]: /url".len()
        }
    );
    assert_eq!(output, "[é] [é]\n\nlast\n\n[é]: /url");
    assert!(output.len() > before);
    assert_eq!(state.finish(&mut output).unwrap(), Progress::default());
}

#[test]
fn unrepresentable_delimiters_fail_without_writing_a_changed_block() {
    let mut state = State::new(pulldown_cmark::Options::empty(), Options::default());
    let mut output = String::new();
    state
        .process(
            [
                Event::Start(Tag::Paragraph),
                Event::Start(Tag::Emphasis),
                Event::End(TagEnd::Emphasis),
                Event::End(TagEnd::Paragraph),
            ],
            &mut output,
        )
        .unwrap();
    assert!(matches!(state.finish(&mut output), Err(Error::Unrepresentable)));
    assert!(output.is_empty());
    assert!(matches!(state.finish(&mut output), Err(Error::Failed)));
}

#[test]
fn source_spelling_checks_previously_defined_references() {
    for (definition, label) in [
        ("link", "LINK"),
        ("straße", "STRASSE"),
        ("two words", "two\twords"),
        (r"a\]b", r"a\]b"),
        ("link", "unrelated"),
    ] {
        let first = format!("[{definition}]\n\n[{definition}]: /url");
        // This independently parsed chunk contains literal reference syntax.
        let source = format!("A [{label}] < 2");
        let expected: Vec<_> = TextMergeStream::new(Parser::new(&first).chain(Parser::new(&source))).collect();
        let mut state = State::new(pulldown_cmark::Options::empty(), Default::default());
        let mut output = String::new();
        state.process(Parser::new(&first), &mut output).unwrap();
        state
            .process_with_source_range(
                Parser::new(&source)
                    .into_offset_iter()
                    .map(|(event, range)| (event, Some(range))),
                &source,
                &mut output,
            )
            .unwrap();
        state.finish(&mut output).unwrap();
        assert_eq!(
            expected,
            TextMergeStream::new(Parser::new(&output)).collect::<Vec<_>>(),
            "{output:?}"
        );
        if label == "unrelated" {
            assert!(
                output.contains(&source),
                "safe source spelling should be retained: {:?}",
                output
            );
        }
    }
}

#[test]
fn source_input_can_be_dropped_and_mixed_with_ordinary_events() {
    let mut state = State::new(pulldown_cmark::Options::empty(), Default::default());
    let mut output = String::new();
    {
        let source = String::from("a < b &amp; c");
        let events = Parser::new(&source)
            .into_offset_iter()
            .map(|(event, range)| (event, Some(range)));
        state.process_with_source_range(events, &source, &mut output).unwrap();
    }
    state.process(Parser::new("next"), &mut output).unwrap();
    state.finish(&mut output).unwrap();
    assert_eq!(output, "a < b &amp; c\n\nnext");
}

#[test]
fn edited_shortcut_labels_keep_the_edit_and_reference_target() {
    for input in ["[old]\n\n[old]: /url", "![old][]\n\n[old]: /url"] {
        let events: Vec<_> = Parser::new(input)
            .into_offset_iter()
            .map(|(event, range)| {
                (
                    if matches!(event, Event::Text(_)) {
                        Event::Text("new".into())
                    } else {
                        event
                    },
                    Some(range),
                )
            })
            .collect();
        for source in [false, true] {
            let mut output = String::new();
            if source {
                cmark_with_source_range(events.clone(), input, &mut output, pulldown_cmark::Options::empty()).unwrap();
            } else {
                cmark(
                    events.iter().map(|(event, _)| event),
                    &mut output,
                    pulldown_cmark::Options::empty(),
                )
                .unwrap();
            }
            assert!(output.contains("[new][old]"), "{}", output);
            assert!(Parser::new(&output).any(|event| matches!(
                event,
                Event::Start(
                    Tag::Link {
                        link_type: LinkType::Reference,
                        ..
                    } | Tag::Image {
                        link_type: LinkType::Reference,
                        ..
                    }
                )
            )));
        }
    }
}

struct RefuseWrites;

impl fmt::Write for RefuseWrites {
    fn write_str(&mut self, _: &str) -> fmt::Result {
        Err(fmt::Error)
    }
}

#[test]
fn empty_calls_and_repeated_finish_do_not_touch_the_writer() {
    let mut state = State::new(pulldown_cmark::Options::empty(), Default::default());
    assert_eq!(
        state.process(std::iter::empty::<Event<'_>>(), RefuseWrites).unwrap(),
        Progress::default()
    );
    assert_eq!(state.finish(RefuseWrites).unwrap(), Progress::default());
    assert_eq!(state.finish(RefuseWrites).unwrap(), Progress::default());
}

#[test]
fn errors_are_terminal() {
    let mut state = State::new(pulldown_cmark::Options::empty(), Default::default());
    assert!(matches!(
        state.process([Event::End(TagEnd::Paragraph)], String::new()),
        Err(Error::UnexpectedEvent)
    ));
    assert!(matches!(state.finish(String::new()), Err(Error::Failed)));

    let mut state = State::new(pulldown_cmark::Options::empty(), Default::default());
    state.process([Event::Start(Tag::Paragraph)], String::new()).unwrap();
    assert!(matches!(state.finish(String::new()), Err(Error::UnexpectedEvent)));
    assert!(matches!(
        state.process([Event::End(TagEnd::Paragraph)], String::new()),
        Err(Error::Failed)
    ));

    let mut state = State::new(pulldown_cmark::Options::empty(), Default::default());
    state.process(Parser::new("first"), String::new()).unwrap();
    assert!(matches!(
        state.process(Parser::new("second"), RefuseWrites),
        Err(Error::FormatFailed(_))
    ));
    assert!(matches!(state.finish(String::new()), Err(Error::Failed)));

    let mut state = State::new(pulldown_cmark::Options::empty(), Default::default());
    state.process(Parser::new("first"), String::new()).unwrap();
    assert!(matches!(state.finish(RefuseWrites), Err(Error::FormatFailed(_))));
    assert!(matches!(state.finish(String::new()), Err(Error::Failed)));
}

#[test]
fn a_partial_write_cannot_be_retried() {
    struct Partial(String);
    impl fmt::Write for Partial {
        fn write_str(&mut self, text: &str) -> fmt::Result {
            self.0.push(text.chars().next().unwrap());
            Err(fmt::Error)
        }
    }
    let mut state = State::new(pulldown_cmark::Options::empty(), Default::default());
    let mut writer = Partial(String::new());
    state.process(Parser::new("éclair"), &mut writer).unwrap();
    assert!(matches!(state.finish(&mut writer), Err(Error::FormatFailed(_))));
    assert_eq!(writer.0, "é");
    assert!(matches!(state.finish(&mut writer), Err(Error::Failed)));
    assert_eq!(writer.0, "é");
}

#[test]
fn finished_states_reject_more_input() {
    let mut state = cmark(Parser::new("done"), String::new(), pulldown_cmark::Options::empty()).unwrap();
    assert!(matches!(
        state.process(Parser::new("more"), String::new()),
        Err(Error::Finished)
    ));
    assert!(matches!(state.finish(String::new()), Err(Error::Failed)));
}

#[test]
fn invalid_source_ranges_return_errors() {
    for range in [0..99, std::ops::Range { start: 2, end: 1 }, 0..1, 1..2] {
        let mut state = State::new(pulldown_cmark::Options::empty(), Default::default());
        assert!(matches!(
            state.process_with_source_range([(Event::Text("é".into()), Some(range))], "é", String::new()),
            Err(Error::InvalidSourceRange)
        ));
        assert!(matches!(state.finish(String::new()), Err(Error::Failed)));
    }
}

#[test]
fn fences_and_link_values_escape_decoded_syntax() {
    for input in [
        "~~~ a ``` b\nx\n~~~\n",
        "~~~~~\n````\n~~~~\n~~~~~\n",
        "[x](foo\\&ouml; \"a \\&ouml;\")",
        "[x](<)(>)",
    ] {
        let mut output = String::new();
        cmark(Parser::new(input), &mut output, pulldown_cmark::Options::empty()).unwrap();
        assert_eq!(
            TextMergeStream::new(Parser::new(input)).collect::<Vec<_>>(),
            TextMergeStream::new(Parser::new(&output)).collect::<Vec<_>>(),
            "{input:?} -> {output:?}"
        );
    }
}

#[test]
fn empty_nested_lists_do_not_become_a_rule() {
    let input = "-\n  -\n    - inner\n";
    let mut output = String::new();
    cmark(Parser::new(input), &mut output, pulldown_cmark::Options::empty()).unwrap();
    assert_eq!(
        TextMergeStream::new(Parser::new(input)).collect::<Vec<_>>(),
        TextMergeStream::new(Parser::new(&output)).collect::<Vec<_>>()
    );
}

#[test]
fn intervening_blocks_reset_list_marker_selection() {
    let mut output = String::new();
    cmark(
        Parser::new("1. first\n\n---\n\n1. second"),
        &mut output,
        pulldown_cmark::Options::empty(),
    )
    .unwrap();
    assert_eq!(output, "1. first\n\n---\n\n1. second");
}
