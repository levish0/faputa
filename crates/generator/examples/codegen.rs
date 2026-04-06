/// Generate winnow parser code from a .nanachi file.
///
/// ```sh
/// cargo run -p nanachi_generator --example codegen -- examples/simple.nanachi
/// cargo run -p nanachi_generator --example codegen -- examples/markdown_bold.nanachi
/// ```
///
/// Set `RUST_LOG=debug` to see tracing output from the compilation pipeline.

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let path = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: codegen <file.nanachi>");
        std::process::exit(1);
    });

    let source = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("Error reading {path}: {e}");
        std::process::exit(1);
    });

    let grammar = nanachi_meta::compile(&source).unwrap_or_else(|e| {
        eprintln!("Error: {e:?}");
        std::process::exit(1);
    });

    let code = nanachi_generator::generate(&grammar);

    // Pretty-print if possible, fall back to raw token stream
    match syn::parse2::<syn::File>(code.clone()) {
        Ok(file) => print!("{}", prettyplease::unparse(&file)),
        Err(_) => print!("{code}"),
    }
}
