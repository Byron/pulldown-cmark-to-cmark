use pulldown_cmark::Event;
use pulldown_cmark_to_cmark::*;

fn s(e: Event) -> String {
    es([e])
}
fn es<'a>(es: impl IntoIterator<Item = Event<'a>>) -> String {
    let mut buf = String::new();
    cmark(es.into_iter(), &mut buf).unwrap();
    buf
}
mod code {
    use pulldown_cmark::Event::*;

    use super::s;

    #[test]
    fn code() {
        assert_eq!(s(Code("foo\nbar".into())), "`foo\nbar`");
    }
}

mod rule {
    use pulldown_cmark::Event::*;

    use super::s;

    #[test]
    fn rule() {
        assert_eq!(s(Rule), "---");
    }
}

mod start {
    use pulldown_cmark::{
        Alignment::{self, Center, Left, Right},
        BlockQuoteKind, CodeBlockKind,
        Event::*,
        HeadingLevel,
        LinkType::*,
        Tag::*,
    };

    use super::s;

    #[test]
    fn paragraph() {
        assert_eq!(s(Start(Paragraph)), "");
    }
    #[test]
    fn header1() {
        assert_eq!(
            s(Start(Heading {
                level: HeadingLevel::H1,
                id: None,
                classes: vec![],
                attrs: vec![]
            })),
            "# "
        );
    }
    #[test]
    fn header2() {
        assert_eq!(
            s(Start(Heading {
                level: HeadingLevel::H2,
                id: None,
                classes: vec![],
                attrs: vec![]
            })),
            "## "
        );
    }
    #[test]
    fn blockquote() {
        assert_eq!(s(Start(BlockQuote(None))), "\n > ");
        assert_eq!(s(Start(BlockQuote(Some(BlockQuoteKind::Note)))), "\n > [!NOTE]");
        assert_eq!(s(Start(BlockQuote(Some(BlockQuoteKind::Tip)))), "\n > [!TIP]");
        assert_eq!(
            s(Start(BlockQuote(Some(BlockQuoteKind::Important)))),
            "\n > [!IMPORTANT]"
        );
        assert_eq!(s(Start(BlockQuote(Some(BlockQuoteKind::Warning)))), "\n > [!WARNING]");
        assert_eq!(s(Start(BlockQuote(Some(BlockQuoteKind::Caution)))), "\n > [!CAUTION]");
    }
    #[test]
    fn codeblock() {
        assert_eq!(
            s(Start(CodeBlock(CodeBlockKind::Fenced("asdf".into())))),
            "\n````asdf\n"
        );
    }
    #[test]
    fn list_unordered() {
        assert_eq!(s(Start(List(None))), "");
    }
    #[test]
    fn list_ordered() {
        assert_eq!(s(Start(List(Some(1)))), "");
    }
    #[test]
    fn item() {
        assert_eq!(s(Start(Item)), "");
    }
    #[test]
    fn footnote_definition() {
        assert_eq!(s(Start(FootnoteDefinition("asdf".into()))), "[^asdf]: ");
    }
    #[test]
    fn emphasis() {
        assert_eq!(s(Start(Emphasis)), "*");
    }
    #[test]
    fn strong() {
        assert_eq!(s(Start(Strong)), "**");
    }
    #[test]
    fn link() {
        assert_eq!(
            s(Start(Link {
                link_type: Inline,
                dest_url: "uri".into(),
                title: "title".into(),
                id: "".into(),
            })),
            "["
        );
    }
    #[test]
    fn link_without_title() {
        assert_eq!(
            s(Start(Link {
                link_type: Inline,
                dest_url: "uri".into(),
                title: "".into(),
                id: "".into()
            })),
            "["
        );
    }
    #[test]
    fn image() {
        assert_eq!(
            s(Start(Image {
                link_type: Inline,
                dest_url: "uri".into(),
                title: "title".into(),
                id: "".into()
            })),
            "!["
        );
    }
    #[test]
    fn image_without_title() {
        assert_eq!(
            s(Start(Image {
                link_type: Inline,
                dest_url: "uri".into(),
                title: "".into(),
                id: "".into()
            })),
            "!["
        );
    }
    #[test]
    fn table() {
        assert_eq!(s(Start(Table(vec![Left, Center, Right, Alignment::None]))), "");
    }
    #[test]
    fn table_head() {
        assert_eq!(s(Start(TableHead)), "");
    }
    #[test]
    fn table_row() {
        assert_eq!(s(Start(TableRow)), "");
    }
    #[test]
    fn table_cell() {
        assert_eq!(s(Start(TableCell)), "|");
    }

    #[test]
    fn definition_list_definition() {
        assert_eq!(s(Start(DefinitionListDefinition)), ": ");
    }
}

mod end {
    use pulldown_cmark::{BlockQuoteKind, CodeBlockKind, CowStr, Event::*, HeadingLevel, LinkType::*, Tag, TagEnd};

    use super::{es, s};

    #[test]
    fn header() {
        let tag = Tag::Heading {
            level: HeadingLevel::H2,
            id: None,
            classes: Default::default(),
            attrs: Default::default(),
        };
        assert_eq!(es([Start(tag.clone()), End(tag.to_end())]), "## ");
    }
    #[test]
    fn paragraph() {
        assert_eq!(s(End(TagEnd::Paragraph)), "");
    }
    #[test]
    fn blockquote() {
        assert_eq!(s(End(TagEnd::BlockQuote(None))), "");
        assert_eq!(
            es([
                Start(Tag::BlockQuote(Some(BlockQuoteKind::Note))),
                Text(CowStr::Borrowed("This is a note")),
                End(TagEnd::BlockQuote(Some(BlockQuoteKind::Note)))
            ]),
            "\n > [!NOTE]\n > This is a note"
        );
    }
    #[test]
    fn codeblock() {
        assert_eq!(
            es([
                Start(Tag::CodeBlock(CodeBlockKind::Fenced("".into()))),
                End(TagEnd::CodeBlock)
            ]),
            "\n````\n````"
        );
    }
    #[test]
    fn codeblock_in_list_item() {
        assert_eq!(
            es([
                Start(Tag::List(None)),
                Start(Tag::Item),
                Start(Tag::CodeBlock(CodeBlockKind::Fenced("".into()))),
                Text("foo".into()),
                End(TagEnd::CodeBlock),
                End(TagEnd::Item),
                End(TagEnd::List(false)),
                Start(Tag::Paragraph),
                Text("bar".into()),
                End(TagEnd::Paragraph),
            ]),
            "* \n  ````\n  foo\n  ````\n\nbar"
        );
    }
    #[test]
    fn codeblock_indented_in_list_item() {
        assert_eq!(
            es([
                Start(Tag::List(None)),
                Start(Tag::Item),
                Start(Tag::CodeBlock(CodeBlockKind::Indented)),
                Text("foo".into()),
                End(TagEnd::CodeBlock),
                End(TagEnd::Item),
                End(TagEnd::List(false)),
                Start(Tag::Paragraph),
                Text("bar".into()),
                End(TagEnd::Paragraph),
            ]),
            "* \n      foo\n      \n\nbar"
        );
    }
    #[test]
    fn footnote_definition() {
        assert_eq!(s(End(TagEnd::FootnoteDefinition)), "");
    }
    #[test]
    fn emphasis() {
        assert_eq!(s(End(TagEnd::Emphasis)), "*");
    }
    #[test]
    fn strong() {
        assert_eq!(s(End(TagEnd::Strong)), "**");
    }
    #[test]
    fn list_unordered() {
        assert_eq!(s(End(TagEnd::List(false))), "");
    }
    #[test]
    fn list_ordered() {
        assert_eq!(s(End(TagEnd::List(true))), "");
    }
    #[test]
    fn item() {
        assert_eq!(s(End(TagEnd::Item)), "");
    }
    #[test]
    fn link() {
        let tag = Tag::Link {
            link_type: Inline,
            dest_url: "/uri".into(),
            title: "title".into(),
            id: "".into(),
        };
        assert_eq!(es([Start(tag.clone()), End(tag.to_end())]), "[](/uri \"title\")");
    }
    #[test]
    fn link_without_title() {
        let tag = Tag::Link {
            link_type: Inline,
            dest_url: "/uri".into(),
            title: "".into(),
            id: "".into(),
        };
        assert_eq!(es([Start(tag.clone()), End(tag.to_end())]), "[](/uri)");
    }
    #[test]
    fn image() {
        let tag = Tag::Image {
            link_type: Inline,
            dest_url: "/uri".into(),
            title: "title".into(),
            id: "".into(),
        };
        assert_eq!(es([Start(tag.clone()), End(tag.to_end())]), "![](/uri \"title\")");
    }
    #[test]
    fn image_without_title() {
        let tag = Tag::Image {
            link_type: Inline,
            dest_url: "/uri".into(),
            title: "".into(),
            id: "".into(),
        };
        assert_eq!(es([Start(tag.clone()), End(tag.to_end())]), "![](/uri)");
    }
    #[test]
    fn table() {
        assert_eq!(s(End(TagEnd::Table)), "");
    }
    #[test]
    fn table_row() {
        assert_eq!(s(End(TagEnd::TableRow)), "|");
    }
    #[test]
    fn table_cell() {
        assert_eq!(s(End(TagEnd::TableCell)), "");
    }
}

#[test]
fn hardbreak() {
    assert_eq!(s(Event::HardBreak), "  \n");
}
#[test]
fn softbreak() {
    assert_eq!(s(Event::SoftBreak), "\n");
}
#[test]
fn html() {
    assert_eq!(s(Event::Html("<table>hi</table>".into())), "<table>hi</table>");
}
#[test]
fn text() {
    assert_eq!(s(Event::Text("asdf".into())), "asdf");
}
#[test]
fn footnote_reference() {
    assert_eq!(s(Event::FootnoteReference("asdf".into())), "[^asdf]");
}
#[test]
fn math() {
    assert_eq!(
        s(Event::InlineMath(r"\sqrt{3x-1}+(1+x)^2".into())),
        r"$\sqrt{3x-1}+(1+x)^2$"
    );
    assert_eq!(s(Event::InlineMath(r"\sqrt{\$4}".into())), r"$\sqrt{\$4}$");
    assert_eq!(s(
      Event::DisplayMath(
        r"\left( \sum_{k=1}^n a_k b_k \right)^2 \leq \left( \sum_{k=1}^n a_k^2 \right) \left( \sum_{k=1}^n b_k^2 \right)".into()
      )),
        r"$$\left( \sum_{k=1}^n a_k b_k \right)^2 \leq \left( \sum_{k=1}^n a_k^2 \right) \left( \sum_{k=1}^n b_k^2 \right)$$"
      );
}
