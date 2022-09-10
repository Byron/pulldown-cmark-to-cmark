use std::{
    env,
    ffi::OsString,
    fs::File,
    io::{stdout, Read, Write},
};

use pulldown_cmark::{Options, Parser};
use pulldown_cmark_to_cmark::{cmark, cmark_resume};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args_os()
        .nth(1)
        .expect("First argument is markdown file to display");
    let event_by_event = env::var_os("STUPICAT_STATE_TEST").is_some();

    let md = read_to_string(path);
    let mut buf = String::with_capacity(md.len() + 128);
    let mut options = Options::all();
    options.remove(Options::ENABLE_SMART_PUNCTUATION);

    if event_by_event {
        let mut state = None;
        for event in Parser::new_ext(&md, options) {
            state = cmark_resume(std::iter::once(event), &mut buf, state.take())?.into();
        }
        if let Some(state) = state {
            state.finalize(&mut buf)?;
        }
    } else {
        cmark(Parser::new_ext(&md, options), &mut buf)?;
    }

    stdout().write_all(buf.as_bytes())?;
    Ok(())
}

fn read_to_string(path: OsString) -> String {
    let mut file = File::open(&path).expect("file to exist for reading");
    let mut buf = String::new();
    file.read_to_string(&mut buf).expect("file to be readable");
    buf
}
