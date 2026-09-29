use pulldown_cmark::Event;
use pulldown_cmark_to_cmark::*;

fn s(e: Event) -> String {
    es([e])
}
fn es<'a>(es: impl IntoIterator<Item = Event<'a>>) -> String {
    let mut buf = String::new();
    cmark(es, &mut buf, SUPPORTED_PARSER_OPTIONS).unwrap();
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

mod containers {
    use super::es;
    use pulldown_cmark::{BlockQuoteKind, CodeBlockKind, Event::*, HeadingLevel, LinkType, Tag};

    fn wrap(tag: Tag<'_>, text: &str) -> String {
        let end = tag.to_end();
        es([Start(tag), Text(text.to_owned().into()), End(end)])
    }

    #[test]
    fn inline_and_block_delimiters() {
        for (tag, expected) in [
            (Tag::Paragraph, "x"),
            (Tag::Emphasis, "*x*"),
            (Tag::Strong, "**x**"),
            (Tag::Strikethrough, "~~x~~"),
            (Tag::Superscript, "^x^"),
            (Tag::Subscript, "~x~"),
            (Tag::BlockQuote(None), "\n > \n > x"),
            (Tag::CodeBlock(CodeBlockKind::Fenced("asdf".into())), "\n````asdf\nx"),
            (Tag::FootnoteDefinition("asdf".into()), "[^asdf]: x"),
        ] {
            assert_eq!(wrap(tag, "x"), expected);
        }
    }

    #[test]
    fn headings() {
        for (level, expected) in [(HeadingLevel::H1, "# x"), (HeadingLevel::H2, "## x")] {
            assert_eq!(
                wrap(
                    Tag::Heading {
                        level,
                        id: None,
                        classes: vec![],
                        attrs: vec![]
                    },
                    "x"
                ),
                expected
            );
        }
    }

    #[test]
    fn alerts() {
        for (kind, name) in [
            (BlockQuoteKind::Note, "NOTE"),
            (BlockQuoteKind::Tip, "TIP"),
            (BlockQuoteKind::Important, "IMPORTANT"),
            (BlockQuoteKind::Warning, "WARNING"),
            (BlockQuoteKind::Caution, "CAUTION"),
        ] {
            assert_eq!(wrap(Tag::BlockQuote(Some(kind)), "x"), format!("\n > [!{name}]\n > x"));
        }
    }

    #[test]
    fn links_and_images() {
        for title in ["", "title"] {
            let suffix = if title.is_empty() { "" } else { " \"title\"" };
            assert_eq!(
                wrap(
                    Tag::Link {
                        link_type: LinkType::Inline,
                        dest_url: "uri".into(),
                        title: title.into(),
                        id: "".into()
                    },
                    "x"
                ),
                format!("[x](uri{suffix})")
            );
            assert_eq!(
                wrap(
                    Tag::Image {
                        link_type: LinkType::Inline,
                        dest_url: "uri".into(),
                        title: title.into(),
                        id: "".into()
                    },
                    "x"
                ),
                format!("![x](uri{suffix})")
            );
        }
    }
}

mod end {
    use pulldown_cmark::{BlockQuoteKind, CodeBlockKind, CowStr, Event::*, HeadingLevel, LinkType::*, Tag, TagEnd};

    use super::es;

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
    fn blockquote() {
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
                Text("foo\n".into()),
                End(TagEnd::CodeBlock),
                End(TagEnd::Item),
                End(TagEnd::List(false)),
                Start(Tag::Paragraph),
                Text("bar".into()),
                End(TagEnd::Paragraph),
            ]),
            "* ````\n  foo\n  ````\n\nbar"
        );
    }
    #[test]
    fn codeblock_indented_in_list_item() {
        assert_eq!(
            es([
                Start(Tag::List(None)),
                Start(Tag::Item),
                Start(Tag::CodeBlock(CodeBlockKind::Indented)),
                Text("foo\n".into()),
                End(TagEnd::CodeBlock),
                End(TagEnd::Item),
                End(TagEnd::List(false)),
                Start(Tag::Paragraph),
                Text("bar".into()),
                End(TagEnd::Paragraph),
            ]),
            "*     foo\n\nbar"
        );
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
