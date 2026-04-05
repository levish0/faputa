use nanachi_derive::Parser;
use std::env;
use std::fs;
use std::process;

#[derive(Parser)]
#[grammar("grammar.nanachi")]
struct Demo;

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        let manifest = env!("CARGO_MANIFEST_DIR");
        format!("{manifest}/sample.txt")
    });

    let source = fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("error: cannot read {path}: {e}");
        process::exit(1);
    });

    for (i, line) in source.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let lineno = i + 1;
        match Demo::parse_assign(line) {
            Ok(matched) => println!("  line {lineno}: OK  {matched}"),
            Err(e) => println!("  line {lineno}: ERR {e}"),
        }
    }
}