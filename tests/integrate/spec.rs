use std::ops::Range;

use pulldown_cmark::{utils::TextMergeStream, Event, Parser};
use pulldown_cmark_to_cmark::{cmark, cmark_with_source_range, Progress, State};

#[path = "../support/spec.rs"]
mod fixtures;
use fixtures::*;

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
        cmark(
            Parser::new(&case.markdown),
            &mut ordinary,
            pulldown_cmark::Options::empty(),
        )
        .unwrap();
        assert_roundtrip(case, &ordinary, index + 1, "ordinary");

        let mut with_source = String::new();
        cmark_with_source_range(
            Parser::new(&case.markdown)
                .into_offset_iter()
                .map(|(event, range)| (event, Some(range))),
            &case.markdown,
            &mut with_source,
            pulldown_cmark::Options::empty(),
        )
        .unwrap();
        assert_roundtrip(case, &with_source, index + 1, "source ranges");
    }
    println!(
        "CommonMark 0.31.2: {}/{} examples passed (ordinary and source ranges)",
        cases.len(),
        COMMONMARK_SPEC_EXAMPLE_COUNT
    );
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
    let cases = parse_common_mark_testsuite();
    assert_eq!(COMMONMARK_SPEC_EXAMPLE_COUNT, cases.len());
    for (index, case) in cases.iter().enumerate() {
        let events: Vec<_> = Parser::new(&case.markdown).into_offset_iter().collect();
        for source in [None, Some(case.markdown.as_str())] {
            let mut expected = String::new();
            let mut state = State::new(pulldown_cmark::Options::empty(), Default::default());
            process(&mut state, &events, source, &mut expected);
            finish(&mut state, &mut expected);
            assert_roundtrip(case, &expected, index + 1, "incremental");

            for split in 0..=events.len() {
                let mut actual = String::new();
                let mut state = State::new(pulldown_cmark::Options::empty(), Default::default());
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
            let mut state = State::new(pulldown_cmark::Options::empty(), Default::default());
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
    println!(
        "CommonMark 0.31.2: {}/{} examples passed (incremental, every event split and event by event)",
        cases.len(),
        COMMONMARK_SPEC_EXAMPLE_COUNT
    );
}
