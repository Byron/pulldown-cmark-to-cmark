// Copied from `fmt.rs`.

#[macro_use]
extern crate indoc;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use pulldown_cmark_to_cmark::{
    cmark_resume_with_source_range_and_options, cmark_with_source_range, Options as CmarkToCmarkOptions, State,
};

fn fmts(s: &str) -> (String, State<'_>) {
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

fn fmts_with_options<'a>(s: &'a str, options: CmarkToCmarkOptions<'a>) -> (String, State<'a>) {
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
fn assert_events_eq(s: &str) {
    let _before_events = Parser::new_ext(s, Options::all());

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
    println!("{}", buf);
    assert_eq!(before_events.collect::<Vec<_>>(), after_events.collect::<Vec<_>>());
}

mod lazy_newlines {
    use super::{fmts, State};

    #[test]
    fn after_some_types_it_has_multiple_newlines() {
        for md in &["paragraph", "## headline", "\n````\n````", "---"] {
            assert_eq!(
                fmts(md),
                (
                    String::from(*md),
                    State {
                        newlines_before_start: 2,
                        ..Default::default()
                    }
                )
            )
        }
    }
}

mod inline_elements {
    use crate::fmts_with_options;

    use super::{fmts, CmarkToCmarkOptions, State};

    #[test]
    fn image() {
        assert_eq!(
            fmts("![a](b)\n![c][d]\n\n[d]: e"),
            (
                "![a](b)\n![c](e)".into(),
                State {
                    newlines_before_start: 2,
                    ..Default::default()
                }
            )
        )
    }

    #[test]
    fn footnote() {
        assert_eq!(
            fmts("a [^b]\n\n[^b]: c"),
            (
                "a [^b]\n\n[^b]: c".into(),
                State {
                    newlines_before_start: 2,
                    ..Default::default()
                }
            )
        )
    }

    #[test]
    fn multiline_footnote() {
        assert_eq!(
            fmts("a [^b]\n\n[^b]: this is\n    one footnote").0,
            "a [^b]\n\n[^b]: this is\n    one footnote",
        )
    }

    #[test]
    fn autolinks_are_fully_resolved() {
        assert_eq!(fmts("<http://a/b>").0, "<http://a/b>",)
    }

    #[test]
    fn links() {
        assert_eq!(
            fmts("[a](b)\n[c][d]\n\n[d]: e"),
            (
                "[a](b)\n[c](e)".into(),
                State {
                    newlines_before_start: 2,
                    ..Default::default()
                }
            )
        )
    }

    #[test]
    fn shortcut_links() {
        assert_eq!(
            fmts("[a](b)\n[c]\n\n[c]: e"),
            (
                "[a](b)\n[c]\n\n[c]: e".into(),
                State {
                    newlines_before_start: 2,
                    ..Default::default()
                }
            )
        )
    }

    #[test]
    fn shortcut_code_links() {
        assert_eq!(
            fmts("[a](b)\n[`c`]\n\n[`c`]: e"),
            (
                "[a](b)\n[`c`]\n\n[`c`]: e".into(),
                State {
                    newlines_before_start: 2,
                    ..Default::default()
                }
            )
        )
    }

    #[test]
    fn multiple_shortcut_links() {
        assert_eq!(
            fmts("[a](b)\n[c] [d]\n\n[c]: e\n[d]: f"),
            (
                "[a](b)\n[c] [d]\n\n[c]: e\n[d]: f".into(),
                State {
                    newlines_before_start: 2,
                    ..Default::default()
                }
            )
        )
    }

    #[test]
    fn various() {
        assert_eq!(
            fmts("*a* b **c**\n<br>\nd\n\ne `c`"),
            (
                "*a* b **c**\n<br>\nd\n\ne `c`".into(),
                State {
                    newlines_before_start: 2,
                    ..Default::default()
                }
            )
        )
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
        assert_eq!(
            state,
            State {
                newlines_before_start: 2,
                ..Default::default()
            }
        )
    }

    #[test]
    fn strikethrough() {
        assert_eq!(fmts("~~strikethrough~~").0, "~~strikethrough~~",);
    }

    #[test]
    fn code_double_backtick() {
        assert_eq!(
            fmts("lorem ``ipsum `dolor` sit`` amet"),
            (
                "lorem ``ipsum `dolor` sit`` amet".into(),
                State {
                    newlines_before_start: 2,
                    ..Default::default()
                }
            )
        )
    }

    #[test]
    fn code_triple_backtick() {
        assert_eq!(
            fmts("lorem ```ipsum ``dolor`` sit``` amet"),
            (
                "lorem ```ipsum ``dolor`` sit``` amet".into(),
                State {
                    newlines_before_start: 2,
                    ..Default::default()
                }
            )
        )
    }

    #[test]
    fn code_backtick_normalization() {
        // The minimum amount of backticks are inserted.
        assert_eq!(
            fmts("lorem ```ipsum ` dolor``` amet"),
            (
                "lorem ``ipsum ` dolor`` amet".into(),
                State {
                    newlines_before_start: 2,
                    ..Default::default()
                }
            )
        )
    }

    #[test]
    fn code_leading_trailing_backtick() {
        // Spaces are inserted if the inline code starts or ends with
        // a backtick.
        assert_eq!(
            fmts("`` `lorem ``   `` ipsum` ``"),
            (
                "`` `lorem ``   `` ipsum` ``".into(),
                State {
                    newlines_before_start: 2,
                    ..Default::default()
                }
            )
        )
    }

    #[test]
    fn code_spaces_before_backtick() {
        //  No space is inserted if it is not needed.
        assert_eq!(
            fmts("` lorem `   ` `"),
            (
                "`lorem`   ` `".into(),
                State {
                    newlines_before_start: 2,
                    ..Default::default()
                }
            )
        )
    }

    #[test]
    fn rustdoc_link() {
        // Brackets are not escaped if not escaped in the source.
        assert_eq!(
            fmts("[`Vec`]"),
            (
                "[`Vec`]".into(),
                State {
                    newlines_before_start: 2,
                    ..Default::default()
                }
            )
        )
    }
}

mod blockquote {
    use super::{assert_events_eq, fmts, State};

    #[test]
    fn with_html() {
        let s = indoc!(
            "
             > <table>
             > </table>
             "
        );

        assert_events_eq(s);

        assert_eq!(fmts(s).0, "\n > \n > <table>\n > </table>\n > ")
    }

    #[test]
    fn with_inlinehtml() {
        assert_eq!(fmts(" > <br>").0, "\n > \n > <br>")
    }

    #[test]
    fn with_plaintext_in_html() {
        assert_eq!(fmts("<del>\n*foo*\n</del>").0, "<del>\n*foo*\n</del>")
    }

    #[test]
    fn with_markdown_nested_in_html() {
        assert_eq!(fmts("<del>\n\n*foo*\n\n</del>").0, "<del>\n\n*foo*\n\n</del>")
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

        assert_events_eq(s);

        assert_eq!(fmts(s).0, "\n > \n > ````a\n > t1\n > t2\n > ````",)
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

        assert_events_eq(s);

        assert_eq!(fmts(s).0, "\n > \n > a\n > \n >  > \n >  > b\n > \n > c",)
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

        assert_events_eq(s);

        assert_eq!(fmts(s).0, "\n > \n >  > \n >  > foo\n >  > bar\n >  > baz",)
    }

    #[test]
    fn simple() {
        let s = indoc!(
            "
             > a
             > b  
             > c
             "
        );

        assert_events_eq(s);

        assert_eq!(
            fmts(s),
            (
                "\n > \n > a\n > b  \n > c".into(),
                State {
                    newlines_before_start: 2,
                    ..Default::default()
                }
            )
        )
    }

    #[test]
    fn empty() {
        let s = " > ";

        assert_events_eq(s);

        assert_eq!(
            fmts(s),
            (
                "\n > ".into(),
                State {
                    newlines_before_start: 2,
                    ..Default::default()
                }
            )
        )
    }

    #[test]
    fn with_blank_line() {
        let s = indoc!(
            "
            > foo

            > bar
            "
        );

        assert_events_eq(s);

        assert_eq!(
            fmts(s),
            (
                "\n > \n > foo\n\n > \n > bar".into(),
                State {
                    newlines_before_start: 2,
                    ..Default::default()
                }
            )
        )
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

        assert_events_eq(s);

        assert_eq!(
            fmts(s),
            (
                "\n > \n > foo\n > baz\n\n > \n > bar".into(),
                State {
                    newlines_before_start: 2,
                    ..Default::default()
                }
            )
        )
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

        assert_events_eq(s);

        assert_eq!(
            fmts(s),
            (
                "* \n   > \n   > * foo\n   >   * baz\n  \n  * \n     > \n     > bar".into(),
                State {
                    newlines_before_start: 2,
                    ..Default::default()
                }
            )
        )
    }

    #[test]
    fn complex_nesting() {
        assert_events_eq(indoc!(
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
    use super::{fmts, fmts_with_options, CmarkToCmarkOptions, State};

    #[test]
    fn simple_and_paragraph() {
        assert_eq!(
            fmts("````hi\nsome\ntext\n````\na"),
            (
                "\n````hi\nsome\ntext\n````\n\na".into(),
                State {
                    newlines_before_start: 2,
                    ..Default::default()
                }
            )
        )
    }

    #[test]
    fn empty() {
        assert_eq!(
            fmts("```\n```"),
            (
                "\n````\n````".into(),
                State {
                    newlines_before_start: 2,
                    ..Default::default()
                }
            )
        )
    }

    #[test]
    fn simple() {
        assert_eq!(
            fmts("```hi\nsome\ntext\n```"),
            (
                "\n````hi\nsome\ntext\n````".into(),
                State {
                    newlines_before_start: 2,
                    ..Default::default()
                }
            )
        )
    }

    #[test]
    fn simple_other_syntax() {
        assert_eq!(
            fmts("~~~hi\nsome\ntext\n~~~"),
            (
                "\n````hi\nsome\ntext\n````".into(),
                State {
                    newlines_before_start: 2,
                    ..Default::default()
                }
            )
        )
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
}

mod escapes {
    use pulldown_cmark::CowStr;

    use crate::{fmts, CmarkToCmarkOptions, Event, Parser, Tag, TagEnd};

    fn run_test_on_each_special_char(f: impl Fn(String, CowStr)) {
        for c in CmarkToCmarkOptions::default().special_characters().chars() {
            let s = format!(r#"\{special}"#, special = c);
            f(s, c.to_string().into())
        }
    }

    #[test]
    fn it_preserves_underscores_escapes() {
        assert_eq!(fmts("\\_hello_world_").0, "\\_hello_world_");
    }

    #[test]
    fn it_recreates_escapes_for_known_special_characters_at_the_beginning_of_the_word() {
        run_test_on_each_special_char(|escaped_special_character, _| {
            assert_eq!(fmts(&escaped_special_character).0, escaped_special_character);
        })
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
        )
    }

    #[test]
    fn would_be_needed_for_single_backticks() {
        let e: Vec<_> = Parser::new(r#"\`hi`"#).collect();
        assert_eq!(
            e,
            vec![
                Event::Start(Tag::Paragraph),
                Event::Text("`".into()),
                Event::Text("hi".into()),
                Event::Text("`".into()),
                Event::End(TagEnd::Paragraph),
            ]
        )
    }

    #[test]
    fn it_escapes_closing_square_brackets() {
        assert_eq!(
            fmts(r#"[\[1\]](http://example.com)"#).0,
            r#"[\[1\]](http://example.com)"#
        );
    }

    #[test]
    fn link_titles() {
        // See https://spec.commonmark.org/0.30/#link-title for the rules around
        // link titles and the characters they may contain
        assert_eq!(
            fmts(r#"[link](http://example.com "'link title'")"#).0,
            r#"[link](http://example.com "'link title'")"#
        );
        assert_eq!(
            fmts(r#"[link](http://example.com "\\\"link \\ title\"")"#).0,
            r#"[link](http://example.com "\\\"link \\ title\"")"#
        );
        assert_eq!(
            fmts(r#"[link](http://example.com "\"link title\"")"#).0,
            r#"[link](http://example.com "\"link title\"")"#
        );
        assert_eq!(
            fmts(r#"[link](http://example.com '"link title"')"#).0,
            r#"[link](http://example.com "\"link title\"")"#
        );
        assert_eq!(
            fmts(r#"[link](http://example.com '\'link title\'')"#).0,
            r#"[link](http://example.com "'link title'")"#
        );
        assert_eq!(
            fmts(r#"[link](http://example.com (\(link title\)))"#).0,
            r#"[link](http://example.com "(link title)")"#
        );
        assert_eq!(
            fmts(r#"[link](http://example.com (你好👋))"#).0,
            r#"[link](http://example.com "你好👋")"#
        );
    }

    #[test]
    fn it_does_not_escape_lone_square_brackets_in_text_if_the_source_does_not() {
        assert_eq!(
            fmts("] a closing bracket does nothing").0,
            "] a closing bracket does nothing"
        )
    }

    #[test]
    fn make_special_characters_into_text_blocks() {
        let e: Vec<_> = Parser::new(r#"hello\*there*and\*\*hello again\*\*"#).collect();
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
        )
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
        )
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
            )
        })
    }
}

mod list {
    use super::{fmts, fmts_with_options, CmarkToCmarkOptions, State};

    #[test]
    fn ordered_and_unordered_nested_and_ordered() {
        assert_eq!(
            fmts("1. *b*\n   * *b*\n1. c"),
            (
                "1. *b*\n   * *b*\n1. c".into(),
                State {
                    newlines_before_start: 2,
                    ..Default::default()
                }
            )
        )
    }

    #[test]
    fn ordered_and_multiple_unordered() {
        assert_eq!(
            fmts("11. *b*\n    * *b*\n    * c"),
            (
                "11. *b*\n    * *b*\n    * c".into(),
                State {
                    newlines_before_start: 2,
                    ..Default::default()
                }
            )
        )
    }

    #[test]
    fn unordered_ordered_unordered() {
        assert_eq!(fmts("* a\n  1. b\n* c").0, "* a\n  1. b\n* c",)
    }

    #[test]
    fn ordered_and_unordered_nested() {
        assert_eq!(
            fmts("1. *b*\n   * *b*"),
            (
                "1. *b*\n   * *b*".into(),
                State {
                    newlines_before_start: 2,
                    ..Default::default()
                }
            )
        )
    }

    #[test]
    fn unordered() {
        assert_eq!(
            fmts("* a\n* b"),
            (
                "* a\n* b".into(),
                State {
                    newlines_before_start: 2,
                    ..Default::default()
                }
            )
        )
    }

    #[test]
    fn unordered_with_custom() {
        let custom_options = CmarkToCmarkOptions {
            list_token: '-',
            ..Default::default()
        };

        let original = "* a\n* b";
        let (s, _) = fmts_with_options(original, custom_options);

        assert_eq!(s, "- a\n- b".to_string())
    }

    #[test]
    fn ordered() {
        assert_eq!(
            fmts("2. a\n2. b"),
            (
                "2. a\n2. b".into(),
                State {
                    newlines_before_start: 2,
                    ..Default::default()
                }
            )
        )
    }

    #[test]
    fn change_ordered_list_token() {
        let custom_options = CmarkToCmarkOptions {
            ordered_list_token: ')',
            ..Default::default()
        };
        assert_eq!(
            fmts_with_options("2. a\n2. b", custom_options),
            (
                "2) a\n2) b".into(),
                State {
                    newlines_before_start: 2,
                    ..Default::default()
                }
            )
        )
    }

    #[test]
    fn increment_ordered_list_bullets() {
        let custom_options = CmarkToCmarkOptions {
            increment_ordered_list_bullets: true,
            ..Default::default()
        };
        assert_eq!(
            fmts_with_options("2. a\n2. b\n2. c", custom_options),
            (
                "2. a\n3. b\n4. c".into(),
                State {
                    newlines_before_start: 2,
                    ..Default::default()
                }
            )
        )
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
        assert_eq!(
            fmts_with_options(input, custom_options),
            (
                expected.into(),
                State {
                    newlines_before_start: 2,
                    ..Default::default()
                }
            )
        )
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
        assert_eq!(
            fmts_with_options(input, custom_options),
            (
                expected.into(),
                State {
                    newlines_before_start: 2,
                    ..Default::default()
                }
            )
        )
    }

    #[test]
    fn checkboxes() {
        assert_eq!(
            fmts(indoc!(
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
    use super::assert_events_eq;

    #[test]
    fn heading_with_classes_and_attrs() {
        assert_events_eq("# Heading { #id .class1 key1=val1 .class2 }");
        assert_events_eq("# Heading { #id .class1 .class2 key1=val1 key2 }");
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