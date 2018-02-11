extern crate pulldown_cmark;
extern crate pulldown_cmark_to_cmark;

use pulldown_cmark::Parser;
use pulldown_cmark_to_cmark::fmt::cmark;
use std::env;
use std::io::stdout;

use std::io::{Read, Write};
use std::fs::File;
use std::ffi::OsString;

fn main() {
    let path = env::args_os()
        .skip(1)
        .next()
        .expect("First argument is markdown file to display");

    let md = read_to_string(path);
    let mut buf = String::with_capacity(md.len() + 128);
    cmark(
        Parser::new_ext(&md, pulldown_cmark::Options::all()),
        &mut buf,
        None,
    ).unwrap();
    stdout().write_all(buf.as_bytes()).unwrap();
}

fn read_to_string(path: OsString) -> String {
    let mut file = File::open(&path).expect("file to exist for reading");
    let mut buf = String::new();
    file.read_to_string(&mut buf).expect("file to be readable");
    buf
}
