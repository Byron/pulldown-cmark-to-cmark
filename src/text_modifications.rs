use super::{fmt, Cow, Options, State};

pub fn padding<F>(f: &mut F, p: &[Cow<'_, str>]) -> fmt::Result
where
    F: fmt::Write,
{
    for padding in p {
        write!(f, "{}", padding)?;
    }
    Ok(())
}
pub fn consume_newlines<F>(f: &mut F, s: &mut State<'_>) -> fmt::Result
where
    F: fmt::Write,
{
    while s.newlines_before_start != 0 {
        s.newlines_before_start -= 1;
        f.write_char('\n')?;
        padding(f, &s.padding)?;
    }
    Ok(())
}

pub fn escape_leading_special_characters<'a>(
    t: &'a str,
    is_in_block_quote: bool,
    options: &Options<'a>,
) -> Cow<'a, str> {
    if is_in_block_quote || t.is_empty() {
        return Cow::Borrowed(t);
    }

    let first = t.chars().next().expect("at least one char");
    if options.special_characters().contains(first) {
        let mut s = String::with_capacity(t.len() + 1);
        s.push('\\');
        s.push(first);
        s.push_str(&t[1..]);
        Cow::Owned(s)
    } else {
        Cow::Borrowed(t)
    }
}

pub fn print_text_without_trailing_newline<F>(t: &str, f: &mut F, p: &[Cow<'_, str>]) -> fmt::Result
where
    F: fmt::Write,
{
    if t.contains('\n') {
        let line_count = t.split('\n').count();
        for (tid, token) in t.split('\n').enumerate() {
            f.write_str(token).and(if tid + 1 == line_count {
                Ok(())
            } else {
                f.write_char('\n').and(padding(f, p))
            })?;
        }
        Ok(())
    } else {
        f.write_str(t)
    }
}

pub fn padding_of(l: Option<u64>) -> Cow<'static, str> {
    match l {
        None => "  ".into(),
        Some(n) => format!("{}. ", n).chars().map(|_| ' ').collect::<String>().into(),
    }
}
