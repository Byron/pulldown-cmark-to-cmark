use std::{
    env,
    io::{stdout, Write},
};

use pulldown_cmark::Parser;
use pulldown_cmark_to_cmark::{cmark_with_options, State};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args_os()
        .nth(1)
        .expect("First argument is markdown file to display");
    let event_by_event = env::var_os("STUPICAT_STATE_TEST").is_some();

    let md = std::fs::read_to_string(&path)?;
    let mut buf = String::with_capacity(md.len() + 128);
    let options = pulldown_cmark_to_cmark::SUPPORTED_PARSER_OPTIONS;

    let render_options = pulldown_cmark_to_cmark::Options::default();

    render_options.validate(options)?;

    if event_by_event {
        let mut state = State::new(options, render_options);
        for event in Parser::new_ext(&md, options) {
            state.process(std::iter::once(event), &mut buf)?;
        }
        state.finish(&mut buf)?;
    } else {
        cmark_with_options(Parser::new_ext(&md, options), &mut buf, options, render_options)?;
    }

    stdout().write_all(buf.as_bytes())?;
    Ok(())
}
