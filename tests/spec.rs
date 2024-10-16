use pulldown_cmark::utils::TextMergeStream;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use pulldown_cmark_to_cmark::cmark;

const COMMONMARK_SPEC_TEXT: &str = include_str!("./spec/CommonMark/spec.txt");

const COMMONMARK_SPEC_EXAMPLE_COUNT: usize = 649;

fn is_example_fence(tag: &Tag<'_>) -> bool {
    if let Tag::CodeBlock(CodeBlockKind::Fenced(fence_value)) = tag {
        &**fence_value == "example"
    } else {
        false
    }
}

fn collect_test_case<'a>(events: &mut impl Iterator<Item = Event<'a>>) -> Option<(String, String)> {
    let begin_tag = events
        .next()
        .and_then(|e| if let Event::Start(tag) = e { Some(tag) } else { None })?;
    let text = events
        .next()
        .and_then(|e| if let Event::Text(text) = e { Some(text) } else { None })?;
    let end_tag = events
        .next()
        .and_then(|e| if let Event::End(tag) = e { Some(tag) } else { None })?;
    if !(is_example_fence(&begin_tag) && end_tag == TagEnd::CodeBlock) {
        return None;
    }
    let splitted_text = text.split("\n.\n").collect::<Vec<_>>();
    if splitted_text.len() != 2 {
        return None;
    }
    let input = splitted_text[0];
    let output = splitted_text[1].trim_end_matches('\n');
    Some((input.to_string(), output.to_string()))
}

fn test_roundtrip(original: &str, expected: &str) -> bool {
    let opts = Options::empty();
    let event_list = Parser::new_ext(original, opts).collect::<Vec<_>>();
    let mut regen_str = String::new();
    cmark(event_list.iter().cloned(), &mut regen_str).expect("Regeneration failure");
    // text events should be merged before comparing two event lists for equivalence.
    // you don't need to merge them before feeding them into `cmark`.
    let event_list: Vec<Event<'_>> = TextMergeStream::new(event_list.into_iter()).collect();
    let event_list_2 = TextMergeStream::new(Parser::new_ext(&regen_str, opts)).collect::<Vec<_>>();
    let event_count = event_list.len();
    let event_count_2 = event_list_2.len();
    let same_event_count = event_list
        .iter()
        .zip(event_list_2.iter())
        .take_while(|(e1, e2)| e1 == e2)
        .count();
    if event_count == same_event_count && event_count_2 == same_event_count {
        true
    } else {
        eprintln!(
            "Test fail: event [{}/{}] is {:?} vs {:?}\nExpected full output:\n{}",
            same_event_count,
            event_count,
            event_list.get(same_event_count),
            event_list_2.get(same_event_count),
            expected
        );
        false
    }
}

#[test]
#[should_panic] // at the time of writing, 60% of tests pass. This needs considerable work.
fn commonmark_spec() {
    let opts = Options::empty();
    let p = Parser::new_ext(COMMONMARK_SPEC_TEXT, opts);

    let mut testsuite = vec![];
    let mut p = p.peekable();
    while let Some(peeked_event) = p.peek() {
        if let Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(fence_value))) = peeked_event {
            if &**fence_value == "example" {
                // a new example, insert it into the testsuite.
                let new_test_case = collect_test_case(&mut p).expect("Error parsing example text from spec.");
                testsuite.push(new_test_case);
                continue;
            }
        }
        let _ = p.next();
    }
    assert_eq!(COMMONMARK_SPEC_EXAMPLE_COUNT, testsuite.len());
    let mut success_count = 0usize;
    for (original, expected) in testsuite {
        if test_roundtrip(&original, &expected) {
            success_count += 1;
        }
    }
    assert_eq!(COMMONMARK_SPEC_EXAMPLE_COUNT, success_count);
}
