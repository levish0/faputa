mod gen_data;

use criterion::{criterion_group, criterion_main, Criterion};

// ── nanachi ──
use nanachi_derive::Parser;

#[derive(Parser)]
#[grammar("grammar.nanachi")]
struct NanachiJson;

// ── pest ──
use pest::Parser as PestParser;

#[derive(pest_derive::Parser)]
#[grammar = "json.pest"]
struct PestJson;

fn bench_json(c: &mut Criterion) {
    let data = gen_data::generate_json();

    // Sanity checks
    NanachiJson::parse_json(&data).expect("nanachi failed");
    PestJson::parse(Rule::json, &data).expect("pest failed");
    serde_json::from_str::<serde_json::Value>(&data).expect("serde_json failed");

    let mut group = c.benchmark_group("json_parse");

    group.bench_function("nanachi", |b| {
        b.iter(|| NanachiJson::parse_json(criterion::black_box(&data)).unwrap())
    });

    group.bench_function("pest", |b| {
        b.iter(|| PestJson::parse(Rule::json, criterion::black_box(&data)).unwrap())
    });

    group.bench_function("serde_json", |b| {
        b.iter(|| {
            serde_json::from_str::<serde_json::Value>(criterion::black_box(&data)).unwrap()
        })
    });

    group.finish();
}

criterion_group!(benches, bench_json);
criterion_main!(benches);
