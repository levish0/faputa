#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    /// Rule name defined more than once.
    DuplicateRule { name: String },

    /// State variable declared more than once.
    DuplicateState { name: String },

    /// Rule references an undefined rule name.
    UndefinedRule { name: String, used_in: String },

    /// State variable used but never declared.
    UndefinedState { name: String, used_in: String },

    /// `guard !x` or `with x { }` used on a counter (expected flag).
    ExpectedFlag { name: String, used_in: String },

    /// `inc x` or `with x += n` used on a flag (expected counter).
    ExpectedCounter { name: String, used_in: String },

    /// A rule name shadows a built-in predicate.
    ShadowsBuiltin { name: String },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateRule { name } => {
                write!(f, "rule '{name}' is defined more than once")
            }
            Self::DuplicateState { name } => {
                write!(f, "state variable '{name}' is declared more than once")
            }
            Self::UndefinedRule { name, used_in } => {
                write!(
                    f,
                    "rule '{name}' is referenced in '{used_in}' but not defined"
                )
            }
            Self::UndefinedState { name, used_in } => {
                write!(
                    f,
                    "state variable '{name}' is used in '{used_in}' but not declared"
                )
            }
            Self::ExpectedFlag { name, used_in } => {
                write!(
                    f,
                    "'{name}' in '{used_in}' is a counter, but a flag is expected here"
                )
            }
            Self::ExpectedCounter { name, used_in } => {
                write!(
                    f,
                    "'{name}' in '{used_in}' is a flag, but a counter is expected here"
                )
            }
            Self::ShadowsBuiltin { name } => {
                write!(f, "rule '{name}' shadows a built-in predicate")
            }
        }
    }
}

impl std::error::Error for ValidationError {}
