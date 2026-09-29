use std::collections::BTreeSet;

use pulldown_cmark::{utils::TextMergeStream, Options as ParserOptions, Parser};
use pulldown_cmark_to_cmark::{
    cmark_with_options, cmark_with_source_range_and_options, Options, OptionsError, State, SUPPORTED_PARSER_OPTIONS,
};

#[path = "support/spec.rs"]
mod spec;

const REGRESSIONS: &[&str] = include!("support/regressions.rs");
const FORMATTING_COMBINATIONS: usize = 192;

fn formats() -> Vec<Options<'static>> {
    (0..FORMATTING_COMBINATIONS)
        .map(|mut i| {
            let mut take = |base| {
                let digit = i % base;
                i /= base;
                digit
            };
            Options {
                code_block_token: ['`', '~'][take(2)],
                list_token: ['*', '-', '+'][take(3)],
                ordered_list_token: ['.', ')'][take(2)],
                increment_ordered_list_bullets: take(2) != 0,
                emphasis_token: ['*', '_'][take(2)],
                strong_token: ["**", "__"][take(2)],
                use_html_for_super_sub_script: take(2) != 0,
                ..Options::default()
            }
        })
        .collect()
}

fn profiles() -> [(&'static str, ParserOptions); 6] {
    let gfm = ParserOptions::ENABLE_TABLES
        | ParserOptions::ENABLE_FOOTNOTES
        | ParserOptions::ENABLE_STRIKETHROUGH
        | ParserOptions::ENABLE_TASKLISTS
        | ParserOptions::ENABLE_GFM;
    let docs = gfm
        | ParserOptions::ENABLE_HEADING_ATTRIBUTES
        | ParserOptions::ENABLE_DEFINITION_LIST
        | ParserOptions::ENABLE_MATH;
    [
        ("CommonMark", ParserOptions::empty()),
        ("GFM", gfm),
        ("documentation", docs),
        (
            "documentation + YAML",
            docs | ParserOptions::ENABLE_YAML_STYLE_METADATA_BLOCKS,
        ),
        (
            "documentation + TOML",
            docs | ParserOptions::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS,
        ),
        ("all modern extensions", SUPPORTED_PARSER_OPTIONS),
    ]
}

#[derive(Default)]
struct Checks {
    total: usize,
    failed: usize,
}

impl Checks {
    fn check(&mut self, input: &str, parser_options: ParserOptions, options: &Options<'_>) {
        options.validate(parser_options).unwrap();
        let events: Vec<_> = Parser::new_ext(input, parser_options).into_offset_iter().collect();
        let expected: Vec<_> = TextMergeStream::new(events.iter().map(|(event, _)| event.clone())).collect();
        for source in [false, true] {
            self.total += 1;
            let mut output = String::new();
            let result = if source {
                cmark_with_source_range_and_options(
                    events.iter().map(|(event, range)| (event, Some(range.clone()))),
                    input,
                    &mut output,
                    parser_options,
                    options.clone(),
                )
            } else {
                cmark_with_options(
                    events.iter().map(|(event, _)| event),
                    &mut output,
                    parser_options,
                    options.clone(),
                )
            };
            let actual: Vec<_> = TextMergeStream::new(Parser::new_ext(&output, parser_options)).collect();
            if result.is_err() || expected != actual {
                self.failed += 1;
                if let Ok(path) = std::env::var("CMARK_DIFFERENTIAL_REPORT") {
                    use std::io::Write;
                    let mut report = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(path)
                        .unwrap();
                    writeln!(report, "{}\t{source}\t{input:?}\t{output:?}", parser_options.bits()).unwrap();
                }
                if self.failed <= 8 {
                    let mismatch = expected
                        .iter()
                        .zip(&actual)
                        .position(|(a, b)| a != b)
                        .unwrap_or(expected.len().min(actual.len()));
                    let start = mismatch.saturating_sub(2);
                    let expected_context = &expected[start..expected.len().min(mismatch + 6)];
                    let actual_context = &actual[start..actual.len().min(mismatch + 6)];
                    eprintln!("input: {input:?}\noutput: {output:?}\nflags: {parser_options:?}; source={source}; emphasis={:?}/{:?}; bullet={}; fence={}\nerror: {:?}\nfirst differing event: {mismatch}\nexpected: {expected_context:?}\nactual: {actual_context:?}\n", options.emphasis_token, options.strong_token, options.list_token, options.code_block_token, result.err());
                }
            }
        }
    }

    fn finish(self, name: &str) {
        println!(
            "{name}: {}/{} round trips passed; {} failed",
            self.total - self.failed,
            self.total,
            self.failed
        );
        assert_eq!(
            self.failed, 0,
            "{name}: differential failures (first eight printed above)"
        );
    }
}

#[test]
fn configuration_counts() {
    let mut accepted = 0;
    let mut unsupported = 0;
    let mut conflicts = 0;
    for bits in 0..1 << 15 {
        let parser_options = ParserOptions::from_bits_retain(bits << 1);
        for options in formats() {
            match options.validate(parser_options) {
                Ok(()) => accepted += 1,
                Err(OptionsError::UnsupportedParserOptions(_)) => unsupported += 1,
                Err(OptionsError::HtmlSuperSubscript) => conflicts += 1,
                other => panic!("unexpected validation result: {:?}", other),
            }
        }
    }
    println!("Configurations: {accepted} accepted; {} rejected ({unsupported} unsupported flags, {conflicts} HTML/script conflicts); {} total", unsupported + conflicts, accepted + unsupported + conflicts);
    assert_eq!((accepted, unsupported, conflicts), (491_520, 5_505_024, 294_912));
}

#[test]
fn validation_boundaries() {
    assert!(Options::default().validate(SUPPORTED_PARSER_OPTIONS).is_ok());
    assert!(Options::default().validate(ParserOptions::ENABLE_FOOTNOTES).is_ok());
    for flags in [
        ParserOptions::ENABLE_OLD_FOOTNOTES,
        ParserOptions::ENABLE_SMART_PUNCTUATION,
        ParserOptions::ENABLE_WIKILINKS,
        ParserOptions::from_bits_retain(1),
    ] {
        assert!(matches!(
            Options::default().validate(flags),
            Err(OptionsError::UnsupportedParserOptions(_))
        ));
    }
    for options in [
        Options {
            list_token: 'x',
            ..Options::default()
        },
        Options {
            ordered_list_token: ':',
            ..Options::default()
        },
        Options {
            code_block_token: '\n',
            ..Options::default()
        },
        Options {
            emphasis_token: 'é',
            ..Options::default()
        },
        Options {
            strong_token: "***",
            ..Options::default()
        },
    ] {
        assert!(matches!(
            options.validate(ParserOptions::empty()),
            Err(OptionsError::InvalidToken(_))
        ));
    }
    assert!(matches!(
        Options {
            newlines_after_metadata: usize::MAX,
            ..Options::default()
        }
        .validate(ParserOptions::empty()),
        Err(OptionsError::SizeOverflow("newlines_after_metadata"))
    ));
    assert!(matches!(
        Options {
            code_block_token_count: usize::MAX,
            ..Options::default()
        }
        .validate(ParserOptions::empty()),
        Err(OptionsError::SizeOverflow("code_block_token_count"))
    ));
    assert!(Options {
        code_block_token_count: isize::MAX as usize - 1,
        ..Options::default()
    }
    .validate(ParserOptions::empty())
    .is_ok());
    macro_rules! overflow {
        ($($field:ident),+ $(,)?) => {$(
            assert_eq!(Options { $field: isize::MAX as usize, ..Options::default() }.validate(ParserOptions::empty()),
                Err(OptionsError::SizeOverflow(stringify!($field))));
        )+};
    }
    overflow!(
        newlines_after_headline,
        newlines_after_paragraph,
        newlines_after_codeblock,
        newlines_after_htmlblock,
        newlines_after_table,
        newlines_after_rule,
        newlines_after_list,
        newlines_after_blockquote,
        newlines_after_rest,
        newlines_after_metadata,
        code_block_token_count
    );
}

fn with_spacing(value: usize) -> Options<'static> {
    Options {
        newlines_after_headline: value,
        newlines_after_paragraph: value,
        newlines_after_codeblock: value,
        newlines_after_htmlblock: value,
        newlines_after_table: value,
        newlines_after_rule: value,
        newlines_after_list: value,
        newlines_after_blockquote: value,
        newlines_after_rest: value,
        newlines_after_metadata: value,
        code_block_token_count: value,
        ..Options::default()
    }
}

#[test]
fn numeric_preferences() {
    let mut checks = Checks::default();
    let cases = spec::parse_common_mark_testsuite();
    for value in [0, 1, 2, 3, 4, 8] {
        let options = with_spacing(value);
        for case in &cases {
            checks.check(&case.markdown, ParserOptions::empty(), &options);
        }
        for (_, flags) in profiles() {
            checks.check(INTERACTIONS, flags, &options);
            checks.check(
                "999999999. one\n999999999. two\n",
                flags,
                &Options {
                    increment_ordered_list_bullets: true,
                    ..options.clone()
                },
            );
        }
    }
    checks.finish("Numeric spacing and fence preferences");
}

#[test]
fn repository_fixtures() {
    let paths: BTreeSet<_> = std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "md"))
        .collect();
    let mut checks = Checks::default();
    for (_, flags) in profiles() {
        for path in &paths {
            let failures = checks.failed;
            checks.check(&std::fs::read_to_string(path).unwrap(), flags, &Options::default());
            if checks.failed != failures {
                eprintln!("Fixture: {} ({flags:?})", path.display());
            }
        }
    }
    checks.finish("Repository fixtures across six parser profiles");
}

#[test]
fn retained_inline_regressions() {
    retained_regressions(false);
}

#[test]
fn retained_block_regressions() {
    retained_regressions(true);
}

fn retained_regressions(multiline: bool) {
    let mut checks = Checks::default();
    for (_, flags) in profiles() {
        for input in REGRESSIONS.iter().filter(|input| input.contains('\n') == multiline) {
            checks.check(input, flags, &Options::default());
        }
    }
    checks.finish("Retained regressions");
}

const INTERACTIONS: &str = "# Heading *emphasis* {#id .class}\n\n***a*b*c* _*a* x*y*z_\n\n- [x] task\n- term\n  : def\n\n  text\n\n> [!NOTE]\n> a ~~strike~~ and ^up^ ~down~\n\nA [link] and [^note].\n\n[link]: /target \"title\"\n\n[^note]: footnote\n\n| a | b |\n|:-|--:|\n| $x\\|y$ | `a\\|b` |\n\nterm\n\n: definition\n\n$$x+y$$\n\n~~~ rust\ncode\n~~~\n\n<!-- comment -->\n";

#[test]
#[ignore = "exhaustive configuration probes; run in release mode"]
fn every_accepted_configuration() {
    let mut checks = Checks::default();
    let formats = formats();
    let inputs = [
        format!("---\ntitle: YAML\n---\n\n{INTERACTIONS}"),
        format!("+++\ntitle = 'TOML'\n+++\n\n{INTERACTIONS}"),
    ];
    let mut accepted = 0;
    for bits in 0..1 << 15 {
        let flags = ParserOptions::from_bits_retain(bits << 1);
        if !SUPPORTED_PARSER_OPTIONS.contains(flags) {
            continue;
        }
        for options in &formats {
            if options.validate(flags).is_ok() {
                accepted += 1;
                for input in &inputs {
                    checks.check(input, flags, options);
                }
            }
        }
    }
    assert_eq!(accepted, 491_520);
    checks.finish("Every accepted configuration (YAML and TOML probes, both rendering paths)");
}

#[test]
#[ignore = "all standard formatting combinations; run in release mode"]
fn commonmark_with_all_formats() {
    let cases = spec::parse_common_mark_testsuite();
    assert_eq!(cases.len(), spec::COMMONMARK_SPEC_EXAMPLE_COUNT);
    let mut checks = Checks::default();
    for options in formats() {
        for case in &cases {
            let failures = checks.failed;
            checks.check(&case.markdown, ParserOptions::empty(), &options);
            if checks.failed != failures && checks.failed <= 8 {
                eprintln!(
                    "CommonMark fixture line {}: HTML {}",
                    case.line_number, case.expected_html
                );
            }
        }
    }
    checks.finish("CommonMark 0.31.2 across all 192 formatting combinations");
}

fn corpus() -> BTreeSet<String> {
    let runs = ["", "*", "**", "***", "****", "_", "__", "___", "____"];
    let mut inputs: BTreeSet<_> = REGRESSIONS.iter().map(|s| s.to_string()).collect();
    for (a, b, c) in [
        ("a", "b", "c"),
        ("a ", "b ", "c"),
        ("a", " b ", "c"),
        ("a!", "b", "c?"),
        ("é", "α", "世"),
        ("a", "b\u{301}", "c"),
    ] {
        for r1 in runs {
            for r2 in runs {
                for r3 in runs {
                    for r4 in runs {
                        inputs.insert(format!("{r1}{a}{r2}{b}{r3}{c}{r4}"));
                    }
                }
            }
        }
    }
    let words = ["a", "a ", " a", "!a", "a!", "!", "α", "\u{301}", ""];
    let mut seed = 0xa37f_2039_d785_326bu64;
    let mut random = |limit| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed % limit as u64) as usize
    };
    for _ in 0..50_000 {
        let mut input = String::new();
        for _ in 0..2 + random(7) {
            input.push_str(runs[random(runs.len())]);
            input.push_str(words[random(words.len())]);
        }
        input.push_str(runs[random(runs.len())]);
        inputs.insert(input);
    }
    for first in [
        "term\n: def",
        "term\n\n: def",
        "<hr>",
        "<!-- comment -->",
        "<script>\nx\n</script>",
        "[^a]: def",
        "a | b\n- | -\nx | y",
        "> a",
        "```\ncode\n```",
        "    code",
        "# h",
        "---",
        "- a",
        "- > a",
    ] {
        for next in [
            "text",
            "`code`",
            "# h",
            "> b",
            "    code",
            "```\nx\n```",
            "term\n: next",
            "<div>",
            "- item",
            "1. item",
            "---",
            "[^b]: next",
            "a | b\n- | -\nx | y",
        ] {
            for gap in ["\n", "\n\n"] {
                let body = format!("{first}{gap}{next}");
                for input in [
                    body.clone(),
                    format!("- {}", body.replace('\n', "\n  ")),
                    format!("- outer\n  - {}", body.replace('\n', "\n    ")),
                    format!("> {}", body.replace('\n', "\n> ")),
                    format!("- {first}\n- {next}"),
                ] {
                    inputs.insert(format!("{input}\n"));
                    inputs.insert(input);
                }
            }
        }
    }
    inputs
}

#[test]
#[ignore = "generated differential corpus; run in release mode"]
fn generated_combinations() {
    let inputs = corpus();
    println!(
        "Generated corpus: {} distinct inputs; seed 0xa37f2039d785326b",
        inputs.len()
    );
    for (name, flags) in profiles() {
        let mut checks = Checks::default();
        for emphasis_token in ['*', '_'] {
            for strong_token in ["**", "__"] {
                let options = Options {
                    emphasis_token,
                    strong_token,
                    ..Options::default()
                };
                for input in &inputs {
                    checks.check(input, flags, &options);
                }
            }
        }
        checks.finish(name);
    }
}

#[test]
#[ignore = "generated extension delimiters; run in release mode"]
fn extension_delimiters() {
    let runs = ["", "~", "~~", "~~~", "^", "^^", "^^^"];
    let mut checks = Checks::default();
    for flags in [
        ParserOptions::ENABLE_STRIKETHROUGH,
        ParserOptions::ENABLE_SUBSCRIPT,
        ParserOptions::ENABLE_SUPERSCRIPT,
        ParserOptions::ENABLE_STRIKETHROUGH | ParserOptions::ENABLE_SUBSCRIPT,
        SUPPORTED_PARSER_OPTIONS,
    ] {
        for (a, b, c) in [("a", "b", "c"), ("a!", "b", "c?"), ("a ", " b ", "c")] {
            for r1 in runs {
                for r2 in runs {
                    for r3 in runs {
                        for r4 in runs {
                            checks.check(&format!("{r1}{a}{r2}{b}{r3}{c}{r4}"), flags, &Options::default());
                        }
                    }
                }
            }
        }
    }
    checks.finish("Symbolic extension delimiter combinations");
}

#[test]
fn incremental_interactions() {
    for (_, flags) in profiles() {
        for input in [INTERACTIONS,
            "[cmark-loose-1]\n\n- [unused]: /unused\n\n  item\n\n[cmark-loose-2]\n\n[cmark-loose-2]: /target\n\n[cmark-loose-3]\n\n- outer\n  - [unused-again]: /unused\n",
            "[link]\n\n[link]: /url\n\n- term\n  : ```\n    code",
            "[link]\n\n[link]: /url\n\n<!-- EOF",
            "[link]\n\n[link]: /url\n\n- <script>\n```\ncode",
            "* term\n  : > <script>\n\n  text",
            "* term\n  : > <!-- open\n\n  text",
            "- $x$\n  - [unused]: /url\n",
            "- [x]\n  - [unused]: /url\n",
        ] {
        let events: Vec<_> = Parser::new_ext(input, flags).into_offset_iter().collect();
        let expected: Vec<_> = TextMergeStream::new(events.iter().map(|(event, _)| event.clone())).collect();
        for source in [false, true] {
            for split in 0..=events.len() {
                let mut state = State::new(flags, Options::default());
                let mut output = String::new();
                for chunk in [&events[..split], &events[split..]] {
                    if source {
                        state.process_with_source_range(chunk.iter().map(|(event, range)| (event, Some(range.clone()))), input, &mut output).unwrap();
                    } else {
                        state.process(chunk.iter().map(|(event, _)| event), &mut output).unwrap();
                    }
                }
                state.finish(&mut output).unwrap();
                assert_eq!(expected, TextMergeStream::new(Parser::new_ext(&output, flags)).collect::<Vec<_>>(), "flags={flags:?} source={source} split={split}: {output:?}");
            }
        }
        }
    }
}

#[test]
#[ignore = "inspect an individual input supplied in CMARK_INPUT"]
fn inspect_input() {
    let input = std::env::var("CMARK_INPUT").unwrap_or_else(|_| "***a*b*c".into());
    eprintln!("CommonMark input events: {:?}", Parser::new(&input).collect::<Vec<_>>());
    let mut checks = Checks::default();
    for (_, flags) in profiles() {
        checks.check(&input, flags, &Options::default());
    }
    checks.finish("Single input");
}
