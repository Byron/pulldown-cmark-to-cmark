use std::{
    env,
    io::{stdout, Write},
};

use pulldown_cmark::{Options, Parser};
use pulldown_cmark_to_cmark::{cmark_resume_with_options, cmark_with_options};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args_os()
        .nth(1)
        .expect("First argument is markdown file to display");
    let event_by_event = env::var_os("STUPICAT_STATE_TEST").is_some();

    let md = std::fs::read_to_string(&path)?;
    let mut buf = String::with_capacity(md.len() + 128);
    let mut options = Options::all();
    options.remove(Options::ENABLE_SMART_PUNCTUATION);

    let mut render_options = pulldown_cmark_to_cmark::Options::default();
    if env::var_os("STUPICAT_SUB_SUPER_SYMBOLIC").is_some() {
        render_options.use_html_for_super_sub_script = false;
    }

    if event_by_event {
        let mut state = None;
        for event in Parser::new_ext(&md, options) {
            state = cmark_resume_with_options(std::iter::once(event), &mut buf, state.take(), render_options.clone())?
                .into();
        }
        if let Some(state) = state {
            state.finalize(&mut buf)?;
        }
    } else {
        cmark_with_options(Parser::new_ext(&md, options), &mut buf, render_options)?;
    }

    stdout().write_all(buf.as_bytes())?;
    Ok(())
}
