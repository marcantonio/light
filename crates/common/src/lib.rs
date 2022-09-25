use serde::Serialize;

mod cli_args;
pub use cli_args::CliArgs;
mod macros;

// A Operator is an extra layer of abstraction between TokenType::Op() and the
// actual character. Convenient in Rust to help constrain matching.
#[derive(Debug, PartialEq, Clone, Copy, Serialize)]
pub enum Operator {
    Add,
    AddEq,
    And,
    Assign,
    BitAnd,
    BitOr,
    BitXor,
    Dec,
    Div,
    DivEq,
    Eq,
    Gt,
    GtEq,
    Inc,
    Lt,
    LtEq,
    Mul,
    MulEq,
    Not,
    NotEq,
    Or,
    Pow,
    RetType,
    Sub,
    SubEq,
}

impl std::fmt::Display for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Operator::*;
        let s = match self {
            Add => "+",
            AddEq => "+=",
            Assign => "=",
            And => "&&",
            BitAnd => "&",
            BitOr => "|",
            BitXor => "^",
            Dec => "--",
            Div => "/",
            DivEq => "/=",
            Eq => "==",
            Gt => ">",
            GtEq => ">=",
            Inc => "++",
            Lt => "<",
            LtEq => "<=",
            Mul => "*",
            MulEq => "*=",
            Not => "!",
            NotEq => "!=",
            Or => "||",
            Pow => "**",
            RetType => "->",
            Sub => "-",
            SubEq => "-=",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub enum Type {
    Int8,
    Int16,
    Int32,
    Int64,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Float,
    Double,
    Bool,
    Char,
    Void,
    Array(Box<Type>, usize),
    Comp(String),
}

impl Type {
    pub fn resolve_primitive(ty: &str) -> Self {
        use Type::*;

        match ty {
            "int8" => Int8,
            "int16" => Int16,
            "int32" => Int32,
            "int64" => Int64,
            "uint8" => UInt8,
            "uint16" => UInt16,
            "uint32" => UInt32,
            "uint64" => UInt64,
            "float" => Float,
            "double" => Double,
            "bool" => Bool,
            "char" => Char,
            "void" => Void,
            "int" => Int32,
            "uint" => UInt32,
            x => Comp(x.to_owned()),
        }
    }
}

impl Default for Type {
    fn default() -> Self {
        Self::Void
    }
}

impl Default for &Type {
    fn default() -> Self {
        &Type::Void
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = format!("{:?}", self).to_ascii_lowercase();
        write!(f, "{}", s)
    }
}

#[cfg(test)]
mod test {
    use crate::Type;

    #[test]
    fn test_resolve_primitive() {
        assert_eq!(Type::resolve_primitive("int32"), Type::Int32);
        assert_eq!(Type::resolve_primitive("Int32"), Type::Comp(String::from("Int32")));
    }
}
