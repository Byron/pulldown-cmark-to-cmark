#![no_main]

use libfuzzer_sys::fuzz_target;
use pulldown_cmark::Parser;
use pulldown_cmark_to_cmark::cmark;

fn round_trip(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    cmark(Parser::new(&text), &mut result).unwrap();
    result
}

fn print_events(text: &str) {
    eprintln!("{text:?} -> [");
    for event in Parser::new(&text) {
        eprintln!("  {event:?}");
    }
    eprintln!("]");
}

fuzz_target!(|text: String| {
    let round_trip_1 = round_trip(&text);
    let round_trip_2 = round_trip(&round_trip_1);
    let round_trip_3 = round_trip(&round_trip_2);
    let round_trip_4 = round_trip(&round_trip_3);
    if round_trip_3 != round_trip_4 {
        print_events(&text);
        print_events(&round_trip_1);
        print_events(&round_trip_2);
        print_events(&round_trip_3);
        print_events(&round_trip_4);

        panic!(
            "round-trip failed:\n\
             -- {text:?}\n\
             -> {round_trip_1:?}\n\
             -> {round_trip_2:?}\n\
             -> {round_trip_3:?}\n\
             -> {round_trip_4:?}"
        );
    }
});
