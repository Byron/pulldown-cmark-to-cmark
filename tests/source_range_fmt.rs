// Copied from `fmt.rs`.

use pulldown_cmark::{Options, Parser};
use pulldown_cmark_to_cmark::{
    cmark_resume_with_source_range_and_options, cmark_with_source_range, Options as CmarkToCmarkOptions, State,
};

pub fn fmts(s: &str) -> (String, State<'_>) {
    let mut buf = String::new();
    let mut s = cmark_with_source_range(
        Parser::new_ext(s, Options::all())
            .into_offset_iter()
            .map(|(e, r)| (e, Some(r))),
        s,
        &mut buf,
    )
    .unwrap();
    // Not testing this field.
    s.last_event_end_index = Default::default();
    (buf, s)
}

pub fn fmts_with_options<'a>(s: &'a str, options: CmarkToCmarkOptions<'a>) -> (String, State<'a>) {
    let mut buf = String::new();
    let mut s = cmark_resume_with_source_range_and_options(
        Parser::new_ext(s, Options::all())
            .into_offset_iter()
            .map(|(e, r)| (e, Some(r))),
        s,
        &mut buf,
        None,
        options,
    )
    .unwrap();
    // Not testing this field.
    s.last_event_end_index = Default::default();
    (buf, s)
}

/// Asserts that if we parse our `str` s into a series of events, then serialize them with `cmark`
/// that we'll get the same series of events when we parse them again.
pub fn assert_events_eq(s: &str) {
    let mut buf = String::new();
    cmark_with_source_range(
        Parser::new_ext(s, Options::all())
            .into_offset_iter()
            .map(|(e, r)| (e, Some(r))),
        s,
        &mut buf,
    )
    .unwrap();

    let before_events = Parser::new_ext(s, Options::all());
    let after_events = Parser::new_ext(&buf, Options::all());
    assert_eq!(before_events.collect::<Vec<_>>(), after_events.collect::<Vec<_>>());
}
