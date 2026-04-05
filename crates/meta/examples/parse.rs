/// Parse a .nanachi file and print its AST.
///
/// ```sh
/// cargo run -p nanachi_meta --example parse -- examples/simple.nanachi
/// cargo run -p nanachi_meta --example parse -- examples/markdown_bold.nanachi
/// ```

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: parse <file.nanachi>");
        std::process::exit(1);
    });

    let source = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("Error reading {path}: {e}");
        std::process::exit(1);
    });

    match nanachi_meta::compile(&source) {
        Ok(grammar) => {
            println!("=== Parsed {path} ===\n");
            for item in &grammar.items {
                println!("{item:#?}\n");
            }
        }
        Err(e) => {
            eprintln!("Error: {e:?}");
            std::process::exit(1);
        }
    }
}
