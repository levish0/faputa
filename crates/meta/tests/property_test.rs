use faputa_meta::{compile, parser, validator};
use proptest::prelude::*;

fn escape_string(chars: &[char]) -> String {
    let mut out = String::from("\"");
    for ch in chars {
        match ch {
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            c => out.push(*c),
        }
    }
    out.push('"');
    out
}

fn string_literal_strategy() -> BoxedStrategy<String> {
    prop::collection::vec(
        prop_oneof![
            Just('a'),
            Just('b'),
            Just('c'),
            Just(' '),
            Just('\n'),
            Just('\t'),
            Just('\\'),
            Just('"'),
            Just('가'),
            Just('힣'),
        ],
        0..=6,
    )
    .prop_map(|chars| escape_string(&chars))
    .boxed()
}

fn char_range_strategy() -> BoxedStrategy<String> {
    prop_oneof![
        Just("'a'..'z'".to_string()),
        Just("'A'..'Z'".to_string()),
        Just("'0'..'9'".to_string()),
        Just("'가'..'힣'".to_string()),
        Just(r"'\n'..'\r'".to_string()),
    ]
    .boxed()
}

fn repeat_suffix_strategy() -> BoxedStrategy<String> {
    prop_oneof![
        Just("+".to_string()),
        Just("*".to_string()),
        Just("?".to_string()),
        (0u32..=4).prop_map(|n| format!("{{{n}}}")),
        (0u32..=4).prop_map(|n| format!("{{{n},}}")),
        (0u32..=4).prop_map(|n| format!("{{,{n}}}")),
        (0u32..=3, 0u32..=3).prop_map(|(a, b)| {
            let (start, end) = if a <= b { (a, b) } else { (b, a) };
            format!("{{{start},{end}}}")
        }),
    ]
    .boxed()
}

fn expr_strategy() -> BoxedStrategy<String> {
    let leaf = prop_oneof![
        string_literal_strategy(),
        char_range_strategy(),
        Just("atom".to_string()),
        Just("helper".to_string()),
        Just("SOI".to_string()),
        Just("EOI".to_string()),
        Just("ANY".to_string()),
        Just("LINE_START".to_string()),
        Just("LINE_END".to_string()),
    ]
    .boxed();

    leaf.prop_recursive(4, 96, 8, |inner| {
        prop_oneof![
            (inner.clone(), repeat_suffix_strategy())
                .prop_map(|(expr, suffix)| format!("{expr}{suffix}")),
            prop::collection::vec(inner.clone(), 2..=3).prop_map(|parts| parts.join(" ")),
            prop::collection::vec(inner.clone(), 2..=3).prop_map(|parts| parts.join(" | ")),
            inner.clone().prop_map(|expr| format!("({expr})")),
            inner.clone().prop_map(|expr| format!("&({expr})")),
            inner.clone().prop_map(|expr| format!("!({expr})")),
            inner
                .clone()
                .prop_map(|expr| format!("with f0 {{ {expr} }}")),
            (1u32..=3, inner.clone())
                .prop_map(|(amount, expr)| format!("with c0 += {amount} {{ {expr} }}")),
            inner
                .clone()
                .prop_map(|expr| format!("when f0 {{ {expr} }}")),
            inner
                .clone()
                .prop_map(|expr| format!("when !f0 {{ {expr} }}")),
            (0u32..=3, inner.clone())
                .prop_map(|(value, expr)| format!("when c0 > {value} {{ {expr} }}")),
            (1u32..=8, inner)
                .prop_map(|(limit, expr)| format!("depth_limit({limit}) {{ {expr} }}")),
        ]
        .boxed()
    })
    .boxed()
}

fn statements_strategy() -> BoxedStrategy<Vec<String>> {
    prop::collection::vec(
        prop_oneof![
            Just("guard f0".to_string()),
            Just("guard !f0".to_string()),
            Just("guard c0 > 0".to_string()),
            Just("guard SOI".to_string()),
            Just("guard LINE_START".to_string()),
            Just("inc c0".to_string()),
        ],
        0..=3,
    )
    .boxed()
}

fn build_valid_source(statements: &[String], expr: &str) -> String {
    let mut source = String::from(
        "let flag f0\n\
let counter c0\n\
atom = { \"a\" | ANY | 'a'..'z' }\n\
helper = { atom+ }\n\
entry = {\n",
    );

    for statement in statements {
        source.push_str("    ");
        source.push_str(statement);
        source.push('\n');
    }

    source.push_str("    ");
    source.push_str(expr);
    source.push_str("\n}\n");
    source
}

fn grammarish_input_strategy() -> BoxedStrategy<String> {
    prop::collection::vec(
        prop_oneof![
            Just('a'),
            Just('b'),
            Just('c'),
            Just('A'),
            Just('Z'),
            Just('0'),
            Just('9'),
            Just('_'),
            Just(' '),
            Just('\n'),
            Just('\t'),
            Just('{'),
            Just('}'),
            Just('('),
            Just(')'),
            Just('|'),
            Just('&'),
            Just('!'),
            Just('?'),
            Just('+'),
            Just('*'),
            Just('='),
            Just(','),
            Just('.'),
            Just(':'),
            Just('<'),
            Just('>'),
            Just('/'),
            Just('\\'),
            Just('"'),
            Just('\''),
            Just('@'),
            Just('$'),
        ],
        0..=256,
    )
    .prop_map(|chars| chars.into_iter().collect())
    .boxed()
}

fn arbitrary_unicode_input_strategy() -> BoxedStrategy<String> {
    prop::collection::vec(any::<char>(), 0..=128)
        .prop_map(|chars| chars.into_iter().collect())
        .boxed()
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        .. ProptestConfig::default()
    })]

    #[test]
    fn generated_valid_grammars_compile(
        statements in statements_strategy(),
        expr in expr_strategy(),
    ) {
        let source = build_valid_source(&statements, &expr);

        let grammar = parser::parse(&source)
            .unwrap_or_else(|err| panic!("generated source should parse:\n{source}\nerror: {err}"));
        validator::validate(&grammar)
            .unwrap_or_else(|errors| panic!("generated source should validate:\n{source}\nerrors: {errors:?}"));
        compile(&source)
            .unwrap_or_else(|err| panic!("generated source should compile:\n{source}\nerror: {err:?}"));
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 128,
        .. ProptestConfig::default()
    })]

    #[test]
    fn grammarish_random_inputs_do_not_panic(input in grammarish_input_strategy()) {
        let _ = parser::parse(&input);
        let _ = compile(&input);
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        .. ProptestConfig::default()
    })]

    #[test]
    fn arbitrary_unicode_inputs_do_not_panic(input in arbitrary_unicode_input_strategy()) {
        let _ = parser::parse(&input);
        let _ = compile(&input);
    }
}
