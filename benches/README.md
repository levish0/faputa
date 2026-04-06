# Benchmarks

This directory contains parser benchmark assets.

Current workspace benchmark package:

- `format_bench`

Run benchmarks from the repository root.

## Run All

```sh
cargo bench -p format_bench
```

Compile the benchmark binary without running it:

```sh
cargo bench -p format_bench --no-run
```

Run the benchmark target directly:

```sh
cargo bench -p format_bench --bench format_parse
```

## Run One Suite

Criterion filters can be passed after `--`.

```sh
cargo bench -p format_bench --bench format_parse -- json_parse
cargo bench -p format_bench --bench format_parse -- csv_parse
cargo bench -p format_bench --bench format_parse -- ini_parse
cargo bench -p format_bench --bench format_parse -- http_parse
```

## Current Comparisons

- `json_parse`: `faputa`, `manual_json`, `pest`, `serde_json`
- `csv_parse`: `faputa`, `manual_csv`, `pest`
- `ini_parse`: `faputa`, `manual_ini`, `pest`
- `http_parse`: `faputa`, `manual_http`, `pest`

## Output

Criterion writes reports under:

```text
target/criterion/
```

HTML reports are generated there when the benchmark run completes successfully.
