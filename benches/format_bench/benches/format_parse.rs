mod gen_data;

use criterion::{Criterion, criterion_group, criterion_main};
use pest::Parser as PestParser;

use format_bench::{manual_csv, manual_http, manual_ini, manual_json};

mod json {
    use faputa_derive::Parser;

    #[derive(Parser)]
    #[grammar("json.faputa")]
    pub struct FaputaJson;

    #[derive(pest_derive::Parser)]
    #[grammar = "json.pest"]
    pub struct PestJson;
}

mod csv {
    use faputa_derive::Parser;

    #[derive(Parser)]
    #[grammar("csv.faputa")]
    pub struct FaputaCsv;

    #[derive(pest_derive::Parser)]
    #[grammar = "csv.pest"]
    pub struct PestCsv;
}

mod ini {
    use faputa_derive::Parser;

    #[derive(Parser)]
    #[grammar("ini.faputa")]
    pub struct FaputaIni;

    #[derive(pest_derive::Parser)]
    #[grammar = "ini.pest"]
    pub struct PestIni;
}

mod http {
    use faputa_derive::Parser;

    #[derive(Parser)]
    #[grammar("http.faputa")]
    pub struct FaputaHttp;

    #[derive(pest_derive::Parser)]
    #[grammar = "http.pest"]
    pub struct PestHttp;
}

fn bench_json(c: &mut Criterion) {
    let data = gen_data::generate_json();

    json::FaputaJson::parse_json(&data).expect("faputa json failed");
    manual_json::parse(&data).expect("manual json failed");
    json::PestJson::parse(json::Rule::json, &data).expect("pest json failed");
    serde_json::from_str::<serde_json::Value>(&data).expect("serde_json failed");

    let mut group = c.benchmark_group("json_parse");

    group.bench_function("faputa", |b| {
        b.iter(|| json::FaputaJson::parse_json(std::hint::black_box(&data)).unwrap())
    });

    group.bench_function("manual_json", |b| {
        b.iter(|| manual_json::parse(std::hint::black_box(&data)).unwrap())
    });

    group.bench_function("pest", |b| {
        b.iter(|| json::PestJson::parse(json::Rule::json, std::hint::black_box(&data)).unwrap())
    });

    group.bench_function("serde_json", |b| {
        b.iter(|| serde_json::from_str::<serde_json::Value>(std::hint::black_box(&data)).unwrap())
    });

    group.finish();
}

fn bench_csv(c: &mut Criterion) {
    let data = gen_data::generate_csv();

    csv::FaputaCsv::parse_file(&data).expect("faputa csv failed");
    manual_csv::parse(&data).expect("manual csv failed");
    csv::PestCsv::parse(csv::Rule::file, &data).expect("pest csv failed");

    let mut group = c.benchmark_group("csv_parse");

    group.bench_function("faputa", |b| {
        b.iter(|| csv::FaputaCsv::parse_file(std::hint::black_box(&data)).unwrap())
    });

    group.bench_function("manual_csv", |b| {
        b.iter(|| manual_csv::parse(std::hint::black_box(&data)).unwrap())
    });

    group.bench_function("pest", |b| {
        b.iter(|| csv::PestCsv::parse(csv::Rule::file, std::hint::black_box(&data)).unwrap())
    });

    group.finish();
}

fn bench_ini(c: &mut Criterion) {
    let data = gen_data::generate_ini();

    ini::FaputaIni::parse_file(&data).expect("faputa ini failed");
    manual_ini::parse(&data).expect("manual ini failed");
    ini::PestIni::parse(ini::Rule::file, &data).expect("pest ini failed");

    let mut group = c.benchmark_group("ini_parse");

    group.bench_function("faputa", |b| {
        b.iter(|| ini::FaputaIni::parse_file(std::hint::black_box(&data)).unwrap())
    });

    group.bench_function("manual_ini", |b| {
        b.iter(|| manual_ini::parse(std::hint::black_box(&data)).unwrap())
    });

    group.bench_function("pest", |b| {
        b.iter(|| ini::PestIni::parse(ini::Rule::file, std::hint::black_box(&data)).unwrap())
    });

    group.finish();
}

fn bench_http(c: &mut Criterion) {
    let data = gen_data::generate_http();

    http::FaputaHttp::parse_http(&data).expect("faputa http failed");
    manual_http::parse(&data).expect("manual http failed");
    http::PestHttp::parse(http::Rule::http, &data).expect("pest http failed");

    let mut group = c.benchmark_group("http_parse");

    group.bench_function("faputa", |b| {
        b.iter(|| http::FaputaHttp::parse_http(std::hint::black_box(&data)).unwrap())
    });

    group.bench_function("manual_http", |b| {
        b.iter(|| manual_http::parse(std::hint::black_box(&data)).unwrap())
    });

    group.bench_function("pest", |b| {
        b.iter(|| http::PestHttp::parse(http::Rule::http, std::hint::black_box(&data)).unwrap())
    });

    group.finish();
}

criterion_group!(benches, bench_json, bench_csv, bench_ini, bench_http);
criterion_main!(benches);
