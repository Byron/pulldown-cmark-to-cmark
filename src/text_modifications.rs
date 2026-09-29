use pulldown_cmark::LinkType;
use std::fmt::{self, Write};

pub(crate) fn close_link<F>(uri: &str, title: &str, f: &mut F, link_type: LinkType) -> fmt::Result
where
    F: fmt::Write,
{
    let needs_brackets = (uri.is_empty() && link_type == LinkType::Shortcut) || {
        let mut depth = 0;
        for b in uri.bytes() {
            match b {
                b'(' => depth += 1,
                b')' => depth -= 1,
                b' ' => {
                    depth += 1;
                    break;
                }
                _ => {}
            }
            if !(0..=3).contains(&depth) {
                break;
            }
        }
        depth != 0
    };
    let separator = match link_type {
        LinkType::Shortcut => ": ",
        _ => "(",
    };

    if needs_brackets {
        write!(f, "]{separator}<{}>", EscapeDestination(uri))?;
    } else {
        write!(f, "]{separator}{}", EscapeDestination(uri))?;
    }
    if !title.is_empty() {
        write!(f, " \"{title}\"", title = EscapeLinkTitle(title))?;
    }
    if link_type != LinkType::Shortcut {
        f.write_char(')')?;
    }

    Ok(())
}

struct EscapeDestination<'a>(&'a str);

impl fmt::Display for EscapeDestination<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for c in self.0.chars() {
            match c {
                '&' => f.write_str("&amp;")?,
                '\n' => f.write_str("&#10;")?,
                '\r' => f.write_str("&#13;")?,
                '\\' | '<' | '>' => {
                    f.write_char('\\')?;
                    f.write_char(c)?;
                }
                c => f.write_char(c)?,
            }
        }
        Ok(())
    }
}

struct EscapeLinkTitle<'a>(&'a str);

/// Writes a link title with double quotes escaped.
/// See https://spec.commonmark.org/0.30/#link-title for the rules around
/// link titles and the characters they may contain.
impl fmt::Display for EscapeLinkTitle<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for c in self.0.chars() {
            match c {
                '&' => f.write_str("&amp;")?,
                '"' => f.write_str(r#"\""#)?,
                '\\' => f.write_str(r"\\")?,
                c => f.write_char(c)?,
            }
        }
        Ok(())
    }
}

pub(crate) fn max_consecutive_chars(text: &str, search: char) -> usize {
    let mut in_search_chars = false;
    let mut max_count = 0;
    let mut cur_count = 0;

    for ch in text.chars() {
        if ch == search {
            cur_count += 1;
            in_search_chars = true;
        } else if in_search_chars {
            max_count = max_count.max(cur_count);
            cur_count = 0;
            in_search_chars = false;
        }
    }
    max_count.max(cur_count)
}

#[cfg(test)]
mod max_consecutive_chars {
    use super::max_consecutive_chars;

    #[test]
    fn happens_in_the_entire_string() {
        assert_eq!(
            max_consecutive_chars("``a```b``", '`'),
            3,
            "the highest seen consecutive segment of backticks counts"
        );
        assert_eq!(
            max_consecutive_chars("```a``b`", '`'),
            3,
            "it can't be downgraded later"
        );
    }
}

//=====================================
// General-purpose formatting utilities
//=====================================

/// `Repeated(content, count` formats as `content` repeated `count` times.
#[derive(Debug)]
pub(crate) struct Repeated<T>(pub T, pub usize);

impl<T: fmt::Display> fmt::Display for Repeated<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Repeated(content, count) = self;

        for _ in 0..*count {
            T::fmt(content, f)?;
        }
        Ok(())
    }
}
