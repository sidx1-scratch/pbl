/// PBL CLI — Command-line interface for the PBL compiler.

use crate::compiler::{self, CompileOptions, Compiler};
#[allow(unused_imports)]
use std::path::PathBuf;

const VERSION: &str = "0.4.0";

const HELP: &str = r#"PBL — Polar Bear Language Compiler v0.4.0

USAGE:
    pbl <command> [options] [file]

COMMANDS:
    run <file>          Compile and execute a PBL program
    build <file>        Build a native executable
    compile <file>      Generate Rust source (no final executable)
    check <file>        Parse, analyze, and check without building
    clean               Remove generated build artifacts
    version             Display the compiler version
    help                Show this help message

OPTIONS:
    --output <path>     Output file path
    --target <triple>   Compilation target (e.g., x86_64-unknown-none)
    --release           Build with optimizations
    --keep-generated    Keep the generated Rust project
    --verbose           Enable verbose output
    --no-std            Generate no_std compatible code

EXAMPLES:
    pbl run hello.pbl
    pbl build hello.pbl --release
    pbl build hello.pbl --target x86_64-unknown-none --no-std
    pbl compile hello.pbl --keep-generated
    pbl check hello.pbl
    pbl clean
"#;

pub fn run() -> i32 {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("{}", HELP);
        return 1;
    }

    let command = &args[1];

    match command.as_str() {
        "version" | "--version" | "-v" => {
            println!("PBL (Polar Bear Language) v{}", VERSION);
            0
        }
        "help" | "--help" | "-h" => {
            println!("{}", HELP);
            0
        }
        "clean" => {
            let source = args.get(2).map(|s| s.as_str());
            match compiler::clean(source) {
                Ok(()) => {
                    println!("Cleaned build artifacts.");
                    0
                }
                Err(e) => {
                    eprintln!("error: {}", e);
                    1
                }
            }
        }
        "run" | "build" | "compile" | "check" => {
            if args.len() < 3 {
                eprintln!("error: missing source file\n\nUsage: pbl {} <file>", command);
                return 1;
            }

            let options = match parse_options(&args[2..]) {
                Ok(opts) => opts,
                Err(e) => {
                    eprintln!("error: {}", e);
                    return 1;
                }
            };

            let compiler = Compiler::new(options);

            match command.as_str() {
                "run" => match compiler.run() {
                    Ok(code) => code,
                    Err(e) => {
                        eprintln!("{}", e);
                        1
                    }
                },
                "build" => match compiler.build() {
                    Ok(path) => {
                        println!("Built: {}", path.display());
                        0
                    }
                    Err(e) => {
                        eprintln!("{}", e);
                        1
                    }
                },
                "compile" => match compiler.compile_only() {
                    Ok(result) => {
                        println!("Generated Rust project at: {}", result.output_dir.display());
                        0
                    }
                    Err(e) => {
                        eprintln!("{}", e);
                        1
                    }
                },
                "check" => match compiler.check() {
                    Ok(()) => {
                        println!("Check passed ✓");
                        0
                    }
                    Err(e) => {
                        eprintln!("{}", e);
                        1
                    }
                },
                _ => unreachable!(),
            }
        }
        other => {
            eprintln!("error: unknown command '{}'\n\nRun 'pbl help' for usage.", other);
            1
        }
    }
}

fn parse_options(args: &[String]) -> Result<CompileOptions, String> {
    let mut options = CompileOptions::default();
    let mut i = 0;

    if args.is_empty() {
        return Err("missing source file".to_string());
    }

    // First positional argument is the source file
    options.source_file = args[0].clone();
    i += 1;

    while i < args.len() {
        match args[i].as_str() {
            "--output" | "-o" => {
                i += 1;
                if i >= args.len() {
                    return Err("--output requires a path".to_string());
                }
                options.output_file = Some(PathBuf::from(&args[i]));
            }
            "--target" => {
                i += 1;
                if i >= args.len() {
                    return Err("--target requires a target triple".to_string());
                }
                options.target = Some(args[i].clone());
            }
            "--release" => {
                options.release = true;
            }
            "--keep-generated" => {
                options.keep_generated = true;
            }
            "--verbose" | "-V" => {
                options.verbose = true;
            }
            "--no-std" => {
                options.no_std = true;
            }
            other => {
                return Err(format!("unknown option '{}'", other));
            }
        }
        i += 1;
    }

    Ok(options)
}
