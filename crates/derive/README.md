# faputa_derive

Derive macro for [faputa](https://github.com/levish0/faputa), a stateful
parser generator.

## Usage

```toml
[dependencies]
faputa = "0.1"
faputa_derive = "0.1"
```

### From a grammar file

```rust
use faputa_derive::Parser;

#[derive(Parser)]
#[grammar("grammar.faputa")]
struct MyParser;

fn main() {
    match MyParser::parse_number("42") {
        Ok(matched) => println!("{matched}"),
        Err(e) => eprintln!("{e}"),
    }
}
```

### Inline grammar

```rust
use faputa_derive::Parser;

#[derive(Parser)]
#[grammar_inline("number = { '0'..'9'+ }")]
struct MyParser;
```

Each rule in the grammar generates a `parse_<rule>(&str) -> Result<&str, String>` method on the struct.

## License

[Apache-2.0](../../LICENSE)
