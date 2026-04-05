# nanachi examples

Example `.nanachi` grammar files and how to run them.

## Files

| File                    | Description                                           |
|-------------------------|-------------------------------------------------------|
| `simple.nanachi`        | Basic grammar: identifiers from letters and digits    |
| `markdown_bold.nanachi` | Stateful parsing: bold markers with re-entrance guard |
| `nested_braces.nanachi` | Counter-based nesting with depth limit                |

## Parse a grammar (prints AST)

```sh
cargo run -p nanachi_meta --example parse -- examples/simple.nanachi
cargo run -p nanachi_meta --example parse -- examples/markdown_bold.nanachi
cargo run -p nanachi_meta --example parse -- examples/nested_braces.nanachi
```

## Generate winnow code

```sh
cargo run -p nanachi_generator --example codegen -- examples/simple.nanachi
cargo run -p nanachi_generator --example codegen -- examples/markdown_bold.nanachi
cargo run -p nanachi_generator --example codegen -- examples/nested_braces.nanachi
```

## Run all tests

```sh
cargo test --workspace
```
