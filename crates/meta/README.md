# faputa_meta

Grammar compiler for [faputa](https://github.com/levish0/faputa).

Parses `.faputa` grammar files and produces validated HIR plus optimized MIR
ready for code generation.

## Pipeline

```
source → Lexer → Parser → AST → Validator → HIR Lowering → HIR Optimize → MIR Lowering → MIR Optimize
```

- **Lexer** — Logos-based tokenizer
- **Parser** — hand-written recursive descent, full PEG expression support
- **Validator** — duplicate/undefined rule checking, state kind verification
- **HIR Lowering** — AST → typed HIR with resolved rule references
- **HIR Optimize** — semantic normalization, trivial/small-rule inlining,
  dead rule elimination, ref-count analysis
- **MIR Lowering / Optimize** — parser-shape lowering such as `Dispatch`,
  `TakeWhile`, `Scan`, and `SeparatedList`

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
