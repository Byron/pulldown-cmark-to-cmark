#[macro_use]
extern crate indoc;

use pulldown_cmark::{Alignment, CodeBlockKind, Event, LinkType, Options, Parser, Tag};
use pulldown_cmark_to_cmark::{cmark, cmark_resume, cmark_resume_with_options, Options as CmarkToCmarkOptions, State};

fn fmts(s: &str) -> (String, State<'static>) {
    let mut buf = String::new();
    let s = cmark(Parser::new_ext(s, Options::all()), &mut buf).unwrap();
    (buf, s)
}

fn fmts_with_options(s: &str, options: CmarkToCmarkOptions) -> (String, State<'static>) {
    let mut buf = String::new();
    let s = cmark_resume_with_options(Parser::new_ext(s, Options::all()), &mut buf, None, options).unwrap();
    (buf, s)
}

fn fmtes(e: &[Event], s: State<'static>) -> (String, State<'static>) {
    let mut buf = String::new();
    let s = cmark_resume(e.iter(), &mut buf, Some(s)).unwrap();
    (buf, s)
}

fn fmte<'a>(e: impl AsRef<[Event<'a>]>) -> (String, State<'static>) {
    let mut buf = String::new();
    let s = cmark(e.as_ref().iter(), &mut buf).unwrap();
    (buf, s)
}

/// Asserts that if we parse our `str` s into a series of events, then serialize them with `cmark`
/// that we'll get the same series of events when we parse them again.
fn assert_events_eq(s: &str) {
    let before_events = Parser::new_ext(s, Options::all());

    let mut buf = String::new();
    cmark(before_events, &mut buf).unwrap();

    let before_events = Parser::new_ext(s, Options::all());
    let after_events = Parser::new_ext(&buf, Options::all());
    println!("{}", buf);
    assert_eq!(before_events.collect::<Vec<_>>(), after_events.collect::<Vec<_>>());
}

mod lazy_newlines {
    use super::{fmte, fmts, Event, LinkType, State, Tag};

    #[test]
    fn after_emphasis_there_is_no_newline() {
        for t in &[
            Tag::Emphasis,
            Tag::Strong,
            Tag::Link(LinkType::Inline, "".into(), "".into()),
            Tag::Image(LinkType::Inline, "".into(), "".into()),
            Tag::FootnoteDefinition("".into()),
        ] {
            assert_eq!(
                fmte(&[Event::End(t.clone())]).1,
                State {
                    newlines_before_start: 0,
                    ..Default::default()
                }
            )
        }
    }

    #[test]
    fn after_anything_else_it_has_one_newline() {
        for e in &[
            Event::End(Tag::Item),
            Event::End(Tag::TableRow),
            Event::End(Tag::TableHead),
        ] {
            assert_eq!(
                fmte(&[e.clone()]).1,
                State {
                    newlines_before_start: 1,
                    ..Default::default()
                }
            )
        }
    }

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

#[test]
fn it_applies_newlines_before_start_before_text() {
    assert_eq!(
        fmtes(
            &[Event::Text("t".into())],
            State {
                newlines_before_start: 2,
                ..Default::default()
            }
        ),
        (
            "\n\nt".into(),
            State {
                newlines_before_start: 0,
                ..Default::default()
            }
        )
    )
}

#[test]
fn it_applies_newlines_before_start_before_any_start_tag() {
    assert_eq!(
        fmtes(
            &[Event::Start(Tag::Paragraph), Event::Text("h".into())],
            State {
                newlines_before_start: 2,
                ..Default::default()
            }
        ),
        (
            "\n\nh".into(),
            State {
                newlines_before_start: 0,
                ..Default::default()
            }
        )
    )
}

mod padding {
    use super::{fmtes, Event, State, Tag};

    #[test]
    fn is_used_before_newlines() {
        assert_eq!(
            fmtes(
                &[Event::Start(Tag::Paragraph), Event::Text("h".into())],
                State {
                    newlines_before_start: 2,
                    padding: vec!["  ".into()],
                    ..Default::default()
                }
            ),
            (
                "\n  \n  h".into(),
                State {
                    newlines_before_start: 0,
                    padding: vec!["  ".into()],
                    ..Default::default()
                }
            )
        )
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
            fmts("a [^b]\n[^b]: c"),
            (
                "a [^b]\n[^b]: c".into(),
                State {
                    newlines_before_start: 2,
                    ..Default::default()
                }
            )
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
        let mut custom_options = CmarkToCmarkOptions::default();
        custom_options.emphasis_token = '_';
        custom_options.code_block_token = '~';

        let (s, state) = fmts_with_options("_a_ b **c**\n<br>\nd\n\ne `c`", custom_options);

        assert_eq!(s, "_a_ b **c**\n<br>\nd\n\ne ~c~".to_string());
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
}

mod blockquote {
    use super::{assert_events_eq, fmte, fmtes, fmts, Event, State, Tag};

    #[test]
    fn it_pops_padding_on_quote_end() {
        assert_eq!(
            fmtes(
                &[Event::End(Tag::BlockQuote),],
                State {
                    padding: vec![" > ".into()],
                    ..Default::default()
                }
            )
            .1,
            State {
                newlines_before_start: 2,
                padding: vec![],
                ..Default::default()
            }
        )
    }

    #[test]
    fn it_pushes_padding_on_quote_start() {
        assert_eq!(
            fmte(&[Event::Start(Tag::BlockQuote),]).1,
            State {
                newlines_before_start: 1,
                padding: vec![" > ".into()],
                ..Default::default()
            }
        )
    }

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
    use super::{fmte, fmts, fmts_with_options, CmarkToCmarkOptions, CodeBlockKind, Event, State, Tag};

    #[test]
    fn it_keeps_track_of_the_presence_of_a_code_block() {
        assert_eq!(
            fmte(&[Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced("s".into()))),]).1,
            State {
                is_in_code_block: true,
                ..Default::default()
            }
        )
    }

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
        let mut custom_options = CmarkToCmarkOptions::default();
        custom_options.code_block_token = '~';
        let original = "~~~hi\nsome\ntext\n~~~";
        let (s, _) = fmts_with_options(original, custom_options);

        assert_eq!(s, "\n~~~~hi\nsome\ntext\n~~~~".to_string());
    }
}

mod table {
    use pretty_assertions::assert_eq;
    use pulldown_cmark_to_cmark::Alignment;

    use super::{fmte, fmtes, Alignment as TableAlignment, Event, State, Tag};

    #[test]
    fn it_forgets_alignments_and_headers_at_the_end_of_tables() {
        assert_eq!(
            fmtes(
                &[Event::End(Tag::Table(vec![])),],
                State {
                    table_alignments: vec![Alignment::None, Alignment::Center],
                    table_headers: vec!["a".into(), "b".into()],
                    ..Default::default()
                }
            )
            .1,
            State {
                newlines_before_start: 2,
                ..Default::default()
            }
        )
    }
    #[test]
    fn it_keeps_track_of_alignments_and_headers() {
        assert_eq!(
            fmte(&[
                Event::Start(Tag::Table(vec![TableAlignment::None, TableAlignment::Center,])),
                Event::Start(Tag::TableHead),
                Event::Start(Tag::TableCell),
                Event::Text("a".into()),
                Event::End(Tag::TableCell),
                Event::Start(Tag::TableCell),
                Event::Text("b".into()),
                Event::End(Tag::TableCell),
            ])
            .1,
            State {
                table_alignments: vec![Alignment::None, Alignment::Center],
                table_headers: vec!["a".into(), "b".into()],
                ..Default::default()
            }
        )
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
            |------|:-:|---:|:-|--|
            |col 3 is|right-aligned|$1600|x|01|
            |col 2 is|centered|$12|y|02|
            |zebra stripes|are neat|$1|z|03|"
            )
        );

        let p = Parser::new_ext(&generated_markdown, Options::all());
        let generated_events: Vec<_> = p.into_iter().collect();

        assert_eq!(original_events, generated_events);
    }
}

mod escapes {
    use pulldown_cmark::CowStr;

    use crate::{fmts, CmarkToCmarkOptions, Event, Parser, Tag};

    fn run_test_on_each_special_char(f: impl Fn(String, CowStr)) {
        for c in CmarkToCmarkOptions::default().special_characters().chars() {
            let s = format!(r#"\{special}"#, special = c);
            f(s, c.to_string().into())
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
                Event::End(Tag::Paragraph),
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
                Event::End(Tag::Paragraph),
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
    fn it_does_esscape_lone_square_brackets_in_text() {
        assert_eq!(
            fmts("] a closing bracket does nothing").0,
            "\\] a closing bracket does nothing"
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
                Event::End(Tag::Paragraph),
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
                Event::End(Tag::Emphasis),
                Event::Text("and".into()),
                Event::Start(Tag::Strong),
                Event::Text("hello again".into()),
                Event::End(Tag::Strong),
                Event::End(Tag::Paragraph),
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
                    Event::End(Tag::Paragraph),
                ]
            )
        })
    }
}

mod list {
    use super::{fmtes, fmts, fmts_with_options, CmarkToCmarkOptions, Event, State, Tag};

    #[test]
    fn it_pops_one_item_from_the_lists_stack_for_each_end_list() {
        assert_eq!(
            fmtes(
                &[Event::End(Tag::List(None))],
                State {
                    list_stack: vec![None, None],
                    ..Default::default()
                }
            )
            .1,
            State {
                list_stack: vec![None],
                ..Default::default()
            }
        )
    }

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
        let mut custom_options = CmarkToCmarkOptions::default();
        custom_options.list_token = '-';

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
