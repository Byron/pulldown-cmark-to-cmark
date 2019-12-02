#[macro_use]
extern crate indoc;
extern crate pretty_assertions;
extern crate pulldown_cmark;
extern crate pulldown_cmark_to_cmark;

use pulldown_cmark::{Alignment, Event, LinkType, Options, Parser, Tag};
use pulldown_cmark_to_cmark::fmt::{cmark, State};

fn fmts(s: &str) -> (String, State<'static>) {
    let mut buf = String::new();
    let s = cmark(Parser::new_ext(s, Options::all()), &mut buf, None).unwrap();
    (buf, s)
}

fn fmtes(e: &[Event], s: State<'static>) -> (String, State<'static>) {
    let mut buf = String::new();
    let s = cmark(e.iter(), &mut buf, Some(s)).unwrap();
    (buf, s)
}

fn fmte(e: &[Event]) -> (String, State<'static>) {
    let mut buf = String::new();
    let s = cmark(e.iter(), &mut buf, None).unwrap();
    (buf, s)
}

/// Asserts that if we parse our `str` s into a series of events, then serialize them with `cmark`
/// that we'll get the same series of events when we parse them again.
fn assert_events_eq(s: &str) {
    let before_events = Parser::new_ext(s, Options::all());

    let mut buf = String::new();
    cmark(before_events.clone(), &mut buf, None).unwrap();

    let after_events = Parser::new_ext(&buf, Options::all());
    println!("{}", buf);
    assert_eq!(before_events.collect::<Vec<_>>(), after_events.collect::<Vec<_>>());
}

mod lazy_newlines {
    use super::{fmte, fmts};
    use super::{Event, LinkType, State, Tag};

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
        for md in &["paragraph", "## headline", "````\n````", "---"] {
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
fn it_applies_newlines_before_start_before_html_and_enforces_newline_after() {
    assert_eq!(
        fmtes(
            &[
                Event::Start(Tag::HtmlBlock),
                Event::Html("<e>".into()),
                Event::End(Tag::HtmlBlock),
            ],
            State {
                newlines_before_start: 2,
                ..Default::default()
            }
        ),
        (
            "\n\n<e>".into(),
            State {
                newlines_before_start: 1,
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
    use super::{fmts, State};

    #[test]
    fn image() {
        assert_eq!(
            fmts("![a](b)\n![c][d]\n[d]: e"),
            (
                "![a](b)\n![c][d]\n[d]: e".into(),
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
        assert_eq!(fmts("<http://a/b>").0, "[http://a/b](http://a/b)",)
    }
    #[test]
    fn links() {
        assert_eq!(
            fmts("[a](b)\n[c][d]\n[d]: e"),
            (
                "[a](b)\n[c][d]\n[d]: e".into(),
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
    fn strikethrough() {
        assert_eq!(fmts("~~strikethrough~~").0, "~~strikethrough~~",);
    }
}

mod blockquote {
    use super::{fmte, fmtes, fmts, Event, State, Tag};
    use assert_events_eq;

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
        let s = indoc!("
             > <table>
             > </table>
             ");

        assert_events_eq(s);

        assert_eq!(
            fmts(s)
            .0,
            "\n > \n > <table>\n > </table>\n > ",
        )
    }
    #[test]
    fn with_inlinehtml() {
        assert_eq!(fmts(" > <br>").0, "\n > \n > <br>")
    }
    #[test]
    fn with_codeblock() {
        let s = indoc!("
             > ```a
             > t1
             > t2
             > ```
            ");

        assert_events_eq(s);

        assert_eq!(
            fmts(s)
            .0,
            "\n > \n > ````a\n > t1\n > t2\n > ````",
        )
    }
    #[test]
    fn nested() {
        let s = indoc!("
             > a
             >
             > > b
             >
             > c
            ");

        assert_events_eq(s);

        assert_eq!(
            fmts(s)
            .0,
            "\n > \n > a\n > \n >  > \n >  > b\n > \n > c",
        )
    }

    #[test]
    fn initially_nested() {
        let s = indoc!("
             > > foo
             > bar
             > > baz
            ");

        assert_events_eq(s);

        assert_eq!(
            fmts(s)
                .0,
            "\n > \n >  > \n >  > foo\n >  > bar\n >  > baz",
        )
    }

    #[test]
    fn simple() {
        let s = indoc!("
             > a
             > b  
             > c
             ");

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
        let s = indoc!("
            > foo

            > bar
            ");

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
        let s = indoc!("
            > foo
            baz

            > bar
            ");

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
        let s = indoc!("
            - > * foo
              >     * baz
                - > bar
            ");

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
    use super::{fmts, State};

    #[test]
    fn simple_and_paragraph() {
        assert_eq!(
            fmts("````hi\nsome\ntext\n````\na"),
            (
                "````hi\nsome\ntext\n````\n\na".into(),
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
                "````\n````".into(),
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
                "````hi\nsome\ntext\n````".into(),
                State {
                    newlines_before_start: 2,
                    ..Default::default()
                }
            )
        )
    }
}

mod table {
    use super::{fmte, fmtes, Alignment as TableAlignment, Event, State, Tag};
    use pretty_assertions::assert_eq;
    use pulldown_cmark_to_cmark::fmt::Alignment;

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
                Event::Start(Tag::Table(vec![
                    TableAlignment::None,
                    TableAlignment::Center,
                ])),
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

mod list {
    use super::{fmtes, fmts, Event, State, Tag};

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
