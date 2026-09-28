use pulldown_cmark::{utils::TextMergeStream, Event, Options, Parser, Tag, TagEnd};
pub use pulldown_cmark_to_cmark::{cmark, cmark_with_options, Options as CmarkToCmarkOptions, State};

fn fmts_both(s: &str) -> String {
    let output = fmts(s);
    let with_source = source_range::fmts(s);
    assert_eq!(
        TextMergeStream::new(Parser::new_ext(&output, Options::all())).collect::<Vec<_>>(),
        TextMergeStream::new(Parser::new_ext(&with_source, Options::all())).collect::<Vec<_>>(),
        "ordinary and source-range output differ for {s:?}",
    );
    output
}

fn fmts(s: &str) -> String {
    let mut output = String::new();
    cmark(Parser::new_ext(s, Options::all()), &mut output).unwrap();
    output
}

fn fmts_with_options(s: &str, options: CmarkToCmarkOptions<'_>) -> String {
    let with_source = source_range::fmts_with_options(s, options.clone());
    let mut output = String::new();
    cmark_with_options(Parser::new_ext(s, Options::all()), &mut output, options).unwrap();
    assert_eq!(
        TextMergeStream::new(Parser::new_ext(&output, Options::all())).collect::<Vec<_>>(),
        TextMergeStream::new(Parser::new_ext(&with_source, Options::all())).collect::<Vec<_>>(),
    );
    output
}

fn fmte<'a>(events: impl AsRef<[Event<'a>]>) -> String {
    let mut output = String::new();
    cmark(events.as_ref(), &mut output).unwrap();
    output
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
    use super::fmts_both;

    #[test]
    fn after_some_types_it_has_multiple_newlines() {
        for md in &["paragraph", "## headline", "\n````\n````", "---"] {
            assert_eq!(fmts_both(md), String::from(*md));
        }
    }
}

mod inline_elements {
    use crate::fmt::fmts_with_options;

    use super::source_range;
    use super::{fmts_both, CmarkToCmarkOptions};

    #[test]
    fn image() {
        assert_eq!(fmts_both("![a](b)\n![c][d]\n\n[d]: e"), "![a](b)\n![c][d]\n\n[d]: e");
    }

    #[test]
    fn image_collapsed() {
        assert_eq!(
            fmts_both("![c][d]\n\n![c][]![c][]\n\n[d]: e\n[c]: f"),
            "![c][d]\n\n![c][]![c][]\n\n[d]: e\n[c]: f"
        );
    }

    #[test]
    fn footnote() {
        assert_eq!(fmts_both("a [^b]\n\n[^b]: c"), "a [^b]\n\n[^b]: c");
    }

    #[test]
    fn multiline_footnote() {
        assert_eq!(
            fmts_both("a [^b]\n\n[^b]: this is\n    one footnote"),
            "a [^b]\n\n[^b]: this is\n    one footnote",
        );
    }

    #[test]
    fn autolinks_are_fully_resolved() {
        assert_eq!(fmts_both("<http://a/b>"), "<http://a/b>",);
    }

    #[test]
    fn links() {
        {
            assert_eq!(fmts_both("[a](b)\n[c][d]\n\n[d]: e"), "[a](b)\n[c][d]\n\n[d]: e");
        }
    }

    #[test]
    fn links_collapsed() {
        assert_eq!(
            fmts_both("[c][d]\n\n[c][][c][]\n\n[d]: e\n[c]: f"),
            "[c][d]\n\n[c][][c][]\n\n[d]: e\n[c]: f"
        );
    }

    #[test]
    fn shortcut_links() {
        {
            assert_eq!(fmts_both("[a](b)\n[c]\n\n[c]: e"), "[a](b)\n[c]\n\n[c]: e");
        }
    }

    #[test]
    fn brackets_in_link_labels_are_escaped() {
        for markdown in [
            "[ref\\[\\]]\n\n[ref\\[\\]]: https://github.com/",
            "[text][ref\\[\\]]\n\n[ref\\[\\]]: https://github.com/",
            "![ref\\[\\]]\n\n[ref\\[\\]]: https://github.com/",
        ] {
            assert_eq!(fmts_both(markdown), markdown);
        }
    }

    #[test]
    fn shortcut_code_links() {
        assert_eq!(fmts_both("[a](b)\n[`c`]\n\n[`c`]: e"), "[a](b)\n[`c`]\n\n[`c`]: e");
    }

    #[test]
    fn multiple_shortcut_links() {
        assert_eq!(
            fmts_both("[a](b)\n[c] [d]\n\n[c]: e\n[d]: f"),
            "[a](b)\n[c] [d]\n\n[c]: e\n[d]: f"
        );
    }

    #[test]
    fn various() {
        assert_eq!(
            fmts_both("*a* b **c**\n<br>\nd\n\ne `c`"),
            "*a* b **c**\n<br>\nd\n\ne `c`"
        );
    }

    #[test]
    fn various_with_custom_options() {
        let custom_options = CmarkToCmarkOptions {
            emphasis_token: '_',
            code_block_token: '~',
            ..Default::default()
        };

        let s = fmts_with_options("_a_ b **c**\n<br>\nd\n\ne `c`", custom_options);

        assert_eq!(s, "_a_ b **c**\n<br>\nd\n\ne `c`".to_string());
    }

    #[test]
    fn strikethrough() {
        assert_eq!(fmts_both("~~strikethrough~~"), "~~strikethrough~~",);
    }

    #[test]
    fn code_double_backtick() {
        assert_eq!(
            fmts_both("lorem ``ipsum `dolor` sit`` amet"),
            "lorem ``ipsum `dolor` sit`` amet"
        );
    }

    #[test]
    fn code_triple_backtick() {
        assert_eq!(
            fmts_both("lorem ```ipsum ``dolor`` sit``` amet"),
            "lorem ```ipsum ``dolor`` sit``` amet"
        );
    }

    #[test]
    fn code_backtick_normalization() {
        // The minimum amount of backticks are inserted.

        assert_eq!(
            fmts_both("lorem ```ipsum ` dolor``` amet"),
            "lorem ``ipsum ` dolor`` amet"
        );
    }

    #[test]
    fn code_leading_trailing_backtick() {
        // Spaces are inserted if the inline code starts or ends with
        // a backtick.
        {
            assert_eq!(fmts_both("`` `lorem ``   `` ipsum` ``"), "`` `lorem ``   `` ipsum` ``");
        }
    }

    #[test]
    fn code_spaces_before_backtick() {
        //  No space is inserted if it is not needed.
        {
            assert_eq!(fmts_both("` lorem `   ` `"), "`lorem`   ` `");
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
            assert_eq!(source_range::fmts("[`Vec`]"), "[`Vec`]");
        }
    }

    #[test]
    fn preserve_less_than_sign_escape() {
        // `<` is not escaped if not escaped in the source.

        assert_eq!(source_range::fmts("a < 1"), "a < 1");
        // `<` is escaped if escaped in the source.

        assert_eq!(source_range::fmts(r"a \< 1"), r"a \< 1");
    }
}

mod blockquote {
    use super::{assert_events_eq_both, fmts_both};
    use indoc::indoc;

    #[test]
    fn with_html() {
        let s = indoc!(
            "
             > <table>
             > </table>
             "
        );

        assert_events_eq_both(s);

        assert_eq!(fmts_both(s), "\n > \n > <table>\n > </table>\n");
    }

    #[test]
    fn with_inlinehtml() {
        assert_eq!(fmts_both(" > <br>"), "\n > \n > <br>");
    }

    #[test]
    fn with_plaintext_in_html() {
        assert_eq!(fmts_both("<del>\n*foo*\n</del>"), "<del>\n*foo*\n</del>");
    }

    #[test]
    fn with_markdown_nested_in_html() {
        assert_eq!(fmts_both("<del>\n\n*foo*\n\n</del>"), "<del>\n\n*foo*\n\n</del>");
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

        assert_eq!(fmts_both(s), "\n > \n > ````a\n > t1\n > t2\n > ````",);
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

        assert_eq!(fmts_both(s), "\n > \n > a\n > \n >  > \n >  > b\n > \n > c",);
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

        assert_eq!(fmts_both(s), "\n > \n >  > \n >  > foo\n >  > bar\n >  > baz",);
    }

    #[test]
    fn simple() {
        // Inlining this rather than using indoc because format-on-save with
        // rustfmt tries to strip the trailing spaces after `b` otherwise in
        // some editors.
        let s = "> a\n> b  \n> c\n";

        assert_events_eq_both(s);

        {
            assert_eq!(fmts_both(s), "\n > \n > a\n > b  \n > c");
        }
    }

    #[test]
    fn empty() {
        let s = " > ";

        assert_events_eq_both(s);

        {
            assert_eq!(fmts_both(s), "\n > ");
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

        assert_eq!(fmts_both(s), "\n > \n > foo\n\n > \n > bar");
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

        assert_eq!(fmts_both(s), "\n > \n > foo\n > baz\n\n > \n > bar");
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

        assert_eq!(
            fmts_both(s),
            "* \n   > \n   > * foo\n   >   * baz\n   >\n  * \n     > \n     > bar"
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
    use super::{fmts_both, fmts_with_options, CmarkToCmarkOptions};

    #[test]
    fn simple_and_paragraph() {
        assert_eq!(
            fmts_both("````hi\nsome\ntext\n````\na"),
            "\n````hi\nsome\ntext\n````\n\na"
        );
    }

    #[test]
    fn empty() {
        {
            assert_eq!(fmts_both("```\n```"), "\n````\n````");
        }
    }

    #[test]
    fn simple() {
        assert_eq!(fmts_both("```hi\nsome\ntext\n```"), "\n````hi\nsome\ntext\n````");
    }

    #[test]
    fn simple_other_syntax() {
        assert_eq!(fmts_both("~~~hi\nsome\ntext\n~~~"), "\n````hi\nsome\ntext\n````");
    }

    #[test]
    fn simple_other_syntax_with_custom() {
        let custom_options = CmarkToCmarkOptions {
            code_block_token: '~',
            ..Default::default()
        };

        let original = "~~~hi\nsome\ntext\n~~~";
        let s = fmts_with_options(original, custom_options);

        assert_eq!(s, "\n~~~~hi\nsome\ntext\n~~~~".to_string());
    }

    #[test]
    fn indented() {
        assert_eq!(
            fmts_both("    first\n    second\nthird"),
            "\n    first\n    second\n\nthird"
        );
    }

    #[test]
    fn html_indented() {
        assert_eq!(
            fmts_both("  <!-- foo -->\n\n    <!-- foo -->"),
            "  <!-- foo -->\n\n    <!-- foo -->\n"
        );
    }
}

mod table {
    use indoc::indoc;
    use pretty_assertions::assert_eq;

    use super::fmte;

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

        let generated_markdown = fmte(&original_events);

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

        let generated_markdown = fmte(&original_events);

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

        let generated_markdown = fmte(&original_events);

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
            fmts("\\_hello_world_"),
            "\\_hello_world\\_" // it actually makes mal-formatted markdown better
        );
    }

    #[test]
    fn it_preserves_underscores_escapes() {
        assert_eq!(source_range::fmts("\\_hello_world_"), "\\_hello_world_");
    }

    #[test]
    fn it_recreates_escapes_for_known_special_characters_at_the_beginning_of_the_word() {
        run_test_on_each_special_char(|escaped_special_character, _| {
            assert_eq!(fmts_both(&escaped_special_character), escaped_special_character);
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
            fmts_both(r"[\[1\]](http://example.com)"),
            r"[\[1\]](http://example.com)"
        );
    }

    #[test]
    fn link_titles() {
        // See https://spec.commonmark.org/0.30/#link-title for the rules around
        // link titles and the characters they may contain
        assert_eq!(
            fmts_both(r#"[link](http://example.com "'link title'")"#),
            r#"[link](http://example.com "'link title'")"#
        );
        assert_eq!(
            fmts_both(r#"[link](http://example.com "\\\"link \\ title\"")"#),
            r#"[link](http://example.com "\\\"link \\ title\"")"#
        );
        assert_eq!(
            fmts_both(r#"[link](http://example.com "\"link title\"")"#),
            r#"[link](http://example.com "\"link title\"")"#
        );
        assert_eq!(
            fmts_both(r#"[link](http://example.com '"link title"')"#),
            r#"[link](http://example.com "\"link title\"")"#
        );
        assert_eq!(
            fmts_both(r"[link](http://example.com '\'link title\'')"),
            r#"[link](http://example.com "'link title'")"#
        );
        assert_eq!(
            fmts_both(r"[link](http://example.com (\(link title\)))"),
            r#"[link](http://example.com "(link title)")"#
        );
        assert_eq!(
            fmts_both(r"[link](http://example.com (你好👋))"),
            r#"[link](http://example.com "你好👋")"#
        );
    }

    #[test]
    fn it_does_esscape_lone_square_brackets_in_text() {
        assert_eq!(
            fmts("] a closing bracket does nothing"),
            "\\] a closing bracket does nothing"
        );
    }

    #[test]
    fn it_does_not_escape_lone_square_brackets_in_text_if_the_source_does_not() {
        assert_eq!(
            source_range::fmts("] a closing bracket does nothing"),
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
    use super::{
        assert_events_eq_both, cmark, fmts_both, fmts_with_options, CmarkToCmarkOptions, Event, Options, Parser, State,
        TagEnd, TextMergeStream,
    };
    use indoc::indoc;

    #[test]
    fn nested_list_followed_by_paragraph() {
        let input = indoc!(
            "
            1. item

               * a
               * b

               para"
        );
        assert_eq!(fmts_both(input), "1. item\n   \n   * a\n   * b\n   \n   para");
        assert_events_eq_both(input);
    }

    #[test]
    fn nested_list_followed_by_table() {
        assert_events_eq_both(indoc!(
            "
            1. item

               * a
               * b

               | a | b |
               | - | - |
               | 1 | 2 |"
        ));
    }

    #[test]
    fn nested_list_followed_by_indented_code() {
        // The wide marker ("10000.") puts the inner item's content past the
        // code's indent, so the code is a sibling of the inner list inside the
        // outer item.
        assert_events_eq_both(indoc!(
            "
            1. item

               10000. a

                   code
            "
        ));
    }

    #[test]
    fn nested_list_followed_by_fenced_code_in_tight_item() {
        assert_events_eq_both(indoc!(
            "
            1. item
               * a
               * b
               ```
               code
               ```"
        ));
    }

    #[test]
    fn nested_list_ending_in_empty_item_followed_by_table_in_tight_item() {
        assert_events_eq_both(indoc!(
            "
            * a
              * b
              *
              | x | y |
              | - | - |"
        ));
        assert_events_eq_both(indoc!(
            "
            > * a
            >   * b
            >   *
            >   | x | y |
            >   | - | - |"
        ));
    }

    #[test]
    fn nested_list_ending_in_empty_item_followed_by_indented_code() {
        assert_events_eq_both(indoc!(
            "
            1. item

               *

                   code
            "
        ));
    }

    #[test]
    fn nested_list_ending_in_fenced_code_followed_by_table_in_tight_item() {
        let input = indoc!(
            "
            1. item
               * a
                 ```
                 x
                 ```
               | a | b |
               | - | - |
               | 1 | 2 |"
        );
        // The default `newlines_after_codeblock` of 2 already emits a blank
        // line here, which hides the potential bug this test is trying to
        // detect. Set it to 1.
        let options = CmarkToCmarkOptions {
            newlines_after_codeblock: 1,
            ..Default::default()
        };
        let output = fmts_with_options(input, options);
        let before: Vec<_> = TextMergeStream::new(Parser::new_ext(input, Options::all())).collect();
        let after: Vec<_> = TextMergeStream::new(Parser::new_ext(&output, Options::all())).collect();
        assert_eq!(before, after, "output:\n{output}");
    }

    #[test]
    fn nested_list_followed_by_definition_list() {
        assert_events_eq_both(indoc!(
            "
            1. item
               * a
               * b

               term
               : definition"
        ));
    }

    #[test]
    fn three_level_nested_list_followed_by_table() {
        assert_events_eq_both(indoc!(
            "
            1. one
               * two
                 - three

               | x | y |
               | - | - |"
        ));
    }

    #[test]
    fn nested_list_followed_by_paragraph_across_resume() {
        let input = indoc!(
            "
            1. item

               * a
               * b

               para"
        );
        let events: Vec<_> = Parser::new_ext(input, Options::all()).collect();
        let split = 1 + events
            .iter()
            .position(|event| *event == Event::End(TagEnd::List(false)))
            .expect("input contains a nested unordered list");
        let (before, after) = events.split_at(split);

        let mut output = String::new();
        let mut state = State::default();
        state.process(before, &mut output).unwrap();
        state.process(after, &mut output).unwrap();
        state.finish(&mut output).unwrap();
        assert_eq!(output, "1. item\n   \n   * a\n   * b\n   \n   para");
    }

    #[test]
    fn heading_after_text_in_tight_item() {
        let input = indoc!(
            "
            * item
              # heading"
        );
        assert_eq!(fmts_both(input), "* item\n  # heading");
        assert_events_eq_both(input);
        assert_events_eq_both(indoc!(
            "
            1. item
               # heading"
        ));
    }

    #[test]
    fn heading_after_inline_content_in_tight_item() {
        assert_events_eq_both(indoc!(
            "
            * *item*
              # heading"
        ));
        assert_events_eq_both(indoc!(
            "
            * `item`
              # heading"
        ));
        assert_events_eq_both(indoc!(
            "
            * [x] item
              # heading"
        ));
        assert_events_eq_both(indoc!(
            "
            * item\\
              # heading"
        ));
    }

    #[test]
    fn heading_after_text_in_nested_tight_item() {
        assert_events_eq_both(indoc!(
            "
            * a
              * item
                # heading"
        ));
        assert_events_eq_both(indoc!(
            "
            > * item
            >   # heading"
        ));
    }

    #[test]
    fn rule_after_text_in_tight_item() {
        let input = indoc!(
            "
            * item
              ***"
        );
        assert_eq!(fmts_both(input), "* item\n  ***");
        assert_events_eq_both(input);
    }

    #[test]
    fn html_block_after_text_in_tight_item() {
        assert_events_eq_both(indoc!(
            "
            * item
              <div>
              </div>"
        ));
        assert_events_eq_both(indoc!(
            "
            * item
              <!-- comment -->"
        ));
    }

    #[test]
    fn table_after_text_in_tight_item() {
        assert_events_eq_both(indoc!(
            "
            * item
              | a | b |
              | - | - |"
        ));
    }

    #[test]
    fn footnote_definition_after_text_in_tight_item() {
        assert_events_eq_both(indoc!(
            "
            * item
              [^1]: footnote"
        ));
    }

    #[test]
    fn heading_between_items_in_tight_list() {
        let input = indoc!(
            "
            * a
            * item
              # heading
            * c"
        );
        assert_eq!(fmts_both(input), "* a\n* item\n  # heading\n* c");
        assert_events_eq_both(input);
    }

    #[test]
    fn text_after_heading_in_tight_item() {
        let input = indoc!(
            "
            * # heading
              text"
        );
        assert_eq!(fmts_both(input), "* # heading\n  text");
        assert_events_eq_both(input);
        assert_events_eq_both(indoc!(
            "
            * item
              ## heading
              more"
        ));
    }

    #[test]
    fn consecutive_headings_in_tight_item() {
        assert_events_eq_both(indoc!(
            "
            * item
              # heading
              # heading2"
        ));
    }

    #[test]
    fn inline_content_after_heading_in_tight_item() {
        assert_events_eq_both(indoc!(
            "
            * # heading
              `code`"
        ));
        assert_events_eq_both(indoc!(
            "
            * # heading
              $x$"
        ));
        assert_events_eq_both(indoc!(
            "
            * # heading
              $$x$$"
        ));
        assert_events_eq_both(indoc!(
            "
            * # heading
              *emphasis*"
        ));
        assert_events_eq_both(indoc!(
            "
            * # heading
              [^1]

            [^1]: footnote"
        ));
    }

    #[test]
    fn inline_content_after_html_in_tight_item() {
        for content in ["`code`", "text", "$x$", "$$x$$", "*emphasis*", "<i>inline</i>"] {
            let input = format!("* <!-- comment -->\n  {content}");
            assert_events_eq_both(&input);
        }
        assert_events_eq_both("* <!-- comment -->\n  [^1]\n\n[^1]: footnote");
        assert_events_eq_both("> * <!-- comment -->\n>   `code`");
        assert_events_eq_both("* <!-- comment -->\n\n  `code`");
        assert_events_eq_both("* item\n  <!-- comment -->\n  ```\n  code\n  ```");
    }

    #[test]
    fn fenced_code_in_tight_list() {
        assert_events_eq_both(indoc!(
            "
            * a
            * ```
              x
              ```
            * c"
        ));
        assert_events_eq_both(indoc!(
            "
            * item
              ```
              x
              ```
              more"
        ));
    }

    #[test]
    fn table_in_tight_list() {
        assert_events_eq_both(indoc!(
            "
            * a
            * | x |
              | - |
            * c"
        ));
        assert_events_eq_both(indoc!(
            "
            * a
              | x |
              | - |
              # heading
            * c"
        ));
    }

    #[test]
    fn blockquote_between_items_in_tight_list() {
        assert_events_eq_both(indoc!(
            "
            * a
              > b
              >
            * c"
        ));
        assert_events_eq_both(indoc!(
            "
            - a
              > b
              ```
              c
              ```
            - d"
        ));
    }

    #[test]
    fn text_after_blockquote_in_tight_item() {
        let input = indoc!(
            "
            * a
              > q
              >
              text"
        );
        // Without the empty `>` line, `text` would lazily continue the `q`
        // paragraph and end up inside the block quote.
        assert_eq!(fmts_both(input), "* a\n   > \n   > q\n   >\n  text");
        assert_events_eq_both(input);
        assert_events_eq_both(indoc!(
            "
            * a
              > q
              >
                  code
            "
        ));
        assert_events_eq_both(indoc!(
            "
            * a
              > > q
              >
              text"
        ));
    }

    #[test]
    fn blank_lines_inside_blockquote_in_tight_item_are_kept() {
        assert_events_eq_both(indoc!(
            "
            - a
              > p1
              >
              > p2
            - d"
        ));
        assert_events_eq_both(indoc!(
            "
            - a
              > | x |
              > | - |
              >
              > para
            - d"
        ));
    }

    #[test]
    fn content_after_nested_blockquote_in_tight_item() {
        let input = indoc!(
            "
            * item
              * a
                > q
                >
              text"
        );
        assert_eq!(fmts_both(input), "* item\n  * a\n     > \n     > q\n     >\n  text");
        assert_events_eq_both(input);
        assert_events_eq_both(indoc!(
            "
            * item
              * a
                > q
                >
              | x |
              | - |"
        ));
        assert_events_eq_both(indoc!(
            "
            1. item
               1. a
                  > q
                  >
               text"
        ));
        assert_events_eq_both(indoc!(
            "
            * item
              * a
                * b
                  > q
                  >
              text"
        ));
        assert_events_eq_both(indoc!(
            "
            * item
              > * a
              >   > q
              >
              text"
        ));
    }

    #[test]
    fn loose_nested_list_in_tight_list() {
        assert_events_eq_both(indoc!(
            "
            - a
              - b

                c
            - d"
        ));
    }

    #[test]
    fn heading_in_nested_tight_list() {
        assert_events_eq_both(indoc!(
            "
            * a
              * b
                # heading
              * c
            * d"
        ));
        assert_events_eq_both(indoc!(
            "
            > * a
            >   # heading
            > * c"
        ));
    }

    #[test]
    fn loose_list_with_blocks_keeps_blank_lines() {
        let input = indoc!(
            "
            * a

              # heading
            * c"
        );
        assert_eq!(fmts_both(input), "* a\n  \n  # heading\n\n* c");
        assert_events_eq_both(input);
        // When we write the gap before `c`, we don't yet know whether the list
        // is tight. In this case, that's correct because the list is loose.
        // (But fixing this properly would require some kind of lookahead.)
        assert_events_eq_both(indoc!(
            "
            * # heading

            * c"
        ));
        assert_events_eq_both(indoc!(
            "
            * ```
              x
              ```

            * c"
        ));
    }

    #[test]
    fn output_is_independent_of_resume_points() {
        for input in [
            "* item\n  # heading",
            "* <!-- comment -->\n  `code`",
            "`term`\n\n: def",
            "* `term`\n\n  : def",
            "* term\n  : > quote\n\n  text",
            "* a\n* item\n  # heading\n* c",
            "* a\n  > q\n  >\n  text",
            "* item\n  * a\n    > q\n    >\n  text",
            "- a\n  - b\n\n    c\n- d",
        ] {
            let events: Vec<_> = Parser::new_ext(input, Options::all()).collect();
            let mut expected = String::new();
            cmark(events.iter(), &mut expected).unwrap();

            for split in 0..=events.len() {
                let (before, after) = events.split_at(split);
                let mut output = String::new();
                let mut state = State::default();
                state.process(before, &mut output).unwrap();
                state.process(after, &mut output).unwrap();
                state.finish(&mut output).unwrap();
                assert_eq!(output, expected, "input {input:?} split at event {split}");
            }
        }
    }

    #[test]
    fn ordered_and_unordered_nested_and_ordered() {
        assert_eq!(fmts_both("1. *b*\n   * *b*\n1. c"), "1. *b*\n   * *b*\n1. c");
    }

    #[test]
    fn ordered_and_multiple_unordered() {
        assert_eq!(fmts_both("11. *b*\n    * *b*\n    * c"), "11. *b*\n    * *b*\n    * c");
    }

    #[test]
    fn unordered_ordered_unordered() {
        assert_eq!(fmts_both("* a\n  1. b\n* c"), "* a\n  1. b\n* c",);
    }

    #[test]
    fn ordered_and_unordered_nested() {
        assert_eq!(fmts_both("1. *b*\n   * *b*"), "1. *b*\n   * *b*");
    }

    #[test]
    fn unordered() {
        assert_eq!(fmts_both("* a\n* b"), "* a\n* b");
    }

    #[test]
    fn unordered_with_custom() {
        let custom_options = CmarkToCmarkOptions {
            list_token: '-',
            ..Default::default()
        };

        let original = "* a\n* b";
        let s = fmts_with_options(original, custom_options);

        assert_eq!(s, "- a\n- b".to_string());
    }

    #[test]
    fn ordered() {
        assert_eq!(fmts_both("2. a\n2. b"), "2. a\n2. b");
    }

    #[test]
    fn change_ordered_list_token() {
        let custom_options = CmarkToCmarkOptions {
            ordered_list_token: ')',
            ..Default::default()
        };

        assert_eq!(fmts_with_options("2. a\n2. b", custom_options), "2) a\n2) b");
    }

    #[test]
    fn increment_ordered_list_bullets() {
        let custom_options = CmarkToCmarkOptions {
            increment_ordered_list_bullets: true,
            ..Default::default()
        };

        assert_eq!(
            fmts_with_options("2. a\n2. b\n2. c", custom_options),
            "2. a\n3. b\n4. c"
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

        assert_eq!(fmts_with_options(input, custom_options), expected);
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

        assert_eq!(fmts_with_options(input, custom_options), expected);
    }

    #[test]
    fn checkboxes() {
        assert_eq!(
            fmts_both(indoc!(
                "
            * [ ] foo
            * [x] bar
            "
            )),
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
        let mut state = cmark(events, &mut output).unwrap();
        state.finish(&mut output).unwrap();

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
        let mut state = cmark(events, &mut output).unwrap();
        state.finish(&mut output).unwrap();

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
            let mut state = cmark_with_options(
                events,
                &mut output,
                pulldown_cmark_to_cmark::Options {
                    newlines_after_metadata: i,
                    ..Default::default()
                },
            )
            .unwrap();
            state.finish(&mut output).unwrap();

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
            let mut state = cmark_with_options(
                events,
                &mut output,
                pulldown_cmark_to_cmark::Options {
                    newlines_after_metadata: i,
                    ..Default::default()
                },
            )
            .unwrap();
            state.finish(&mut output).unwrap();

            assert_eq!(input, output);
            newlines.push('\n');
        }
    }
}

mod definition_list {
    use super::{assert_events_eq, assert_events_eq_both};

    #[test]
    fn ending_definition_does_not_restart_blockquote() {
        assert_events_eq_both("* term\n  : > quote\n\n  text");
        assert_events_eq_both("* outer\n  * term\n    : > quote\n\n  text");
        assert_events_eq_both("* term\n  : * item\n      > quote\n\n  text");
        assert_events_eq_both("> * term\n>   : > quote\n>\n>   text");
    }

    #[test]
    fn loose_definitions_keep_paragraphs() {
        for term in ["`term`", "term", "*term*", "$term$", "$$term$$", "[term](url)"] {
            assert_events_eq_both(&format!("{term}\n\n: def"));
            assert_events_eq_both(&format!("{term}\n: def"));
        }
        for input in [
            "`term`\n\n: first\n: second",
            "`term`\n: first\n\n  second",
            "`term`\n: first\n\n`other`\n\n: second",
            "* `term`\n\n  : def",
            "> `term`\n>\n> : def",
            "> term\n> : first\n>\n>   second",
            "`term`\n:",
        ] {
            assert_events_eq_both(input);
        }
    }

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
    use pulldown_cmark::{utils::TextMergeStream, Options, Parser};
    use pulldown_cmark_to_cmark::{
        cmark_with_source_range, cmark_with_source_range_and_options, Options as CmarkToCmarkOptions,
    };

    pub fn fmts(s: &str) -> String {
        let mut output = String::new();
        cmark_with_source_range(
            Parser::new_ext(s, Options::all())
                .into_offset_iter()
                .map(|(event, range)| (event, Some(range))),
            s,
            &mut output,
        )
        .unwrap();
        output
    }

    pub fn fmts_with_options(s: &str, options: CmarkToCmarkOptions<'_>) -> String {
        let mut output = String::new();
        cmark_with_source_range_and_options(
            Parser::new_ext(s, Options::all())
                .into_offset_iter()
                .map(|(event, range)| (event, Some(range))),
            s,
            &mut output,
            options,
        )
        .unwrap();
        output
    }

    pub fn assert_events_eq(s: &str) {
        let output = fmts(s);
        assert_eq!(
            TextMergeStream::new(Parser::new_ext(s, Options::all())).collect::<Vec<_>>(),
            TextMergeStream::new(Parser::new_ext(&output, Options::all())).collect::<Vec<_>>(),
            "source-range round trip failed for {s:?}: {output:?}",
        );
    }
}
