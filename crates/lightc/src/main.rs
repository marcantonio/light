use clap::Parser as Clap;
use inkwell::{
    context::Context,
    module::Module,
    passes::PassManager,
    targets::{InitializationConfig, Target, TargetMachine},
    OptimizationLevel,
};
use std::fs;
use std::path::PathBuf;
use std::process::{exit, Command};

use codegen::Codegen;
use lexer::Lexer;
use parser::Parser;
use type_checker::TypeChecker;

mod jit_externs;

fn main() {
    let args = Args::parse();
    let source = fs::read_to_string(args.file).expect("Error opening file");

    // Lexer
    let tokens = Lexer::new(&source).scan().unwrap_or_else(|e| {
        eprintln!("{}", e);
        exit(1);
    });

    if args.tokens {
        println!("Tokens:");
        tokens.iter().for_each(|t| println!("{:?}", t));
        println!();
    }

    // Parser
    let parser = Parser::new(&tokens);
    let mut ast = parser.parse().unwrap_or_else(|e| {
        eprintln!("{}", e);
        exit(1);
    });

    if args.ast_pre {
        println!("AST (pre):");
        for node in ast.nodes() {
            println!("{}", node);
        }
        println!();
    }

    // Type Checker
    let mut type_checker = TypeChecker::new();
    type_checker.walk(&mut ast).expect("Type checking error");

    if args.ast {
        println!("AST:");
        for node in ast.nodes() {
            println!("{}", node);
        }
        println!();
    }

    // Codegen
    let context = Context::create();
    let builder = context.create_builder();
    let module = context.create_module("light_main");
    set_target_machine(&module);
    let fpm = PassManager::create(&module);
    let mut codegen = Codegen::new(
        &context,
        &builder,
        &module,
        &fpm,
        args.opt_level,
        args.no_verify,
    );
    codegen.walk(&ast).expect("Compiler error");

    let tmp_file = tempfile::Builder::new()
        .prefix("lightc-")
        .suffix(".ll")
        .tempfile()
        .expect("Error creating temp file")
        .into_temp_path();

    module
        .print_to_file(&tmp_file)
        .expect("Error writing tmp IR");

    if args.ir {
        println!("IR:");
        println!("{}", module.print_to_string().to_string());
    }

    if args.jit {
        run_jit(&module);
    } else {
        Command::new("clang")
            .arg(&tmp_file)
            .arg("-lm")
            .spawn()
            .expect("Error compiling")
            .wait()
            .expect("Error waiting on clang");
    }
}

// Optimizes for host CPU
// TODO: Make more generic
fn set_target_machine(module: &Module) {
    Target::initialize_x86(&InitializationConfig::default());
    let triple = TargetMachine::get_default_triple();
    let target = Target::from_triple(&triple).expect("Target error");
    let target_machine = target
        .create_target_machine(
            &triple,
            &TargetMachine::get_host_cpu_name().to_string(),
            &TargetMachine::get_host_cpu_features().to_string(),
            OptimizationLevel::Default,
            inkwell::targets::RelocMode::Default,
            inkwell::targets::CodeModel::Default,
        )
        .expect("Target machine error");

    module.set_data_layout(&target_machine.get_target_data().get_data_layout());
    module.set_triple(&triple);
}

fn run_jit(module: &Module) {
    jit_externs::load();

    let ee = module
        .create_jit_execution_engine(OptimizationLevel::None)
        .unwrap();

    let f = unsafe { ee.get_function::<unsafe extern "C" fn() -> i64>("main") };
    match f {
        Ok(f) => unsafe {
            let ret = f.call();
            eprintln!("main() return value: {:?}", ret);
        },
        Err(e) => {
            eprintln!("Execution error: {}", e);
        }
    };
}

#[derive(Clap, Debug)]
struct Args {
    /// Display lexeme tokens
    #[clap(short, long, parse(from_flag))]
    tokens: bool,

    /// Display AST pre type checker
    #[clap(short = 'A', long, parse(from_flag))]
    ast_pre: bool,

    /// Display AST
    #[clap(short, long, parse(from_flag))]
    ast: bool,

    /// Display IR
    #[clap(short, long, parse(from_flag))]
    ir: bool,

    /// Run jit rather than outputting a binary
    #[clap(short, long, parse(from_flag))]
    jit: bool,

    /// Output to <file>
    #[clap(short, long, value_name="file", default_value_t = String::from("./a.out"))]
    output: String,

    /// Optimization level
    #[clap(short = 'O', long, value_name="level", default_value_t = 1, parse(try_from_str=valid_opt_level))]
    opt_level: usize,

    /// Disable LLVM function validation (useful for debugging)
    #[clap(short, long, parse(from_flag))]
    no_verify: bool,

    /// Input file
    #[clap(parse(from_os_str))]
    file: PathBuf,
}

fn valid_opt_level(s: &str) -> Result<usize, String> {
    let opt_level = s
        .parse()
        .map_err(|_| format!("`{}` isn't an optimization level", s))?;

    if (0..=1).contains(&opt_level) {
        Ok(opt_level)
    } else {
        Err("Must be one of: 0 (none), 1 (basic)".to_string())
    }
}