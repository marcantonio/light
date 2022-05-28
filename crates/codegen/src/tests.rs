use inkwell::context::Context;
use inkwell::passes::PassManager;
use inkwell::values::{AnyValue, FunctionValue};

use lexer::Lexer;
use parser::Parser;
use type_checker::TypeChecker;

use super::*;

macro_rules! run_insta {
    ($prefix:expr, $tests:expr) => {
        insta::with_settings!({ snapshot_path => "tests/snapshots", prepend_module_to_snapshot => false }, {
            for test in $tests {
                let tokens = Lexer::new(test[1]).scan().unwrap();
                let mut ast = Parser::new(&tokens).parse().unwrap();
                TypeChecker::new().walk(&mut ast).unwrap();
                let context = Context::create();
                let builder = context.create_builder();
                let module = context.create_module("main_mod");
                let fpm = PassManager::create(&module);
                let mut codegen = Codegen::new(&context, &builder, &module, &fpm, 1, false);
                let res = code_to_string(codegen.walk(&ast));
                insta::assert_yaml_snapshot!(format!("{}_{}", $prefix, test[0]), (test[1], res));
            }
        })
    };
}

// This is how we deserialize FunctionValue to work with insta
fn code_to_string(code: Result<FunctionValue, String>) -> String {
    match code {
        Ok(code) => code.print_to_string().to_string(),
        Err(err) => err,
    }
}

#[test]
fn test_block() {
    let tests = [[
        "basic",
        r#"
fn main() {
    {
        10 + 2
        1.0
        5
    }
}
"#,
    ]];
    run_insta!("block", tests);
}

#[test]
fn test_for() {
    let tests = [[
        "basic",
        r#"
extern putchar(x: int) -> int
fn main() {
    let x: int = 3
    for y: int = 0; y < x; 1 {
        putchar(46)
    }
    putchar(10)
}
"#,
    ]];
    run_insta!("for", tests);
}

#[test]
fn test_cond() {
    let tests = [
        [
            "cond_success",
            r#"
fn plus_one(x: int) -> int { x + 1 }
fn main() {
    if plus_one(6) == 7 {
        7
    }
}
"#,
        ],
        [
            "cond_fail",
            r#"
fn plus_one(x: int) -> int { x + 1 }
fn main() {
    if plus_one(6) == 5 {
        7
    }
}
"#,
        ],
        [
            "if_else_cond_success",
            r#"
fn plus_one(x: int) -> int { x + 1 }
fn main() {
    if plus_one(6) == 7 {
        7
    } else {
        1
    }
}
"#,
        ],
        [
            "if_else_cond_fail",
            r#"
fn plus_one(x: int) -> int { x + 1 }
fn main() {
    if plus_one(6) == 8 {
        7
    } else {
        1
    }
}
"#,
        ],
        [
            "if_else_if_else_cond_success",
            r#"
fn plus_one(x: int) -> int { x + 1 }
fn main() {
    if plus_one(6) == 7 {
        7
    } else if plus_one(1) == 2 {
        2
    } else {
        1
    }
}
"#,
        ],
        [
            "if_else_if_else_cond_2_success",
            r#"
fn plus_one(x: int) -> int { x + 1 }
fn main() {
    if plus_one(6) == 5 {
        7
    } else if plus_one(1) == 2 {
        2
    } else {
        1
    }
}
"#,
        ],
        [
            "if_else_if_else_conds_fail",
            r#"
fn plus_one(x: int) -> int { x + 1 }
fn main() {
    if plus_one(6) == 5 {
        7
    } else if plus_one(1) == 3 {
        2
    } else {
        1
    }
}
"#,
        ],
        [
            "if_else_if_cond_success",
            r#"
fn plus_one(x: int) -> int { x + 1 }
fn main() {
    if plus_one(6) == 7 {
        7
    } else if plus_one(1) == 1 {
        2
    }
}
"#,
        ],
        [
            "if_else_if_cond_2_success",
            r#"
fn plus_one(x: int) -> int { x + 1 }
fn main() {
    if plus_one(6) == 5 {
        7
    } else if plus_one(1) == 2 {
        2
    }
}
"#,
        ],
        [
            "true_lit",
            r#"
fn main() {
    if true {
        7
    } else {
        2
    }
}
"#,
        ],
        [
            "false_lit",
            r#"
fn main() {
    if false {
        7
    } else {
        2
    }
}
"#,
        ],
    ];
    run_insta!("cond", tests);
}

#[test]
fn test_let() {
    let tests = [
        [
            "basic",
            r#"
        fn main() {
    let x: int = 1
}
"#,
        ],
        [
            "float_default",
            r#"
fn main() {
    let x: float
}
"#,
        ],
        [
            "bool",
            r#"
fn main() {
    let x: bool
}
"#,
        ],
    ];
    run_insta!("let", tests);
}

#[test]
fn test_scope() {
    let tests = [
        [
            "basic_shadowing",
            r#"
fn foo(a: int) -> int {
    let b: int = 1
    {
        let b: bool = false
    }
    b
}
fn main() { foo(3) }
"#,
        ],
        [
            "nested_shadowing",
            r#"
fn foo(a: int) -> int {
    let b: int = 1
    {
        let b: bool = false
        let a: float = {
            let b: float = 1.0
            b
        }
    }
    b
}
fn main() { foo(3) }
"#,
        ],
        [
            "for",
            r#"
fn foo() {
    let x: float = 1.0
    for x: int = 1; x < 10; 1 {
        x
    }
    x
}
fn main() { foo() }
"#,
        ],
        [
            "if",
            r#"
fn foo() {
    let x: float = 1.0
    if x < 2.0 {
        let y: int = 2
        x
    }
    x
}
fn main() { foo() }
"#,
        ],
        [
            "if_else",
            r#"
fn foo() {
    let x: float = 1.0
    if x < 2.0 {
        let y: int = 2
        y
    } else {
        let y: int = -2
        y
    }
}
fn main() { foo() }
"#,
        ],
    ];
    run_insta!("scope", tests);
}

#[test]
fn test_unop() {
    let tests = [[
        "basic",
        r#"
fn test(x: int) -> int {
    x + 1
}

fn main() -> int {
    test(-40)
}
"#,
    ]];
    run_insta!("unop", tests);
}