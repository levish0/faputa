# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.2] - 2026-04-06

### Added

- **Error messages** with accurate source positions
  - `LineIndex` in runtime crate: memchr-accelerated newline index with O(log n) binary search for byte-offset → line:col conversion
  - `StrContext::Label` on every rule for "invalid <rule>" messages
  - `StrContext::Expected` on string literals and char ranges for "expected ..." messages
  - `furthest_pos` tracking in `ParseState` to report the actual failure position after backtracking
  - Errors now format as `parse error at 3:12: invalid value\nexpected "null", "true", "false"` instead of raw `ContextError { context: [], cause: None }`

- **`memchr` dependency** added to nanachi runtime for SIMD-accelerated newline scanning

### Fixed

- Error position no longer reports `1:1` for all failures — uses furthest position reached during parsing

## [0.1.1] - 2026-04-06

### Added

- **Runtime crate** (`nanachi`): winnow-based parser runtime
  - `State` trait with flag/counter accessors and line position helpers
  - `Input` type alias wrapping `winnow::stream::Stateful<LocatingSlice<&str>, S>`

- **Code generator** (`nanachi_generator`): produces Rust + winnow parser code from AST
  - Per-rule entry points: `parse_<rule>(source) -> Result<&str, String>`
  - Automatic `alt()` chunking for >21 branches (winnow tuple limit)
  - Type unification via `.void()` on choice branches and `.fold()` on repeats
  - Full stateful codegen: `with`/`when`/`guard`/`depth_limit` expressions
  - `generate()` for build.rs (pub mod), `generate_with_mod()` for derive (hidden mod)

- **Derive macro** (`nanachi_derive`): `#[derive(Parser)]` proc macro
  - `#[grammar("path")]` to load from file
  - `#[grammar_inline("...")]` for inline grammars
  - Generates hidden module + `impl StructName` with `parse_<rule>()` methods

- **Examples**
  - `examples/parse_demo`: assignment parser, reads from file
  - `examples/parse_json`: full JSON (RFC 8259) grammar and file parser

- **Benchmarks** (`benches/json_bench`): criterion benchmarks comparing nanachi vs pest vs serde_json

- **winnow `simd` feature** enabled for memchr-accelerated literal matching

### Changed

- Fixture files moved to workspace root `fixtures/` for shared access across crates
- End-to-end tests (`tests/e2e`) use build.rs codegen with prettyplease formatting

## [0.1.0] - 2026-04-06

Initial release of the nanachi meta-compiler pipeline (`nanachi_meta`).

### Added

- **Lexer** (`nanachi_meta::lexer`): Logos-based tokenizer for `.nanachi` grammar files
  - All keywords, operators, delimiters, and built-in predicates (`SOI`, `EOI`, `ANY`, `LINE_START`, `LINE_END`)
  - String literals with escape sequences (`\n`, `\t`, `\r`, `\\`, `\"`)
  - Char literals with escape sequences for char ranges (`'a'..'z'`)
  - Line comments (`//`) and block comments (`/* */`)
  - Unrecognized characters reported as errors with position

- **AST** (`nanachi_meta::ast`): Full abstract syntax tree for the `.nanachi` DSL
  - State declarations: `let flag`, `let counter`
  - Rule definitions with guard statements, emit statements, and expressions
  - Expressions: string literals, char ranges, identifiers, sequences, choices, repetition (`+`, `*`, `?`, `{n,m}`), positive/negative lookahead, grouping, `with` (flag/counter increment), `when` conditionals, `depth_limit`

- **Parser** (`nanachi_meta::parser`): Hand-written recursive descent parser
  - Parses tokenized `.nanachi` source into the AST
  - Operator precedence: choice (`|`) < sequence (whitespace) < postfix (`+*?{n,m}`) < prefix (`&!`) < atoms
  - Save/restore backtracking for ambiguous `{` (repeat bounds vs. block)
  - Error reporting with source offset

- **Validator** (`nanachi_meta::validator`): Semantic validation pass
  - Duplicate rule and state detection
  - Undefined rule and state reference checking
  - State kind verification (flag vs. counter usage mismatch)
  - Built-in name shadowing prevention

- **Convenience API** (`nanachi_meta::compile`): Combined parse + validate entry point

- **Workspace structure**: Multi-crate workspace (`nanachi`, `nanachi_meta`, `nanachi_generator`, `nanachi_derive`, `nanachi_vm`, `nanachi_debugger`, `xtask`)

- **DSL specification**: `docs/nanachi-spec.md` with full syntax reference and winnow codegen mapping