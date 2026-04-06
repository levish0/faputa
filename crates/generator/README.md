# faputa_generator

Code generator for [faputa](https://github.com/levish0/faputa).

Takes optimized MIR from `faputa_meta` and produces Rust source code using
[winnow](https://docs.rs/winnow) combinators.

## Usage

Typically used through `faputa_derive`, but can be called directly for
build-script codegen:

```rust
use faputa_meta::compile;
use faputa_generator::generate;

let grammar = compile("number = { '0'..'9'+ }").unwrap();
let code: String = generate(&grammar);
// Write `code` to a file in your build script
```

- **`generate()`** — produces a `pub mod` with `parse_<rule>()` entry points
- **`generate_with_mod()`** — produces a hidden module (used by the derive macro)

## What it generates

Each rule becomes a function. Entry-point rules get `.context(Label)` +
`track_pos()` wrapping. Internal rules get minimal wrappers. MIR shape nodes
such as `TakeWhile`, `Dispatch`, `Scan`, and `SeparatedList` map directly to
specialized winnow code.

## License

[Apache-2.0](../../LICENSE)
