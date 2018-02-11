extern crate pulldown_cmark;
extern crate pulldown_cmark_to_cmark;

use pulldown_cmark::Event;
use pulldown_cmark_to_cmark::fmt;

fn s(e: Event) -> String {
    let mut buf = String::new();
    fmt::cmark([e].iter(), &mut buf, None).unwrap();
    buf
}

mod start {
    use pulldown_cmark::Event::*;
    use pulldown_cmark::Tag::*;
    use pulldown_cmark::Alignment::{self, Center, Left, Right};
    use super::s;

    #[test]
    fn paragraph() {
        assert_eq!(s(Start(Paragraph)), "")
    }
    #[test]
    fn rule() {
        assert_eq!(s(Start(Rule)), "---")
    }
    #[test]
    fn header1() {
        assert_eq!(s(Start(Header(1))), "# ")
    }
    #[test]
    fn header2() {
        assert_eq!(s(Start(Header(2))), "## ")
    }
    #[test]
    fn blockquote() {
        assert_eq!(s(Start(BlockQuote)), " > ")
    }
    #[test]
    fn codeblock() {
        assert_eq!(s(Start(CodeBlock("asdf".into()))), "```asdf\n")
    }
    #[test]
    fn list_unordered() {
        assert_eq!(s(Start(List(None))), "")
    }
    #[test]
    fn list_ordered() {
        assert_eq!(s(Start(List(Some(1)))), "")
    }
    #[test]
    fn item() {
        assert_eq!(s(Start(Item)), "")
    }
    #[test]
    fn footnote_definition() {
        assert_eq!(s(Start(FootnoteDefinition("asdf".into()))), "[^asdf]: ")
    }
    #[test]
    fn emphasis() {
        assert_eq!(s(Start(Emphasis)), "*")
    }
    #[test]
    fn strong() {
        assert_eq!(s(Start(Strong)), "**")
    }
    #[test]
    fn code() {
        assert_eq!(s(Start(Code)), "`")
    }
    #[test]
    fn link() {
        assert_eq!(s(Start(Link("uri".into(), "title".into()))), "[")
    }
    #[test]
    fn link_without_title() {
        assert_eq!(s(Start(Link("uri".into(), "".into()))), "[")
    }
    #[test]
    fn image() {
        assert_eq!(s(Start(Image("uri".into(), "title".into()))), "![")
    }
    #[test]
    fn image_without_title() {
        assert_eq!(s(Start(Image("uri".into(), "".into()))), "![")
    }
    #[test]
    fn table() {
        assert_eq!(
            s(Start(Table(vec![Left, Center, Right, Alignment::None]))),
            ""
        )
    }
    #[test]
    fn table_head() {
        assert_eq!(s(Start(TableHead)), "")
    }
    #[test]
    fn table_row() {
        assert_eq!(s(Start(TableRow)), "")
    }
    #[test]
    fn table_cell() {
        assert_eq!(s(Start(TableCell)), "|")
    }
}

mod end {
    use pulldown_cmark::Event::*;
    use pulldown_cmark::Tag::*;
    use pulldown_cmark::Alignment::{self, Center, Left, Right};
    use super::s;

    #[test]
    fn header() {
        assert_eq!(s(End(Header(2))), "")
    }
    #[test]
    fn paragraph() {
        assert_eq!(s(End(Paragraph)), "")
    }
    #[test]
    fn rule() {
        assert_eq!(s(End(Rule)), "")
    }
    #[test]
    fn blockquote() {
        assert_eq!(s(End(BlockQuote)), "")
    }
    #[test]
    fn codeblock() {
        assert_eq!(s(End(CodeBlock("asdf".into()))), "```")
    }
    #[test]
    fn footnote_definition() {
        assert_eq!(s(End(FootnoteDefinition("asdf".into()))), "")
    }
    #[test]
    fn emphasis() {
        assert_eq!(s(End(Emphasis)), "*")
    }
    #[test]
    fn strong() {
        assert_eq!(s(End(Strong)), "**")
    }
    #[test]
    fn list_unordered() {
        assert_eq!(s(End(List(None))), "")
    }
    #[test]
    fn list_ordered() {
        assert_eq!(s(End(List(Some(1)))), "")
    }
    #[test]
    fn item() {
        assert_eq!(s(End(Item)), "")
    }
    #[test]
    fn code() {
        assert_eq!(s(End(Code)), "`")
    }
    #[test]
    fn link() {
        assert_eq!(
            s(End(Link("/uri".into(), "title".into()))),
            "](/uri \"title\")"
        )
    }
    #[test]
    fn link_without_title() {
        assert_eq!(s(End(Link("/uri".into(), "".into()))), "](/uri)")
    }
    #[test]
    fn image() {
        assert_eq!(
            s(End(Image("/uri".into(), "title".into()))),
            "](/uri \"title\")"
        )
    }
    #[test]
    fn image_without_title() {
        assert_eq!(s(End(Image("/uri".into(), "".into()))), "](/uri)")
    }
    #[test]
    fn table() {
        assert_eq!(
            s(End(Table(vec![Left, Center, Right, Alignment::None]))),
            ""
        )
    }
    #[test]
    fn table_row() {
        assert_eq!(s(End(TableRow)), "|")
    }
    #[test]
    fn table_cell() {
        assert_eq!(s(End(TableCell)), "")
    }
}

#[test]
fn hardbreak() {
    assert_eq!(s(Event::HardBreak), "  \n")
}
#[test]
fn softbreak() {
    assert_eq!(s(Event::SoftBreak), "\n")
}
#[test]
fn html() {
    assert_eq!(
        s(Event::Html("<table>hi</table>".into())),
        "<table>hi</table>"
    )
}
#[test]
fn inlinehtml() {
    assert_eq!(s(Event::InlineHtml("<br>".into())), "<br>")
}
#[test]
fn text() {
    assert_eq!(s(Event::Text("asdf".into())), "asdf")
}
#[test]
fn footnote_reference() {
    assert_eq!(s(Event::FootnoteReference("asdf".into())), "[^asdf]")
}
