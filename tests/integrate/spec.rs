use std::cmp::Ordering;
use std::{convert::TryFrom, ops::Range};

use pretty_assertions::Comparison as PrettyComparison;
use yansi::Paint;

use pulldown_cmark::utils::TextMergeStream;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use pulldown_cmark_to_cmark::cmark;

const COMMONMARK_SPEC_TEXT: &str = include_str!("../spec/CommonMark/spec.txt");

const COMMONMARK_SPEC_EXAMPLE_COUNT: usize = 649;

// At the time of writing, ~90% of tests pass. This needs some additional work.
const EXPECTED_SUCCESS_EXAMPLE_COUNT: usize = 585;

const FULL_CMARK_RESULTS_VAR: &str = "FULL_CMARK_RESULTS";

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
    let splitted_text = text.split("\n.\n").collect::<Vec<_>>();
    let Ok([input, output]) = <[_; 2]>::try_from(splitted_text) else {
        panic!("CommonMark spec example code block has unexpected form.");
    };
    let output = output.trim_end_matches('\n');
    Some((input.to_string(), output.to_string()))
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

fn test_roundtrip(original: &str, expected_html: &str, line_number: usize, show_full_results: bool) -> bool {
    //
    // Markdown => [Event, ..] => Markdown
    // |_________ A _________|
    //             |__________ B ________|
    //
    // A: pulldown-cmark
    // B: pulldown-cmark-to-cmark

    // Do A
    let opts = Options::empty();
    let event_list = Parser::new_ext(original, opts).collect::<Vec<_>>();

    // Do B
    let mut regen_str = String::new();
    cmark(event_list.iter().cloned(), &mut regen_str).expect("Regeneration failure");

    // text events should be merged before comparing two event lists for equivalence.
    // you don't need to merge them before feeding them into `cmark`.
    let event_list: Vec<Event<'_>> = TextMergeStream::new(event_list.into_iter()).collect();
    let event_list_2 = TextMergeStream::new(Parser::new_ext(&regen_str, opts)).collect::<Vec<_>>();

    if event_list == event_list_2 {
        return true;
    }

    if show_full_results {
        eprintln!(
            "{}\n",
            format!("===== Conformance Test Failure (L{line_number}) =====")
                .bold()
                .underline()
        );

        eprintln!("{}\n", "Original Markdown Example");
        eprint_indented(original, "    ");
        eprintln!();

        eprintln!("{}\n", "Regenerated Markdown Example");
        eprint_indented(&regen_str, "    ");
        eprintln!();

        eprintln!("{}\n", "Expected HTML");
        eprint_indented(expected_html, "    ");
        eprintln!();

        eprintln!("{}\n", "Original vs Regenerated Event Sequence");
        let comparision = PrettyComparison::new(&event_list, &event_list_2);
        for line in format!("{comparision}").lines() {
            eprintln!("    {}", line);
        }
        eprintln!();
    }

    false
}

fn eprint_indented(text_block: &str, indent: &str) {
    for line in text_block.lines() {
        eprintln!("{indent}{line}");
    }
}

//======================================
// Tests
//======================================

#[test]
fn commonmark_spec() {
    let testsuite = parse_common_mark_testsuite();
    assert_eq!(COMMONMARK_SPEC_EXAMPLE_COUNT, testsuite.len());

    let show_full_results = std::env::var(FULL_CMARK_RESULTS_VAR).is_ok();

    let mut success_count = 0usize;
    for test_case in &testsuite {
        let MarkdownTestCase {
            markdown,
            expected_html,
            line_number,
        } = test_case;

        if test_roundtrip(markdown, expected_html, *line_number, show_full_results) {
            success_count += 1;
        }
    }

    let expected_percent = EXPECTED_SUCCESS_EXAMPLE_COUNT as f64 / testsuite.len() as f64;
    let actual_percent = success_count as f64 / testsuite.len() as f64;

    eprintln!();

    let (change, change_icon) = match success_count.cmp(&EXPECTED_SUCCESS_EXAMPLE_COUNT) {
        // If the user requested the full results, then proceed to printing
        // the full results and failing the test, even if the test would
        // have otherwise passed if the user hadn't requested the results.
        Ordering::Equal if !show_full_results => return,
        Ordering::Equal => ("Unchanged".blue(), ""),
        Ordering::Less => ("DECREASED".red(), "🔻"),
        Ordering::Greater => ("INCREASED".green(), "🟢"),
    };

    eprintln!("{}: {change}\n", "CommonMark Conformance Test Rate".bold().underline());

    eprintln!(
        "Expected to pass: {} ({:.1}%)",
        EXPECTED_SUCCESS_EXAMPLE_COUNT,
        100. * expected_percent
    );
    eprintln!(
        " Actually passed: {success_count} ({:.1}%) {change_icon}",
        100. * actual_percent,
    );
    eprintln!();
    eprintln!("CommonMark total: {}", testsuite.len());
    eprintln!();

    // Only ask the user to update the expected success count if they've managed
    // to increase it. Note: Some increases could be do to improvements in
    // pulldown-cmark, not this crate.
    if success_count > EXPECTED_SUCCESS_EXAMPLE_COUNT {
        eprintln!("Please update `EXPECTED_SUCCESS_EXAMPLE_COUNT` in {}\n", file!());
    }

    eprintln!("To see the full results:\n");
    eprintln!("    $ {}=true cargo test\n", FULL_CMARK_RESULTS_VAR);

    panic!()
}
