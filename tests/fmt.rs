#[macro_use]
extern crate indoc;
extern crate pulldown_cmark;
extern crate pulldown_cmark_to_cmark;

use pulldown_cmark_to_cmark::fmt::{cmark, State};
use pulldown_cmark::{Alignment, Event, Options, Parser, Tag};

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

mod lazy_newlines {
    use super::{fmte, fmts};
    use super::{Event, State, Tag};

    #[test]
    fn after_emphasis_there_is_no_newline() {
        for t in &[
            Tag::Emphasis,
            Tag::Strong,
            Tag::Code,
            Tag::BlockQuote,
            Tag::Link("".into(), "".into()),
            Tag::Image("".into(), "".into()),
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
        for md in &["paragraph", "## headline", "```\n```", "---"] {
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
            &[Event::Html("<e>".into())],
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
}

mod blockquote {
    use super::{fmte, fmtes, fmts, Event, State, Tag};

    #[test]
    fn it_pops_padding_on_quote_end() {
        assert_eq!(
            fmtes(
                &[Event::End(Tag::BlockQuote),],
                State {
                    padding: vec![" > ".into()],
                    ..Default::default()
                }
            ).1,
            State {
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
                padding: vec![" > ".into()],
                ..Default::default()
            }
        )
    }

    #[test]
    fn with_html() {
        assert_eq!(
            fmts(indoc!(
                "
         > <table>
         > </table>"
            )).0,
            " > <table>\n > </table>",
        )
    }
    #[test]
    fn with_inlinehtml() {
        assert_eq!(fmts(" > <br>").0, " > <br>",)
    }
    #[test]
    fn with_codeblock() {
        assert_eq!(
            fmts(indoc!(
                "
             > ```a
             > t1
             > t2
             > ```
            "
            )).0,
            " > ```a\n > t1\n > t2\n > ```",
        )
    }
    #[test]
    fn nested() {
        assert_eq!(
            fmts(indoc!(
                "
             > a
             > > b
             >
             > c
            "
            )).0,
            " > a\n >  > \n >  > b\n > \n > c",
        )
    }
    #[test]
    fn simple() {
        assert_eq!(
            fmts(indoc!(
                "
             > a
             > b  
             > c"
            )),
            (
                " > a\n > b  \n > c".into(),
                State {
                    newlines_before_start: 2,
                    ..Default::default()
                }
            )
        )
    }
}

mod codeblock {
    use super::{fmts, State};

    #[test]
    fn simple_and_paragraph() {
        assert_eq!(
            fmts("```hi\nsome\ntext\n```\na"),
            (
                "```hi\nsome\ntext\n```\n\na".into(),
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
                "```\n```".into(),
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
                "```hi\nsome\ntext\n```".into(),
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
    use pulldown_cmark_to_cmark::fmt::Alignment;

    #[test]
    fn it_forget_alignments_and_headers_at_the_end_of_tables() {
        assert_eq!(
            fmtes(
                &[Event::End(Tag::Table(vec![])),],
                State {
                    table_alignments: vec![Alignment::None, Alignment::Center],
                    table_headers: vec!["a".into(), "b".into()],
                    ..Default::default()
                }
            ).1,
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
                Event::Start(Tag::TableCell),
                Event::Text("b".into()),
            ]).1,
            State {
                table_alignments: vec![Alignment::None, Alignment::Center],
                table_headers: vec!["a".into(), "b".into()],
                ..Default::default()
            }
        )
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
            ).1,
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
}
