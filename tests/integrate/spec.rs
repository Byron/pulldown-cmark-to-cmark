use std::ops::Range;

use pulldown_cmark::{utils::TextMergeStream, CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use pulldown_cmark_to_cmark::{cmark, cmark_with_source_range, Progress, State};

const COMMONMARK_SPEC_TEXT: &str = include_str!("../spec/CommonMark/spec.txt");
const COMMONMARK_SPEC_EXAMPLE_COUNT: usize = 652;

struct MarkdownTestCase {
    markdown: String,
    expected_html: String,
    line_number: usize,
}

fn is_example_fence(tag: &Tag<'_>) -> bool {
    if let Tag::CodeBlock(CodeBlockKind::Fenced(fence_value)) = tag {
        &**fence_value == "example"
    } else {
        false
    }
}

fn collect_test_case<'a>(events: &mut impl Iterator<Item = (Event<'a>, Range<usize>)>) -> Option<(String, String)> {
    let Event::Start(begin_tag) = events.next()?.0 else {
        return None;
    };
    let Event::Text(text) = events.next()?.0 else {
        return None;
    };
    let Event::End(end_tag) = events.next()?.0 else {
        return None;
    };
    if !(is_example_fence(&begin_tag) && end_tag == TagEnd::CodeBlock) {
        return None;
    }
    let Some((input, output)) = text.split_once("\n.\n") else {
        panic!("CommonMark spec example code block has unexpected form.");
    };
    Some((format!("{}\n", input.replace('→', "\t")), output.replace('→', "\t")))
}

fn parse_common_mark_testsuite() -> Vec<MarkdownTestCase> {
    let opts = Options::empty();
    let p = Parser::new_ext(COMMONMARK_SPEC_TEXT, opts).into_offset_iter();

    let mut testsuite = vec![];
    let mut p = p.peekable();
    while let Some((peeked_event, range)) = p.peek() {
        match peeked_event {
            Event::Start(tag) if is_example_fence(tag) => (),
            _ => {
                let _ = p.next();
                continue;
            }
        }

        let line_number = COMMONMARK_SPEC_TEXT[..range.start].lines().count() + 1;

        // a new example, insert it into the testsuite.
        let (markdown, expected_html) = collect_test_case(&mut p).expect("Error parsing example text from spec.");
        testsuite.push(MarkdownTestCase {
            line_number,
            markdown,
            expected_html,
        });
    }

    testsuite
}

fn assert_roundtrip(case: &MarkdownTestCase, output: &str, example: usize, mode: &str) {
    let expected = TextMergeStream::new(Parser::new(&case.markdown)).collect::<Vec<_>>();
    let actual = TextMergeStream::new(Parser::new(output)).collect::<Vec<_>>();
    assert_eq!(
        expected, actual,
        "CommonMark 0.31.2 example {example}, line {}, {mode}\ninput: {:?}\noutput: {output:?}\nHTML: {}",
        case.line_number, case.markdown, case.expected_html
    );
}

#[test]
fn fixture_tabs_and_final_newlines_are_preserved() {
    let mut events = Parser::new("```example\n#→Foo\n.\n<h1>Foo</h1>\n```\n").into_offset_iter();
    assert_eq!(
        collect_test_case(&mut events),
        Some(("#\tFoo\n".into(), "<h1>Foo</h1>\n".into()))
    );
}

#[test]
fn commonmark_spec() {
    let cases = parse_common_mark_testsuite();
    assert_eq!(COMMONMARK_SPEC_EXAMPLE_COUNT, cases.len());
    for (index, case) in cases.iter().enumerate() {
        let mut ordinary = String::new();
        cmark(Parser::new(&case.markdown), &mut ordinary).unwrap();
        assert_roundtrip(case, &ordinary, index + 1, "ordinary");

        let mut with_source = String::new();
        cmark_with_source_range(
            Parser::new(&case.markdown)
                .into_offset_iter()
                .map(|(event, range)| (event, Some(range))),
            &case.markdown,
            &mut with_source,
        )
        .unwrap();
        assert_roundtrip(case, &with_source, index + 1, "source ranges");
    }
}

fn process(state: &mut State, events: &[(Event<'_>, Range<usize>)], source: Option<&str>, output: &mut String) {
    let before = output.len();
    let progress = if let Some(source) = source {
        state.process_with_source_range(
            events.iter().map(|(event, range)| (event, Some(range.clone()))),
            source,
            &mut *output,
        )
    } else {
        state.process(events.iter().map(|(event, _)| event), &mut *output)
    }
    .unwrap();
    assert_eq!(
        progress,
        Progress {
            events_consumed: events.len(),
            bytes_written: output.len() - before
        }
    );
}

fn finish(state: &mut State, output: &mut String) {
    let before = output.len();
    let progress = state.finish(&mut *output).unwrap();
    assert_eq!(
        progress,
        Progress {
            events_consumed: 0,
            bytes_written: output.len() - before
        }
    );
    assert_eq!(state.finish(output).unwrap(), Progress::default());
}

#[test]
fn commonmark_spec_at_every_event_boundary() {
    for (index, case) in parse_common_mark_testsuite().iter().enumerate() {
        let events: Vec<_> = Parser::new(&case.markdown).into_offset_iter().collect();
        for source in [None, Some(case.markdown.as_str())] {
            let mut expected = String::new();
            let mut state = State::default();
            process(&mut state, &events, source, &mut expected);
            finish(&mut state, &mut expected);
            assert_roundtrip(case, &expected, index + 1, "incremental");

            for split in 0..=events.len() {
                let mut actual = String::new();
                let mut state = State::default();
                process(&mut state, &events[..split], source, &mut actual);
                process(&mut state, &events[split..], source, &mut actual);
                finish(&mut state, &mut actual);
                assert_eq!(
                    actual,
                    expected,
                    "example {}, split {split}, source ranges: {}",
                    index + 1,
                    source.is_some()
                );
            }
            let mut actual = String::new();
            let mut state = State::default();
            for event in &events {
                process(&mut state, std::slice::from_ref(event), source, &mut actual);
            }
            finish(&mut state, &mut actual);
            assert_eq!(
                actual,
                expected,
                "example {}, event by event, source ranges: {}",
                index + 1,
                source.is_some()
            );
        }
    }
}
