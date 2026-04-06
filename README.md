# nanachi

A stateful parser generator that compiles `.nanachi` grammar files into fast
[winnow](https://docs.rs/winnow)-based Rust parsers.

## Features

- **PEG-style grammar DSL** — sequences, ordered choice, repetition, lookahead,
  character ranges, and built-in boundaries (`SOI`, `EOI`, `ANY`)
- **Stateful parsing** — first-class flags, counters, guards, and recursion
  depth limits for context-sensitive grammars
- **Compile-time codegen** — generates Rust code via derive macro or build
  script; no runtime interpretation overhead
- **IR optimization pipeline** — trivial rule inlining, literal fusion, CharSet
  merging, flatten, and `TakeWhile` recognition (maps to winnow's
  SIMD-accelerated `take_while`)
- **Custom error messages** — `@` label syntax at rule and expression level for
  user-friendly parse errors with accurate source positions
- **Tracing** — `RUST_LOG=debug` shows the full compilation pipeline; the
  `debug` feature enables winnow's runtime parse-tree tracing

## Quick Start

Add the dependencies:

```toml
[dependencies]
nanachi = "0.1"
nanachi_derive = "0.1"
```

Write a grammar file (`grammar.nanachi`):

```nanachi
alpha  = { 'a'..'z' | 'A'..'Z' | "_" }
digit  = { '0'..'9' }
ident  = { alpha (alpha | digit)* }
number = { digit+ }
assign = { ident "=" (number | ident) }
```

Derive the parser:

```rust
use nanachi_derive::Parser;

#[derive(Parser)]
#[grammar("grammar.nanachi")]
struct MyParser;

fn main() {
    match MyParser::parse_assign("x=42") {
        Ok(matched) => println!("parsed: {matched}"),
        Err(e) => eprintln!("{e}"),
    }
}
```

Each rule generates a `parse_<name>(&str) -> Result<&str, String>` method.

## Grammar Syntax

```nanachi
// Rules
rule_name = { body }

// Terminals
"hello"              // literal string
'a'..'z'             // character range (inclusive)
ANY                  // any single character
SOI  EOI             // start / end of input
LINE_START  LINE_END // line boundaries

// Combinators
a b c                // sequence
a | b | c            // ordered choice
a*                   // zero or more
a+                   // one or more
a?                   // optional
a{3}                 // exactly 3
a{2,5}               // 2 to 5
&a                   // positive lookahead
!a                   // negative lookahead
(a | b) c            // grouping

// Error labels
rule = @ "human name" { ... }   // rule-level
expr @ "description"            // expression-level

// Stateful extensions
let flag verbose
let counter depth

rule = {
    guard verbose          // only run if flag is set
    with verbose { ... }   // set flag inside block
    when depth < 10 { ... }
    depth_limit 100 { ... }
}
```

## Error Labels

Without labels, errors show rule names:

```
parse error at 1:2: invalid assign
```

With `@` labels you control the message:

```nanachi
ident  = @ "identifier" { alpha (alpha | digit)* }
number = @ "number" { digit+ }

value = @ "value" {
    number @ "a number"
  | ident  @ "an identifier"
}

assign = { ident "=" @ "equals sign" value }
```

```
parse error at 1:2: invalid assign
expected equals sign
```

## Examples

| Example        | Description           | Run                                     |
|----------------|-----------------------|-----------------------------------------|
| `parse_demo`   | Assignment parser     | `cargo run -p parse_demo -- file.txt`   |
| `parse_json`   | Full RFC 8259 JSON    | `cargo run -p parse_json -- file.json`  |
| `error_labels` | Custom error messages | `cargo run -p error_labels -- file.txt` |

## Crate Structure

| Crate               | Purpose                                                       |
|---------------------|---------------------------------------------------------------|
| `nanachi`           | Runtime (re-exports winnow types, `LineIndex`, `State` trait) |
| `nanachi_meta`      | Lexer → Parser → Validator → IR lowering → Optimizer          |
| `nanachi_generator` | IR → Rust/winnow codegen                                      |
| `nanachi_derive`    | `#[derive(Parser)]` proc macro                                |
| `nanachi_vm`        | Bytecode VM interpreter (WIP)                                 |

## Feature Flags

| Feature | Effect                                                                         |
|---------|--------------------------------------------------------------------------------|
| `debug` | Enables winnow's `trace()` combinator — prints parse tree to stderr at runtime |

```toml
nanachi = { version = "0.1", features = ["debug"] }
```

## Tracing

All example binaries include a tracing subscriber. Set `RUST_LOG` to see
compilation internals:

```sh
RUST_LOG=debug cargo run -p parse_json -- file.json
```

## License

[Apache-2.0](LICENSE)