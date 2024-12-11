use pulldown_cmark::{utils::TextMergeStream, Alignment, CodeBlockKind, Event, LinkType, Options, Parser, Tag, TagEnd};
pub use pulldown_cmark_to_cmark::{
    cmark, cmark_resume, cmark_resume_with_options, Options as CmarkToCmarkOptions, State,
};

fn assert_output_and_states_eq(output0: &str, state0: &State, output1: &str, state1: &State) {
    assert_eq!(
        output0, output1,
        "Output of formatting without and with source range differs!"
    );
    assert_eq!(
        state0, state1,
        "States of formatting without and with source range differs!"
    );
}

fn fmts_both(s: &str) -> (String, State<'_>) {
    let (buf0, s0) = fmts(s);
    let (buf1, s1) = source_range::fmts(s);
    assert_output_and_states_eq(&buf0, &s0, &buf1, &s1);
    (buf0, s0)
}

fn fmts(s: &str) -> (String, State<'_>) {
    let mut buf = String::new();
    let s = cmark(Parser::new_ext(s, Options::all()), &mut buf).unwrap();
    (buf, s)
}

fn fmts_with_options<'a>(s: &'a str, options: CmarkToCmarkOptions<'a>) -> (String, State<'a>) {
    let (buf1, s1) = source_range::fmts_with_options(s, options.clone());
    let mut buf = String::new();
    let s = cmark_resume_with_options(Parser::new_ext(s, Options::all()), &mut buf, None, options).unwrap();
    assert_output_and_states_eq(&buf, &s, &buf1, &s1);
    (buf, s)
}

fn fmtes<'a>(e: &'a [Event], s: State<'a>) -> (String, State<'a>) {
    let mut buf = String::new();
    let s = cmark_resume(e.iter(), &mut buf, Some(s)).unwrap();
    (buf, s)
}

fn fmte<'a>(e: impl AsRef<[Event<'a>]>) -> (String, State<'a>) {
    let mut buf = String::new();
    let s = cmark(e.as_ref().iter(), &mut buf).unwrap();
    (buf, s)
}

fn assert_events_eq_both(s: &str) {
    assert_events_eq(s);
    source_range::assert_events_eq(s);
}

/// Asserts that if we parse our `str` s into a series of events, then serialize them with `cmark`
/// that we'll get the same series of events when we parse them again.
fn assert_events_eq(s: &str) {
    let before_events = Parser::new_ext(s, Options::all());

    let mut buf = String::new();
    cmark(before_events, &mut buf).unwrap();

    let before_events = TextMergeStream::new(Parser::new_ext(s, Options::all()));
    let after_events = TextMergeStream::new(Parser::new_ext(&buf, Options::all()));
    println!("{buf}");
    assert_eq!(before_events.collect::<Vec<_>>(), after_events.collect::<Vec<_>>());
}

mod lazy_newlines {
    use super::{fmte, fmts_both, Event, LinkType, State, Tag, TagEnd};

    #[test]
    fn after_emphasis_there_is_no_newline() {
        for t in [
            Tag::Emphasis,
            Tag::Strong,
            Tag::Link {
                link_type: LinkType::Inline,
                dest_url: "".into(),
                title: "".into(),
                id: "".into(),
            },
            Tag::Image {
                link_type: LinkType::Inline,
                dest_url: "".into(),
                title: "".into(),
                id: "".into(),
            },
            Tag::FootnoteDefinition("".into()),
        ] {
            let end = t.to_end();
            let mut state = State::default();
            state.newlines_before_start = 0;
            assert_eq!(fmte(&[Event::Start(t), Event::End(end)]).1, state);
        }
    }

    #[test]
    fn after_anything_else_it_has_one_newline() {
        for e in &[
            Event::End(TagEnd::Item),
            Event::End(TagEnd::TableRow),
            Event::End(TagEnd::TableHead),
        ] {
            let mut state = State::default();
            state.newlines_before_start = 1;
            assert_eq!(fmte(&[e.clone()]).1, state);
        }
    }

    #[test]
    fn after_some_types_it_has_multiple_newlines() {
        for md in &["paragraph", "## headline", "\n````\n````", "---"] {
            let mut state = State::default();
            state.newlines_before_start = 2;
            assert_eq!(fmts_both(md), (String::from(*md), state));
        }
    }
}

#[test]
fn it_applies_newlines_before_start_before_text() {
    let mut first = State::default();
    first.newlines_before_start = 2;
    first.last_was_text_without_trailing_newline = true;

    let mut second = State::default();
    second.newlines_before_start;
    second.last_was_text_without_trailing_newline = true;

    assert_eq!(fmtes(&[Event::Text("t".into())], first), ("\n\nt".into(), second));
}

#[test]
fn it_applies_newlines_before_start_before_any_start_tag() {
    let mut first = State::default();
    first.newlines_before_start = 2;
    first.last_was_text_without_trailing_newline = true;

    let mut second = State::default();
    second.newlines_before_start = 0;
    second.last_was_text_without_trailing_newline = true;

    assert_eq!(
        fmtes(&[Event::Start(Tag::Paragraph), Event::Text("h".into())], first,),
        ("\n\nh".into(), second)
    );
}

mod padding {
    use super::{fmtes, Event, State, Tag};

    #[test]
    fn is_used_before_newlines() {
        let mut first = State::default();
        first.newlines_before_start = 2;
        first.padding = vec!["  ".into()];
        first.last_was_text_without_trailing_newline = true;

        let mut second = State::default();
        second.newlines_before_start = 0;
        second.padding = vec!["  ".into()];
        second.last_was_text_without_trailing_newline = true;

        assert_eq!(
            fmtes(&[Event::Start(Tag::Paragraph), Event::Text("h".into())], first,),
            ("\n  \n  h".into(), second)
        );
    }
}

mod inline_elements {
    use crate::fmt::fmts_with_options;

    use super::source_range;
    use super::{fmts_both, CmarkToCmarkOptions, State};

    #[test]
    fn image() {
        let mut state = State::default();
        state.newlines_before_start = 2;
        assert_eq!(
            fmts_both("![a](b)\n![c][d]\n\n[d]: e"),
            ("![a](b)\n![c][d]\n\n[d]: e".into(), state)
        );
    }

    #[test]
    fn image_collapsed() {
        let mut state = State::default();
        state.newlines_before_start = 2;
        assert_eq!(
            fmts_both("![c][d]\n\n![c][]![c][]\n\n[d]: e\n[c]: f"),
            ("![c][d]\n\n![c][]![c][]\n\n[d]: e\n[c]: f".into(), state)
        );
    }

    #[test]
    fn footnote() {
        let mut state = State::default();
        state.newlines_before_start = 2;
        assert_eq!(fmts_both("a [^b]\n\n[^b]: c"), ("a [^b]\n\n[^b]: c".into(), state));
    }

    #[test]
    fn multiline_footnote() {
        assert_eq!(
            fmts_both("a [^b]\n\n[^b]: this is\n    one footnote").0,
            "a [^b]\n\n[^b]: this is\n    one footnote",
        );
    }

    #[test]
    fn autolinks_are_fully_resolved() {
        assert_eq!(fmts_both("<http://a/b>").0, "<http://a/b>",);
    }

    #[test]
    fn links() {
        {
            let mut state = State::default();
            state.newlines_before_start = 2;
            assert_eq!(
                fmts_both("[a](b)\n[c][d]\n\n[d]: e"),
                ("[a](b)\n[c][d]\n\n[d]: e".into(), state)
            );
        }
    }

    #[test]
    fn links_collapsed() {
        let mut state = State::default();
        state.newlines_before_start = 2;
        assert_eq!(
            fmts_both("[c][d]\n\n[c][][c][]\n\n[d]: e\n[c]: f"),
            ("[c][d]\n\n[c][][c][]\n\n[d]: e\n[c]: f".into(), state)
        );
    }

    #[test]
    fn shortcut_links() {
        {
            let mut state = State::default();
            state.newlines_before_start = 2;
            assert_eq!(
                fmts_both("[a](b)\n[c]\n\n[c]: e"),
                ("[a](b)\n[c]\n\n[c]: e".into(), state)
            );
        }
    }

    #[test]
    fn shortcut_code_links() {
        let mut state = State::default();
        state.newlines_before_start = 2;
        assert_eq!(
            fmts_both("[a](b)\n[`c`]\n\n[`c`]: e"),
            ("[a](b)\n[`c`]\n\n[`c`]: e".into(), state)
        );
    }

    #[test]
    fn multiple_shortcut_links() {
        let mut state = State::default();
        state.newlines_before_start = 2;
        assert_eq!(
            fmts_both("[a](b)\n[c] [d]\n\n[c]: e\n[d]: f"),
            ("[a](b)\n[c] [d]\n\n[c]: e\n[d]: f".into(), state)
        );
    }

    #[test]
    fn various() {
        let mut state = State::default();
        state.newlines_before_start = 2;
        assert_eq!(
            fmts_both("*a* b **c**\n<br>\nd\n\ne `c`"),
            ("*a* b **c**\n<br>\nd\n\ne `c`".into(), state)
        );
    }

    #[test]
    fn various_with_custom_options() {
        let custom_options = CmarkToCmarkOptions {
            emphasis_token: '_',
            code_block_token: '~',
            ..Default::default()
        };

        let (s, state) = fmts_with_options("_a_ b **c**\n<br>\nd\n\ne `c`", custom_options);

        assert_eq!(s, "_a_ b **c**\n<br>\nd\n\ne `c`".to_string());

        let mut expected = State::default();
        expected.newlines_before_start = 2;
        assert_eq!(state, expected);
    }

    #[test]
    fn strikethrough() {
        assert_eq!(fmts_both("~~strikethrough~~").0, "~~strikethrough~~",);
    }

    #[test]
    fn code_double_backtick() {
        let mut state = State::default();
        state.newlines_before_start = 2;
        assert_eq!(
            fmts_both("lorem ``ipsum `dolor` sit`` amet"),
            ("lorem ``ipsum `dolor` sit`` amet".into(), state)
        );
    }

    #[test]
    fn code_triple_backtick() {
        let mut state = State::default();
        state.newlines_before_start = 2;
        assert_eq!(
            fmts_both("lorem ```ipsum ``dolor`` sit``` amet"),
            ("lorem ```ipsum ``dolor`` sit``` amet".into(), state)
        );
    }

    #[test]
    fn code_backtick_normalization() {
        // The minimum amount of backticks are inserted.
        let mut state = State::default();
        state.newlines_before_start = 2;
        assert_eq!(
            fmts_both("lorem ```ipsum ` dolor``` amet"),
            ("lorem ``ipsum ` dolor`` amet".into(), state)
        );
    }

    #[test]
    fn code_leading_trailing_backtick() {
        // Spaces are inserted if the inline code starts or ends with
        // a backtick.
        {
            let mut state = State::default();
            state.newlines_before_start = 2;
            assert_eq!(
                fmts_both("`` `lorem ``   `` ipsum` ``"),
                ("`` `lorem ``   `` ipsum` ``".into(), state)
            );
        }
    }

    #[test]
    fn code_spaces_before_backtick() {
        //  No space is inserted if it is not needed.
        {
            let mut state = State::default();
            state.newlines_before_start = 2;
            assert_eq!(fmts_both("` lorem `   ` `"), ("`lorem`   ` `".into(), state));
        }
    }

    #[test]
    fn no_escaping_special_character_in_code() {
        // https://github.com/Byron/pulldown-cmark-to-cmark/issues/73
        let input = r#"
```rust
# fn main() {
println!("Hello, world!");
# }
```
"#;
        let iter = pulldown_cmark::Parser::new(input);
        let mut actual = String::new();
        pulldown_cmark_to_cmark::cmark_with_source_range_and_options(
            iter.map(|e| (e, None)),
            input,
            &mut actual,
            Default::default(),
        )
        .unwrap();
        let expected = r#"
````rust
# fn main() {
println!("Hello, world!");
# }
````"#;
        assert_eq!(actual, expected);
    }

    #[test]
    fn rustdoc_link() {
        // Brackets are not escaped if not escaped in the source.
        {
            let mut state = State::default();
            state.newlines_before_start = 2;
            assert_eq!(source_range::fmts("[`Vec`]"), ("[`Vec`]".into(), state));
        }
    }

    #[test]
    fn preserve_less_than_sign_escape() {
        // `<` is not escaped if not escaped in the source.
        let mut state = State::default();
        state.newlines_before_start = 2;
        assert_eq!(source_range::fmts("a < 1"), ("a < 1".into(), state));
        // `<` is escaped if escaped in the source.
        let mut state = State::default();
        state.newlines_before_start = 2;
        assert_eq!(source_range::fmts(r"a \< 1"), (r"a \< 1".into(), state));
    }
}

mod blockquote {
    use super::{assert_events_eq_both, fmte, fmtes, fmts_both, Event, State, Tag, TagEnd};
    use indoc::indoc;

    #[test]
    fn it_pops_padding_on_quote_end() {
        let mut first = State::default();
        first.padding = vec![" > ".into()];

        let mut second = State::default();
        second.newlines_before_start = 2;
        second.padding = vec![];

        assert_eq!(fmtes(&[Event::End(TagEnd::BlockQuote(None)),], first,).1, second);
    }

    #[test]
    fn it_pushes_padding_on_quote_start() {
        let mut state = State::default();
        state.newlines_before_start = 1;
        state.padding = vec![" > ".into()];
        assert_eq!(fmte(&[Event::Start(Tag::BlockQuote(None)),]).1, state);
    }

    #[test]
    fn with_html() {
        let s = indoc!(
            "
             > <table>
             > </table>
             "
        );

        assert_events_eq_both(s);

        assert_eq!(fmts_both(s).0, "\n > \n > <table>\n > </table>\n > ");
    }

    #[test]
    fn with_inlinehtml() {
        assert_eq!(fmts_both(" > <br>").0, "\n > \n > <br>");
    }

    #[test]
    fn with_plaintext_in_html() {
        assert_eq!(fmts_both("<del>\n*foo*\n</del>").0, "<del>\n*foo*\n</del>");
    }

    #[test]
    fn with_markdown_nested_in_html() {
        assert_eq!(fmts_both("<del>\n\n*foo*\n\n</del>").0, "<del>\n\n*foo*\n\n</del>");
    }

    #[test]
    fn with_codeblock() {
        let s = indoc!(
            "
             > ```a
             > t1
             > t2
             > ```
            "
        );

        assert_events_eq_both(s);

        assert_eq!(fmts_both(s).0, "\n > \n > ````a\n > t1\n > t2\n > ````",);
    }

    #[test]
    fn nested() {
        let s = indoc!(
            "
             > a
             >
             > > b
             >
             > c
            "
        );

        assert_events_eq_both(s);

        assert_eq!(fmts_both(s).0, "\n > \n > a\n > \n >  > \n >  > b\n > \n > c",);
    }

    #[test]
    fn initially_nested() {
        let s = indoc!(
            "
             > > foo
             > bar
             > > baz
            "
        );

        assert_events_eq_both(s);

        assert_eq!(fmts_both(s).0, "\n > \n >  > \n >  > foo\n >  > bar\n >  > baz",);
    }

    #[test]
    fn simple() {
        // Inlining this rather than using indoc because format-on-save with
        // rustfmt tries to strip the trailing spaces after `b` otherwise in
        // some editors.
        let s = "> a\n> b  \n> c\n";

        assert_events_eq_both(s);

        {
            let mut state = State::default();
            state.newlines_before_start = 2;
            assert_eq!(fmts_both(s), ("\n > \n > a\n > b  \n > c".into(), state));
        }
    }

    #[test]
    fn empty() {
        let s = " > ";

        assert_events_eq_both(s);

        {
            let mut state = State::default();
            state.newlines_before_start = 2;
            assert_eq!(fmts_both(s), ("\n > ".into(), state));
        }
    }

    #[test]
    fn with_blank_line() {
        let s = indoc!(
            "
            > foo

            > bar
            "
        );

        assert_events_eq_both(s);

        let mut state = State::default();
        state.newlines_before_start = 2;
        assert_eq!(fmts_both(s), ("\n > \n > foo\n\n > \n > bar".into(), state));
    }

    #[test]
    fn with_lazy_continuation() {
        let s = indoc!(
            "
            > foo
            baz

            > bar
            "
        );

        assert_events_eq_both(s);

        let mut state = State::default();
        state.newlines_before_start = 2;
        assert_eq!(fmts_both(s), ("\n > \n > foo\n > baz\n\n > \n > bar".into(), state));
    }

    #[test]
    fn with_lists() {
        let s = indoc!(
            "
            - > * foo
              >     * baz
                - > bar
            "
        );

        assert_events_eq_both(s);

        let mut state = State::default();
        state.newlines_before_start = 2;
        assert_eq!(
            fmts_both(s),
            (
                "* \n   > \n   > * foo\n   >   * baz\n  \n  * \n     > \n     > bar".into(),
                state
            )
        );
    }

    #[test]
    fn complex_nesting() {
        assert_events_eq_both(indoc!(
            "
            > one
            > > two
            > > three
            > four
            >
            > > five
            >
            > > six
            > seven
            > > > eight
            nine

            > ten

            >

            >
            > >


            > >

            > - eleven
            >    - twelve
            > > thirteen
            > -

            - > fourteen
                - > fifteen
            "
        ));
    }
}

mod codeblock {
    use super::{fmte, fmts_both, fmts_with_options, CmarkToCmarkOptions, CodeBlockKind, Event, State, Tag};

    #[test]
    fn it_keeps_track_of_the_presence_of_a_code_block() {
        let mut state = State::default();
        state.code_block = Some(pulldown_cmark_to_cmark::CodeBlockKind::Fenced);
        assert_eq!(
            fmte(&[Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced("s".into()))),]).1,
            state
        );
    }

    #[test]
    fn simple_and_paragraph() {
        let mut state = State::default();
        state.newlines_before_start = 2;
        assert_eq!(
            fmts_both("````hi\nsome\ntext\n````\na"),
            ("\n````hi\nsome\ntext\n````\n\na".into(), state)
        );
    }

    #[test]
    fn empty() {
        {
            let mut state = State::default();
            state.newlines_before_start = 2;
            assert_eq!(fmts_both("```\n```"), ("\n````\n````".into(), state));
        }
    }

    #[test]
    fn simple() {
        let mut state = State::default();
        state.newlines_before_start = 2;
        assert_eq!(
            fmts_both("```hi\nsome\ntext\n```"),
            ("\n````hi\nsome\ntext\n````".into(), state)
        );
    }

    #[test]
    fn simple_other_syntax() {
        let mut state = State::default();
        state.newlines_before_start = 2;
        assert_eq!(
            fmts_both("~~~hi\nsome\ntext\n~~~"),
            ("\n````hi\nsome\ntext\n````".into(), state)
        );
    }

    #[test]
    fn simple_other_syntax_with_custom() {
        let custom_options = CmarkToCmarkOptions {
            code_block_token: '~',
            ..Default::default()
        };

        let original = "~~~hi\nsome\ntext\n~~~";
        let (s, _) = fmts_with_options(original, custom_options);

        assert_eq!(s, "\n~~~~hi\nsome\ntext\n~~~~".to_string());
    }

    #[test]
    fn indented() {
        let mut state = State::default();
        state.newlines_before_start = 2;
        assert_eq!(
            fmts_both("    first\n    second\nthird"),
            ("\n    first\n    second\n    \n\nthird".into(), state)
        );
    }

    #[test]
    fn html_indented() {
        let mut state = State::default();
        state.newlines_before_start = 2;
        assert_eq!(
            fmts_both("  <!-- foo -->\n\n    <!-- foo -->"),
            ("  <!-- foo -->\n\n    <!-- foo -->\n    ".into(), state)
        );
    }
}

mod table {
    use indoc::indoc;
    use pretty_assertions::assert_eq;
    use pulldown_cmark_to_cmark::Alignment;

    use super::{fmte, fmtes, Alignment as TableAlignment, Event, State, Tag, TagEnd};

    #[test]
    fn it_forgets_alignments_and_headers_at_the_end_of_tables() {
        let mut first = State::default();
        first.table_alignments = vec![Alignment::None, Alignment::Center];
        first.table_headers = vec!["a".into(), "b".into()];

        let mut second = State::default();
        second.newlines_before_start = 2;

        assert_eq!(fmtes(&[Event::End(TagEnd::Table),], first,).1, second);
    }

    #[test]
    fn it_keeps_track_of_alignments_and_headers() {
        let mut state = State::default();
        state.table_alignments = vec![Alignment::None, Alignment::Center];
        state.table_headers = vec!["a".into(), "b".into()];
        assert_eq!(
            fmte(&[
                Event::Start(Tag::Table(vec![TableAlignment::None, TableAlignment::Center])),
                Event::Start(Tag::TableHead),
                Event::Start(Tag::TableCell),
                Event::Text("a".into()),
                Event::End(TagEnd::TableCell),
                Event::Start(Tag::TableCell),
                Event::Text("b".into()),
                Event::End(TagEnd::TableCell),
            ])
            .1,
            state
        );
    }

    #[test]
    fn it_generates_equivalent_table_markdown() {
        use pulldown_cmark::{Options, Parser};

        let original_table_markdown = indoc!(
            "
            | Tables        | Are           | Cool  | yo ||
            |---------------|:-------------:|------:|:---|--|
            | col 3 is      | right-aligned | $1600 | x  |01|
            | col 2 is      | centered      |   $12 | y  |02|
            | zebra stripes | are neat      |    $1 | z  |03|"
        );
        let p = Parser::new_ext(original_table_markdown, Options::all());
        let original_events: Vec<_> = p.into_iter().collect();

        let (generated_markdown, _) = fmte(&original_events);

        assert_eq!(
            generated_markdown,
            indoc!(
                "
            |Tables|Are|Cool|yo||
            |------|:-:|---:|:-|-|
            |col 3 is|right-aligned|$1600|x|01|
            |col 2 is|centered|$12|y|02|
            |zebra stripes|are neat|$1|z|03|"
            )
        );

        let p = Parser::new_ext(&generated_markdown, Options::all());
        let generated_events: Vec<_> = p.into_iter().collect();

        assert_eq!(original_events, generated_events);
    }

    #[test]
    fn it_generates_equivalent_table_markdown_with_empty_headers() {
        use pulldown_cmark::{Options, Parser};

        let original_table_markdown = indoc!(
            "
            ||||||
            |:-------------:|:--------------|------:|:--:|:-:|
            | col 3 is      | right-aligned | $1600 | x  |01|
            | col 2 is      | centered      |   $12 | y  |02|
            | zebra stripes | are neat      |    $1 | z  |03|"
        );
        let p = Parser::new_ext(original_table_markdown, Options::all());
        let original_events: Vec<_> = p.into_iter().collect();

        let (generated_markdown, _) = fmte(&original_events);

        assert_eq!(
            generated_markdown,
            indoc!(
                "
            ||||||
            |:-:|:-|-:|:-:|:-:|
            |col 3 is|right-aligned|$1600|x|01|
            |col 2 is|centered|$12|y|02|
            |zebra stripes|are neat|$1|z|03|"
            )
        );

        let p = Parser::new_ext(&generated_markdown, Options::all());
        let generated_events: Vec<_> = p.into_iter().collect();

        assert_eq!(original_events, generated_events);
    }
    #[test]
    fn table_with_pipe_in_column() {
        use pulldown_cmark::{Options, Parser};

        let original_table_markdown = indoc!(
            r"
            | \| | a\|b |
            |----|------|
            | \| | a\|b |"
        );
        let p = Parser::new_ext(original_table_markdown, Options::all());
        let original_events: Vec<_> = p.into_iter().collect();

        let (generated_markdown, _) = fmte(&original_events);

        assert_eq!(
            generated_markdown,
            indoc!(
                r"
                |\||a\|b|
                |-|---|
                |\||a\|b|"
            )
        );

        let p = Parser::new_ext(&generated_markdown, Options::all());
        let generated_events: Vec<_> = p.into_iter().collect();

        assert_eq!(original_events, generated_events);
    }
}

mod escapes {
    use pulldown_cmark::CowStr;

    use super::source_range;
    use crate::{fmt::fmts, fmt::fmts_both, fmt::CmarkToCmarkOptions, fmt::Event, fmt::Parser, fmt::Tag, fmt::TagEnd};

    fn run_test_on_each_special_char(f: impl Fn(String, CowStr)) {
        for c in CmarkToCmarkOptions::default().special_characters().chars() {
            let s = format!(r#"\{c}"#);
            f(s, c.to_string().into());
        }
    }

    #[test]
    fn it_does_not_recreate_escapes_for_underscores_in_the_middle_of_a_word() {
        assert_eq!(
            fmts("\\_hello_world_").0,
            "\\_hello_world\\_" // it actually makes mal-formatted markdown better
        );
    }

    #[test]
    fn it_preserves_underscores_escapes() {
        assert_eq!(source_range::fmts("\\_hello_world_").0, "\\_hello_world_");
    }

    #[test]
    fn it_recreates_escapes_for_known_special_characters_at_the_beginning_of_the_word() {
        run_test_on_each_special_char(|escaped_special_character, _| {
            assert_eq!(fmts_both(&escaped_special_character).0, escaped_special_character);
        });
    }

    #[test]
    fn are_not_needed_for_underscores_within_a_word_and_no_spaces() {
        let e: Vec<_> = Parser::new("hello_there_and__hello again_").collect();
        assert_eq!(
            e,
            vec![
                Event::Start(Tag::Paragraph),
                Event::Text("hello_there_and__hello again".into()),
                Event::Text("_".into()),
                Event::End(TagEnd::Paragraph),
            ]
        );
    }

    #[test]
    fn would_be_needed_for_single_backticks() {
        let e: Vec<_> = Parser::new(r"\`hi`").collect();
        assert_eq!(
            e,
            vec![
                Event::Start(Tag::Paragraph),
                Event::Text("`".into()),
                Event::Text("hi".into()),
                Event::Text("`".into()),
                Event::End(TagEnd::Paragraph),
            ]
        );
    }

    #[test]
    fn it_escapes_closing_square_brackets() {
        assert_eq!(
            fmts_both(r"[\[1\]](http://example.com)").0,
            r"[\[1\]](http://example.com)"
        );
    }

    #[test]
    fn link_titles() {
        // See https://spec.commonmark.org/0.30/#link-title for the rules around
        // link titles and the characters they may contain
        assert_eq!(
            fmts_both(r#"[link](http://example.com "'link title'")"#).0,
            r#"[link](http://example.com "'link title'")"#
        );
        assert_eq!(
            fmts_both(r#"[link](http://example.com "\\\"link \\ title\"")"#).0,
            r#"[link](http://example.com "\\\"link \\ title\"")"#
        );
        assert_eq!(
            fmts_both(r#"[link](http://example.com "\"link title\"")"#).0,
            r#"[link](http://example.com "\"link title\"")"#
        );
        assert_eq!(
            fmts_both(r#"[link](http://example.com '"link title"')"#).0,
            r#"[link](http://example.com "\"link title\"")"#
        );
        assert_eq!(
            fmts_both(r"[link](http://example.com '\'link title\'')").0,
            r#"[link](http://example.com "'link title'")"#
        );
        assert_eq!(
            fmts_both(r"[link](http://example.com (\(link title\)))").0,
            r#"[link](http://example.com "(link title)")"#
        );
        assert_eq!(
            fmts_both(r"[link](http://example.com (你好👋))").0,
            r#"[link](http://example.com "你好👋")"#
        );
    }

    #[test]
    fn it_does_esscape_lone_square_brackets_in_text() {
        assert_eq!(
            fmts("] a closing bracket does nothing").0,
            "\\] a closing bracket does nothing"
        );
    }

    #[test]
    fn it_does_not_escape_lone_square_brackets_in_text_if_the_source_does_not() {
        assert_eq!(
            source_range::fmts("] a closing bracket does nothing").0,
            "] a closing bracket does nothing"
        );
    }

    #[test]
    fn make_special_characters_into_text_blocks() {
        let e: Vec<_> = Parser::new(r"hello\*there*and\*\*hello again\*\*").collect();
        assert_eq!(
            e,
            vec![
                Event::Start(Tag::Paragraph),
                Event::Text("hello".into()),
                Event::Text("*there".into()),
                Event::Text("*".into()),
                Event::Text("and".into()),
                Event::Text("*".into()),
                Event::Text("*hello again".into()),
                Event::Text("*".into()),
                Event::Text("*".into()),
                Event::End(TagEnd::Paragraph),
            ]
        );
    }

    #[test]
    fn would_be_needed_for_asterisks_within_a_word_and_no_spaces() {
        let e: Vec<_> = Parser::new("hello*there*and**hello again**").collect();
        assert_eq!(
            e,
            vec![
                Event::Start(Tag::Paragraph),
                Event::Text("hello".into()),
                Event::Start(Tag::Emphasis),
                Event::Text("there".into()),
                Event::End(TagEnd::Emphasis),
                Event::Text("and".into()),
                Event::Start(Tag::Strong),
                Event::Text("hello again".into()),
                Event::End(TagEnd::Strong),
                Event::End(TagEnd::Paragraph),
            ]
        );
    }

    #[test]
    fn are_not_specifically_provided_as_events() {
        run_test_on_each_special_char(|s, c| {
            let e: Vec<_> = Parser::new(&s).collect();
            assert_eq!(
                e,
                vec![
                    Event::Start(Tag::Paragraph),
                    Event::Text(c.to_string().into()),
                    Event::End(TagEnd::Paragraph),
                ]
            );
        });
    }

    #[test]
    fn entity_escape_is_not_code_block_indent() {
        source_range::assert_events_eq("&#9;foo");
        source_range::assert_events_eq("&#32;   foo");
        source_range::assert_events_eq(" * &#32;   foo\n * &#9;foo");
    }
}

mod list {
    use super::{fmtes, fmts_both, fmts_with_options, CmarkToCmarkOptions, Event, State, TagEnd};
    use indoc::indoc;

    #[test]
    fn it_pops_one_item_from_the_lists_stack_for_each_end_list() {
        let mut first = State::default();
        first.list_stack = vec![None, None];

        let mut second = State::default();
        second.list_stack = vec![None];

        assert_eq!(fmtes(&[Event::End(TagEnd::List(false))], first,).1, second);
    }

    #[test]
    fn ordered_and_unordered_nested_and_ordered() {
        let mut state = State::default();
        state.newlines_before_start = 2;
        assert_eq!(
            fmts_both("1. *b*\n   * *b*\n1. c"),
            ("1. *b*\n   * *b*\n1. c".into(), state)
        );
    }

    #[test]
    fn ordered_and_multiple_unordered() {
        let mut state = State::default();
        state.newlines_before_start = 2;
        assert_eq!(
            fmts_both("11. *b*\n    * *b*\n    * c"),
            ("11. *b*\n    * *b*\n    * c".into(), state)
        );
    }

    #[test]
    fn unordered_ordered_unordered() {
        assert_eq!(fmts_both("* a\n  1. b\n* c").0, "* a\n  1. b\n* c",);
    }

    #[test]
    fn ordered_and_unordered_nested() {
        let mut state = State::default();
        state.newlines_before_start = 2;
        assert_eq!(fmts_both("1. *b*\n   * *b*"), ("1. *b*\n   * *b*".into(), state));
    }

    #[test]
    fn unordered() {
        let mut state = State::default();
        state.newlines_before_start = 2;
        assert_eq!(fmts_both("* a\n* b"), ("* a\n* b".into(), state));
    }

    #[test]
    fn unordered_with_custom() {
        let custom_options = CmarkToCmarkOptions {
            list_token: '-',
            ..Default::default()
        };

        let original = "* a\n* b";
        let (s, _) = fmts_with_options(original, custom_options);

        assert_eq!(s, "- a\n- b".to_string());
    }

    #[test]
    fn ordered() {
        let mut state = State::default();
        state.newlines_before_start = 2;
        assert_eq!(fmts_both("2. a\n2. b"), ("2. a\n2. b".into(), state));
    }

    #[test]
    fn change_ordered_list_token() {
        let custom_options = CmarkToCmarkOptions {
            ordered_list_token: ')',
            ..Default::default()
        };
        let mut state = State::default();
        state.newlines_before_start = 2;
        assert_eq!(
            fmts_with_options("2. a\n2. b", custom_options),
            ("2) a\n2) b".into(), state)
        );
    }

    #[test]
    fn increment_ordered_list_bullets() {
        let custom_options = CmarkToCmarkOptions {
            increment_ordered_list_bullets: true,
            ..Default::default()
        };
        let mut state = State::default();
        state.newlines_before_start = 2;
        assert_eq!(
            fmts_with_options("2. a\n2. b\n2. c", custom_options),
            ("2. a\n3. b\n4. c".into(), state)
        );
    }

    #[test]
    fn nested_increment_ordered_list_bullets() {
        let custom_options = CmarkToCmarkOptions {
            increment_ordered_list_bullets: true,
            ..Default::default()
        };
        let input = indoc!(
            "
        1. level 1
           1. level 2
              1. level 3
              1. level 3
           1. level 2
        1. level 1"
        );

        let expected = indoc!(
            "
        1. level 1
           1. level 2
              1. level 3
              2. level 3
           2. level 2
        2. level 1"
        );
        let mut state = State::default();
        state.newlines_before_start = 2;
        assert_eq!(fmts_with_options(input, custom_options), (expected.into(), state));
    }

    #[test]
    fn nested_increment_ordered_list_bullets_change_ordered_list_token() {
        let custom_options = CmarkToCmarkOptions {
            increment_ordered_list_bullets: true,
            ordered_list_token: ')',
            ..Default::default()
        };
        let input = indoc!(
            "
        1. level 1
           1. level 2
              1. level 3
              1. level 3
           1. level 2
        1. level 1
        1. level 1
           1. level 2
              1. level 3
              1. level 3
           1. level 2
        1. level 1"
        );

        let expected = indoc!(
            "
        1) level 1
           1) level 2
              1) level 3
              2) level 3
           2) level 2
        2) level 1
        3) level 1
           1) level 2
              1) level 3
              2) level 3
           2) level 2
        4) level 1"
        );
        let mut state = State::default();
        state.newlines_before_start = 2;
        assert_eq!(fmts_with_options(input, custom_options), (expected.into(), state));
    }

    #[test]
    fn checkboxes() {
        assert_eq!(
            fmts_both(indoc!(
                "
            * [ ] foo
            * [x] bar
            "
            ))
            .0,
            "* [ ] foo\n* [x] bar",
        );
    }
}

mod heading {
    use super::assert_events_eq_both;

    #[test]
    fn heading_with_classes_and_attrs() {
        assert_events_eq_both("# Heading { #id .class1 key1=val1 .class2 }");
        assert_events_eq_both("# Heading { #id .class1 .class2 key1=val1 key2 }");
    }
    #[test]
    fn heading_with_hashes_at_end() {
        assert_events_eq_both("Heading #\n====");
        assert_events_eq_both("Heading \\#\n====");
        assert_events_eq_both("# Heading \\#");
    }
}

mod frontmatter {
    use pulldown_cmark::{Options, Parser};
    use pulldown_cmark_to_cmark::{cmark, cmark_with_options};

    #[test]
    fn yaml_frontmatter_should_be_supported() {
        let input = "---
key1: value1
key2: value2
---

# Frontmatter should be supported";

        let mut opts = Options::empty();
        opts.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);
        let events = Parser::new_ext(input, opts);

        let mut output = String::new();
        let state = cmark(events, &mut output).unwrap();
        state.finalize(&mut output).unwrap();

        assert_eq!(input, output);
    }

    #[test]
    fn toml_frontmatter_should_be_supported() {
        let input = "+++
key = value1
key = value2
+++

# Frontmatter should be supported";

        let mut opts = Options::empty();
        opts.insert(Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS);

        let events = Parser::new_ext(input, opts);
        let mut output = String::new();
        let state = cmark(events, &mut output).unwrap();
        state.finalize(&mut output).unwrap();

        assert_eq!(input, output);
    }

    #[test]
    fn yaml_frontmatter_supports_newline_option() {
        let mut newlines = String::new();

        for i in 0..10 {
            let input = format!(
                "---
key: value1
key: value2
---{newlines}
# Frontmatter should be supported"
            );

            let mut opts = Options::empty();
            opts.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);

            let events = Parser::new_ext(&input, opts);
            let mut output = String::new();
            let state = cmark_with_options(
                events,
                &mut output,
                pulldown_cmark_to_cmark::Options {
                    newlines_after_metadata: i,
                    ..Default::default()
                },
            )
            .unwrap();
            state.finalize(&mut output).unwrap();

            assert_eq!(input, output);
            newlines.push('\n');
        }
    }

    #[test]
    fn toml_frontmatter_supports_newline_option() {
        let mut newlines = String::new();

        for i in 0..10 {
            let input = format!(
                "+++
key = value1
key = value2
+++{newlines}
# Frontmatter should be supported"
            );

            let mut opts = Options::empty();
            opts.insert(Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS);

            let events = Parser::new_ext(&input, opts);
            let mut output = String::new();
            let state = cmark_with_options(
                events,
                &mut output,
                pulldown_cmark_to_cmark::Options {
                    newlines_after_metadata: i,
                    ..Default::default()
                },
            )
            .unwrap();
            state.finalize(&mut output).unwrap();

            assert_eq!(input, output);
            newlines.push('\n');
        }
    }
}

mod definition_list {
    use super::assert_events_eq;

    #[test]
    fn round_trip() {
        let input = r"First Term
: This is the definition of the first term.

Second Term
: This is one definition of the second term.
: This is another definition of the second term.";

        assert_events_eq(input);
    }
}

mod source_range {
    // Copied from `fmt.rs`.

    use pulldown_cmark::{utils::TextMergeStream, Options, Parser};
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

        let before_events = TextMergeStream::new(Parser::new_ext(s, Options::all()));
        let after_events = TextMergeStream::new(Parser::new_ext(&buf, Options::all()));
        assert_eq!(before_events.collect::<Vec<_>>(), after_events.collect::<Vec<_>>());
    }
}
