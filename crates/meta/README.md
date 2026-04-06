# faputa_meta

Grammar compiler for [faputa](https://github.com/levish0/faputa).

Parses `.faputa` grammar files and produces a validated, optimized IR ready for
code generation.

## Pipeline

```
source → Lexer → Parser → AST → Validator → IR Lowering → Optimizer → IrProgram
```

- **Lexer** — Logos-based tokenizer
- **Parser** — hand-written recursive descent, full PEG expression support
- **Validator** — duplicate/undefined rule checking, state kind verification
- **IR Lowering** — AST → typed IR with resolved rule references
- **Optimizer** — trivial rule inlining, literal fusion, CharSet merging,
  flatten, TakeWhile recognition, dead rule elimination, ref-count analysis

## Usage

```rust
use faputa_meta::compile;

let grammar = compile(r#"
    digit = { '0'..'9' }
    number = { digit+ }
"#).unwrap();
```

For lower-level access, call `parser::parse()` and `validator::validate()`
individually.

## License

[Apache-2.0](../../LICENSE)
