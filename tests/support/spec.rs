use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use std::ops::Range;

const COMMONMARK_SPEC_TEXT: &str = include_str!("../spec/CommonMark/spec.txt");
pub const COMMONMARK_SPEC_EXAMPLE_COUNT: usize = 652;

pub struct MarkdownTestCase {
    pub markdown: String,
    pub expected_html: String,
    pub line_number: usize,
}

fn is_example_fence(tag: &Tag<'_>) -> bool {
    if let Tag::CodeBlock(CodeBlockKind::Fenced(fence_value)) = tag {
        &**fence_value == "example"
    } else {
        false
    }
}

pub fn collect_test_case<'a>(events: &mut impl Iterator<Item = (Event<'a>, Range<usize>)>) -> Option<(String, String)> {
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

pub fn parse_common_mark_testsuite() -> Vec<MarkdownTestCase> {
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
