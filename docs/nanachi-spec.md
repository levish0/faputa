# .nanachi DSL Specification (Draft)

nanachi의 자체 문법 기술 언어. winnow 기반 Rust 파서 코드를 생성하기 위한 DSL.

## 설계 원칙

- **자체 문법**: Rust도 pest도 winnow도 아닌 nanachi 전용 DSL
- **사람이 읽기 좋게**: `+ * ?` 같은 보편적 표기, whitespace sequence
- **codegen이 최적화**: 사용자는 의도를 표현하고, optimizer/generator가 효율적 winnow 코드 생성
- **stateful 파싱 일급 지원**: flag, counter, guard, with 등으로 상태를 선언적으로 기술

## State 선언

개별 `let` 선언. 타입은 종류(flag/counter)에서 결정됨.

```nanachi
let flag inside_bold
let flag inside_italic
let flag inside_header

let counter section_counter
let counter footnote_counter
let counter trim_brace_depth
```

향후 확장 가능:

```nanachi
let stack indent_levels                           // push/pop 스택
let mode block_context: normal | raw | code       // enum 상태
```

## 규칙 정의

```nanachi
rule_name = { expression }
```

### 기본 표현식

| 문법           | 의미               | 생성되는 winnow 코드       |
|--------------|------------------|----------------------|
| `"literal"`  | 문자열 매칭           | `literal("literal")` |
| `'a'..'z'`   | 문자 범위            | `one_of('a'..='z')`  |
| `a b c`      | 시퀀스 (whitespace) | `(a, b, c)`          |
| `a \| b`     | 선택               | `alt((a, b))`        |
| `p+`         | 1회 이상 반복         | `repeat(1.., p)`     |
| `p*`         | 0회 이상 반복         | `repeat(0.., p)`     |
| `p?`         | 선택적              | `opt(p)`             |
| `p{n}`       | 정확히 n회           | `repeat(n, p)`       |
| `p{n,m}`     | n~m회 반복          | `repeat(n..=m, p)`   |
| `p{n,}`      | n회 이상            | `repeat(n.., p)`     |
| `p{,m}`      | m회 이하            | `repeat(..=m, p)`    |
| `&p`         | 긍정 lookahead     | `peek(p)`            |
| `!p`         | 부정 lookahead     | `not(p)`             |
| `(a b \| c)` | 그룹핑              | 괄호로 우선순위 제어          |

### 연산자 우선순위 (높은 순)

1. `+ * ? {n,m}` — 후위 반복
2. `& !` — 전위 lookahead
3. 시퀀스 (whitespace)
4. `|` — 선택

### 내장 Predicate

| 이름           | 의미       | winnow 매핑               |
|--------------|----------|-------------------------|
| `SOI`        | 입력 시작    | offset == 0 체크          |
| `EOI`        | 입력 끝     | `eof`                   |
| `ANY`        | 아무 토큰 1개 | `any`                   |
| `LINE_START` | 줄 시작 위치  | `LocatingSlice`에서 위치 조회 |
| `LINE_END`   | 줄 끝 위치   | `peek(newline \| eof)`  |

## Stateful 구문

### guard — 전제조건

```nanachi
guard <condition>
```

조건이 거짓이면 rule fail (backtrack).

```nanachi
bold = {
    guard !inside_bold
    "**" inline+ "**"
}

header = {
    guard LINE_START
    guard !inside_header
    "#"{1,6} " " inline+
}
```

### with — 스코프 상태 변경

블록 진입 시 상태 변경, 블록 종료(성공/실패 모두) 시 원래 값으로 복원.

```nanachi
// flag: 진입 시 true, 퇴장 시 false
with inside_bold {
    "**" inline+ "**"
}

// counter: 진입 시 +1, 퇴장 시 -1
with trim_brace_depth += 1 {
    block_content*
}
```

### emit — 카운터 캡처 및 증가

현재 카운터 값을 파싱 결과에 포함하고 +1 증가.

```nanachi
header = {
    guard LINE_START
    emit section_counter
    with inside_header {
        "#"{1,6} " " inline+
    }
}
```

### when — 조건부 서브표현식

조건이 true일 때만 내부 표현식 실행.

```nanachi
newline = {
    guard !inside_header
    when trim_brace_depth > 0 {
        !(WHITESPACE+ &"}}}")
    }
    "\n"
}
```

### depth_limit — 재귀 깊이 제한

내장 combinator. 진입 시 depth++, 퇴장 시 depth--, N 초과 시 fail.

```nanachi
nested_element = {
    depth_limit(64) {
        bold | italic | link | raw_block
    }
}
```

## 예시: sevenmark bold를 .nanachi로

### Before (수동 Rust + winnow)

```rust
pub fn markdown_bold_parser(parser_input: &mut ParserInput) -> Result<Element> {
    if parser_input.state.inside_bold {
        return Err(winnow::error::ContextError::new());
    }
    parser_input.state.inside_bold = true;
    let result = ("**", repeat(1.., inline_parser), "**").parse_next(parser_input);
    parser_input.state.inside_bold = false;
    result.map(|(_, content, _)| Element::Bold(content))
}
```

### After (.nanachi)

```nanachi
let flag inside_bold

bold = {
    guard !inside_bold
    with inside_bold {
        "**" inline+ "**"
    }
}
```

## 예시: 중첩 brace 블록

```nanachi
let counter brace_depth

raw_block = {
    "{{{" with brace_depth += 1 { raw_content* } "}}}"
}

raw_content = {
    raw_block
    | !("}}}" ) ANY
}
```

## 코드 생성 파이프라인

```
.nanachi 파일
  → nanachi_meta: winnow로 .nanachi 파싱 → AST
  → nanachi_meta optimizer: prefix factorization, literal 병합, alt→dispatch 변환 등
  → OptimizedAST
  → nanachi_generator: Rust + winnow 코드 emit
  → 사용자 프로젝트에서 build.rs 또는 #[derive(Parser)]로 통합
```

### 생성 코드의 입력 타입

```rust
type ParserInput<'i> = Stateful<LocatingSlice<&'i str>, ParseState>;

struct ParseState {
    // let flag → bool 필드
    inside_bold: bool,
    inside_italic: bool,
    // let counter → u32 필드
    section_counter: u32,
    // ...
}
```

## 미확정 사항

- [ ] 규칙 모디파이어 (silent, atomic 등) 필요 여부 및 문법
- [ ] 에러 메시지 커스터마이징 문법
- [ ] 연산자 우선순위 파싱 (Pratt parser) 지원 문법
- [ ] `let stack`, `let mode` 등 확장 state 종류의 구체적 문법
- [ ] 규칙 간 공유 state vs 규칙 로컬 state
- [ ] 주석 문법 (`//` vs `/* */`)
- [ ] import/모듈 시스템
