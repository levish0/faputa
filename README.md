# faputa

A stateful parser generator that compiles `.faputa` grammar files into fast
[winnow](https://docs.rs/winnow)-based Rust parsers.

The goal is to make context-sensitive grammars — the kind that trip up pure PEG
parsers — expressible and readable. Think Markdown formatting rules, indentation
tracking, or nesting-depth limits: things that require remembering *what you've
already seen* while parsing.

## Features

- **PEG-style grammar DSL** — sequences, ordered choice, repetition, lookahead,
  character ranges, and built-in boundaries (`SOI`, `EOI`, `ANY`, `LINE_START`,
  `LINE_END`)
- **Stateful parsing** — first-class `flag` and `counter` declarations with
  scoped mutation (`with`), conditional dispatch (`when`), rule-level guards,
  and recursion depth limits
- **Compile-time codegen** — generates Rust code via derive macro or build
  script; no runtime interpretation overhead
- **IR optimization pipeline** — rule inlining, literal fusion, `CharSet`
  merging, `TakeWhile` recognition (maps to winnow's SIMD-accelerated
  `take_while`), and dispatch table generation
- **Custom error messages** — `@` label syntax at rule and expression level for
  user-friendly parse errors with accurate source positions
- **Tracing** — `RUST_LOG=debug` shows the full compilation pipeline; the
  `debug` feature enables winnow's runtime parse-tree tracing

## Quick Start

### Derive Macro

```toml
[dependencies]
faputa = "0.1"
faputa_derive = "0.1"
```

Write a grammar file (`grammar.faputa`):

```faputa
alpha  = { 'a'..'z' | 'A'..'Z' | "_" }
digit  = { '0'..'9' }
ident  = { alpha (alpha | digit)* }
number = { digit+ }
assign = { ident "=" (number | ident) }
```

Derive the parser:

```rust
use faputa_derive::Parser;

#[derive(Parser)]
#[grammar("grammar.faputa")]
struct MyParser;

fn main() {
    match MyParser::parse_assign("x=42") {
        Ok(matched) => println!("parsed: {matched}"),
        Err(e) => eprintln!("{e}"),
    }
}
```

Each rule generates a `parse_<name>(&str) -> Result<&str, String>` method.

### `build.rs` Codegen

If you prefer generating sources at build time (pest-style):

```toml
[dependencies]
faputa = "0.1"

[build-dependencies]
faputa_meta = "0.1"
faputa_generator = "0.1"
prettyplease = "0.2"
syn = "2"
```

```rust
// build.rs
use std::path::Path;

fn main() {
    let source = std::fs::read_to_string("grammar.faputa").unwrap();
    let grammar = faputa_meta::compile(&source).unwrap();
    let tokens = faputa_generator::generate(&grammar);

    let code = match syn::parse2::<syn::File>(tokens.clone()) {
        Ok(file) => prettyplease::unparse(&file),
        Err(_) => tokens.to_string(),
    };

    let out_file = Path::new(&std::env::var("OUT_DIR").unwrap()).join("grammar.rs");
    std::fs::write(&out_file, code).unwrap();

    println!("cargo::rerun-if-changed=grammar.faputa");
}
```

```rust
// main.rs
include!(concat!(env!("OUT_DIR"), "/grammar.rs"));

fn main() {
    let parsed = __faputa::parse_assign("x=42").unwrap();
    println!("{parsed}");
}
```

## Grammar Syntax

### Terminals

```faputa
"hello"              // literal string
'a'..'z'             // character range (inclusive)
ANY                  // any single character
SOI  EOI             // start / end of input
LINE_START  LINE_END // line boundaries (zero-width)
```

### Combinators

```faputa
a b c                // sequence
a | b | c            // ordered choice
a*                   // zero or more
a+                   // one or more
a?                   // optional
a{3}                 // exactly 3
a{2,5}               // 2 to 5
a{2,}                // at least 2
&a                   // positive lookahead (zero-width)
!a                   // negative lookahead (zero-width)
(a | b) c            // grouping
```

### State declarations

Declare state at the top of the grammar, before any rules:

```faputa
let flag   my_flag      // boolean, default false
let counter my_counter  // unsigned integer, default 0
```

### Stateful expressions

```faputa
with my_flag { ... }           // set flag to true inside block, restore on exit
with my_counter += 2 { ... }   // increment counter inside block, decrement on exit
when my_flag { ... }           // run body only if flag is true, else succeed empty
when !my_flag { ... }          // run body only if flag is false
when my_counter > 0 { ... }    // run body only if counter satisfies condition
depth_limit(64) { ... }        // fail if recursion exceeds 64 levels
```

Supported comparison operators in `when`: `==`, `!=`, `<`, `<=`, `>`, `>=`.

### Rule-level statements

These appear at the top of a rule body, before the main expression:

```faputa
rule = {
    guard my_flag          // fail immediately if flag is not set
    guard !my_flag         // fail immediately if flag is set
    guard my_counter > 0   // fail immediately if condition does not hold
    guard LINE_START        // fail immediately if not at start of line
    emit my_counter        // increment counter by 1 (not scoped — permanent)
    ...
}
```

### Error labels

```faputa
ident  = @ "identifier" { alpha (alpha | digit)* }   // rule-level label
number = @ "number"     { digit+ }

assign = {
    ident "=" @ "right-hand side" (number | ident)   // expression-level label
}
```

## Stateful Parsing Examples

### Preventing re-entrant inline formatting (Markdown-style)

Flags prevent bold/italic from nesting inside themselves:

```faputa
let flag inside_bold
let flag inside_italic

inline = { bold | italic | text }

bold = {
    guard !inside_bold
    with inside_bold {
        "**" inline+ "**"
    }
}

italic = {
    guard !inside_italic
    with inside_italic {
        "*" inline+ "*"
    }
}

text = { !("*") ANY }
```

### Tracking nesting depth

Counters track depth; `depth_limit` prevents runaway recursion:

```faputa
let counter brace_depth

document = {
    depth_limit(64) { block* }
}

block = {
    "{" with brace_depth += 1 { block* } "}"
}
```

### Conditional parsing based on position

Built-in predicates work inside `guard` and `when`:

```faputa
let counter section_count

header = {
    guard LINE_START
    emit section_count
    "#"{1,6} " " text
}
```

## Error Labels

Without labels, errors show raw rule names. With `@` labels you control the
message:

```faputa
ident  = @ "identifier" { alpha (alpha | digit)* }
number = @ "number" { digit+ }

value  = @ "value" {
    number @ "a number"
  | ident  @ "an identifier"
}

assign = { ident "=" @ "right-hand side" value }
```

A failed parse at `x=` would report:

```
parse error at 1:3
expected right-hand side (a number or an identifier)
```

## Examples

| Example        | Description                   | Run                                     |
|----------------|-------------------------------|-----------------------------------------|
| `parse_demo`   | Assignment parser             | `cargo run -p parse_demo -- file.txt`   |
| `parse_json`   | Full RFC 8259 JSON            | `cargo run -p parse_json -- file.json`  |
| `error_labels` | Custom error messages         | `cargo run -p error_labels -- file.txt` |

## Crate Structure

| Crate              | Purpose                                                       |
|--------------------|---------------------------------------------------------------|
| `faputa`           | Runtime: re-exports winnow types, `LineIndex`, `State` trait  |
| `faputa_meta`      | Lexer → Parser → Validator → HIR → MIR → Optimizer            |
| `faputa_generator` | MIR → Rust/winnow codegen                                     |
| `faputa_derive`    | `#[derive(Parser)]` proc macro                                |

## Feature Flags

| Feature | Effect                                                                         |
|---------|--------------------------------------------------------------------------------|
| `debug` | Enables winnow's `trace()` combinator — prints parse tree to stderr at runtime |

```toml
faputa = { version = "0.1", features = ["debug"] }
```

## Tracing

All example binaries include a tracing subscriber. Set `RUST_LOG` to see
compilation internals:

```sh
RUST_LOG=debug cargo run -p parse_json -- file.json
```

Output includes each pipeline stage: lex → parse → validate → HIR lower →
HIR optimize → MIR lower → MIR optimize → codegen.

## Publishing

```sh
cargo xtask publish-dry  # dry run
cargo xtask publish      # publish to crates.io
```

Crates are published in dependency order:
`faputa` → `faputa_meta` → `faputa_generator` → `faputa_derive`

## License

[Apache-2.0](LICENSE)
