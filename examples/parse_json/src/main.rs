use nanachi_derive::Parser;
use std::env;
use std::fs;
use std::process;

#[derive(Parser)]
#[grammar("grammar.nanachi")]
struct Json;

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        // Default to bundled sample
        let manifest = env!("CARGO_MANIFEST_DIR");
        format!("{manifest}/sample.json")
    });

    let source = fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("error: cannot read {path}: {e}");
        process::exit(1);
    });

    match Json::parse_json(&source) {
        Ok(matched) => {
            println!("Valid JSON ({} bytes)", matched.len());
            println!("{matched}");
        }
        Err(e) => {
            eprintln!("Invalid JSON: {e}");
            process::exit(1);
        }
    }
}