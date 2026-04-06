# faputa

Runtime crate for [faputa](https://github.com/levish0/faputa), a stateful
parser generator.

This crate provides the types and traits that generated parsers depend on at
runtime. You do **not** use this crate directly — it is pulled in
automatically when you use `faputa_derive`.

## What's inside

- **`Input`** — winnow input type with position tracking and stateful context
- **`State`** — runtime state container (flags, counters, position tracking)
- **`LineIndex`** — memchr-accelerated newline index for byte-offset → line:col conversion
- **Re-exported `winnow`** — generated code references `faputa::winnow::*`

## Feature Flags

| Feature | Effect |
|---------|--------|
| `debug` | Enables winnow's `trace()` combinator — prints parse tree to stderr at runtime |

## License

[Apache-2.0](../../LICENSE)
