use std::{fmt::Write as _, hint::black_box, time::Duration};

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode, Throughput};
use pulldown_cmark::{utils::TextMergeStream, Event, Parser};
use pulldown_cmark_to_cmark::cmark_with_source_range;

fn render(input: &str, edited: bool) -> String {
    let events = Parser::new(input).into_offset_iter().map(|(event, range)| {
        let event = if edited && event == Event::Text("A ".into()) {
            Event::Text("B ".into())
        } else {
            event
        };
        (event, Some(range))
    });
    let mut output = String::new();
    cmark_with_source_range(events, input, &mut output, pulldown_cmark::Options::empty()).unwrap();
    output
}

fn reference_links(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("source_range_reference_links");
    group.sample_size(20);
    group.sampling_mode(SamplingMode::Flat);
    group.warm_up_time(Duration::from_millis(250));
    group.measurement_time(Duration::from_secs(1));

    for workload in ["shared", "distinct", "edited", "nested"] {
        for count in [1000, 2000, 4000, 8000] {
            let mut input = String::new();
            for index in 0..count {
                if workload == "distinct" {
                    writeln!(input, "A [link{index}].\n").unwrap();
                } else if workload == "nested" {
                    input.push_str("***a*b*c* [link].\n\n");
                } else {
                    input.push_str("A [link].\n\n");
                }
            }
            if workload == "distinct" {
                for index in 0..count {
                    writeln!(input, "[link{index}]: /url{index}").unwrap();
                }
            } else {
                input.push_str("[link]: /url\n");
            }
            let edited = workload == "edited";
            let expected = if edited {
                input.replace("A ", "B ")
            } else {
                input.clone()
            };
            let output = render(&input, edited);
            assert_eq!(
                TextMergeStream::new(Parser::new(&expected)).collect::<Vec<_>>(),
                TextMergeStream::new(Parser::new(&output)).collect::<Vec<_>>()
            );

            group.throughput(Throughput::Bytes(input.len() as u64));
            group.bench_with_input(BenchmarkId::new(workload, count), &input, |bench, input| {
                bench.iter(|| render(black_box(input), edited));
            });
        }
    }
    group.finish();
}

criterion_group!(benches, reference_links);
criterion_main!(benches);
