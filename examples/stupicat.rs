use std::{
    env,
    ffi::OsString,
    fs::File,
    io::{stdout, Read, Write},
};

use pulldown_cmark::{Options, Parser};
use pulldown_cmark_to_cmark::cmark;

fn main() {
    let path = env::args_os()
        .skip(1)
        .next()
        .expect("First argument is markdown file to display");

    let md = read_to_string(path);
    let mut buf = String::with_capacity(md.len() + 128);
    let mut options = Options::all();
    options.remove(Options::ENABLE_SMART_PUNCTUATION);
    cmark(Parser::new_ext(&md, options), &mut buf, None).unwrap();
    stdout().write_all(buf.as_bytes()).unwrap();
}

fn read_to_string(path: OsString) -> String {
    let mut file = File::open(&path).expect("file to exist for reading");
    let mut buf = String::new();
    file.read_to_string(&mut buf).expect("file to be readable");
    buf
}
