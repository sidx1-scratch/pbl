/// PBL Compiler — Orchestrates the compilation pipeline.

use crate::ast::{Item, Program, ProjectSetting};
use crate::codegen::{self, CodeGenConfig};
use crate::errors::{PblError, PblResult};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::project::ProjectManager;
use crate::semantic;
use std::path::{Path, PathBuf};

/// Options for the compilation pipeline.
pub struct CompileOptions {
    pub source_file: String,
    pub output_dir: Option<PathBuf>,
    pub output_file: Option<PathBuf>,
    pub release: bool,
    pub target: Option<String>,
    pub keep_generated: bool,
    pub verbose: bool,
    pub no_std: bool,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            source_file: String::new(),
            output_dir: None,
            output_file: None,
            release: false,
            target: None,
            keep_generated: false,
            verbose: false,
            no_std: false,
        }
    }
}

/// Result of a compilation.
pub struct CompileResult {
    pub rust_source: String,
    pub program: Program,
    pub output_dir: PathBuf,
}

/// Full compilation pipeline.
pub struct Compiler {
    options: CompileOptions,
}

impl Compiler {
    pub fn new(options: CompileOptions) -> Self {
        Self { options }
    }

    /// Read, parse, analyze, and generate Rust source (no Cargo build).
    pub fn compile(&self) -> PblResult<CompileResult> {
        let source = self.read_source()?;
        let program = self.parse(&source)?;
        self.analyze(&program)?;
        let rust_source = self.generate(&program)?;

        let output_dir = self
            .options
            .output_dir
            .clone()
            .unwrap_or_else(|| {
                let stem = Path::new(&self.options.source_file)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("pbl_output");
                PathBuf::from(format!(".pbl_build/{}", stem))
            });

        Ok(CompileResult {
            rust_source,
            program,
            output_dir,
        })
    }

    /// Full compile + cargo build pipeline.
    pub fn build(&self) -> PblResult<PathBuf> {
        let result = self.compile()?;

        if self.options.verbose {
            eprintln!("[pbl] generated Rust source ({} bytes)", result.rust_source.len());
        }

        // Set up the project
        let mut pm = ProjectManager::new(&self.options.source_file, &result.output_dir);
        pm.collect_from_program(&result.program);

        if self.options.no_std {
            pm.no_std = true;
        }

        pm.generate_project(&result.rust_source)?;

        if self.options.verbose {
            eprintln!("[pbl] generated Cargo project at {}", result.output_dir.display());
        }

        // Build with Cargo
        let output = pm.cargo_build(
            self.options.release,
            self.options.target.as_deref(),
        )?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PblError::project(format!(
                "Rust compilation failed:\n{}",
                stderr
            )));
        }

        if self.options.verbose {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if !stdout.is_empty() {
                eprintln!("[pbl] cargo: {}", stdout.trim());
            }
        }

        // Return path to executable
        let exe_path = if let Some(ref out) = self.options.output_file {
            // Copy executable to desired location
            let built = pm.executable_path(
                self.options.release,
                self.options.target.as_deref(),
            );
            if built.exists() {
                std::fs::copy(&built, out).map_err(|e| {
                    PblError::project(format!("failed to copy executable: {}", e))
                })?;
            }
            out.clone()
        } else {
            pm.executable_path(
                self.options.release,
                self.options.target.as_deref(),
            )
        };

        Ok(exe_path)
    }

    /// Compile and run the program.
    pub fn run(&self) -> PblResult<i32> {
        let result = self.compile()?;

        let mut pm = ProjectManager::new(&self.options.source_file, &result.output_dir);
        pm.collect_from_program(&result.program);

        if self.options.no_std {
            pm.no_std = true;
        }

        pm.generate_project(&result.rust_source)?;

        let output = pm.cargo_run()?;

        // Print stdout and stderr from the program
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !stdout.is_empty() {
            print!("{}", stdout);
        }

        // Filter cargo noise from stderr but preserve program errors
        if !stderr.is_empty() {
            for line in stderr.lines() {
                if !line.trim_start().starts_with("Compiling")
                    && !line.trim_start().starts_with("Finished")
                    && !line.trim_start().starts_with("Running")
                    && !line.trim_start().starts_with("Downloading")
                    && !line.trim_start().starts_with("Downloaded")
                {
                    eprintln!("{}", line);
                }
            }
        }

        Ok(output.status.code().unwrap_or(1))
    }

    /// Parse and check only (no code generation or build).
    pub fn check(&self) -> PblResult<()> {
        let source = self.read_source()?;
        let program = self.parse(&source)?;
        self.analyze(&program)?;

        // Also generate and cargo check for full validation
        let rust_source = self.generate(&program)?;
        let output_dir = self
            .options
            .output_dir
            .clone()
            .unwrap_or_else(|| {
                let stem = Path::new(&self.options.source_file)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("pbl_output");
                PathBuf::from(format!(".pbl_build/{}", stem))
            });

        let mut pm = ProjectManager::new(&self.options.source_file, &output_dir);
        pm.collect_from_program(&program);
        pm.generate_project(&rust_source)?;

        let output = pm.cargo_check()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PblError::project(format!(
                "Rust check failed:\n{}",
                stderr
            )));
        }

        Ok(())
    }

    /// Generate Rust source only (no Cargo build).
    pub fn compile_only(&self) -> PblResult<CompileResult> {
        let result = self.compile()?;

        let mut pm = ProjectManager::new(&self.options.source_file, &result.output_dir);
        pm.collect_from_program(&result.program);
        pm.generate_project(&result.rust_source)?;

        Ok(result)
    }

    // -----------------------------------------------------------------------
    // Pipeline stages
    // -----------------------------------------------------------------------

    fn read_source(&self) -> PblResult<String> {
        std::fs::read_to_string(&self.options.source_file).map_err(|e| {
            PblError::io(format!(
                "failed to read '{}': {}",
                self.options.source_file, e
            ))
        })
    }

    fn parse(&self, source: &str) -> PblResult<Program> {
        let mut lexer = Lexer::new(source, &self.options.source_file);
        let tokens = lexer.tokenize()?;

        if self.options.verbose {
            eprintln!("[pbl] lexed {} tokens", tokens.len());
        }

        let mut parser = Parser::new(tokens, &self.options.source_file);
        let program = parser.parse()?;

        if self.options.verbose {
            eprintln!("[pbl] parsed {} items", program.items.len());
        }

        Ok(program)
    }

    fn analyze(&self, program: &Program) -> PblResult<()> {
        let errors = semantic::analyze(program);
        if !errors.is_empty() {
            // Report all errors
            for error in &errors {
                eprintln!("{}", error);
            }
            return Err(errors.into_iter().next().unwrap());
        }
        Ok(())
    }

    fn generate(&self, program: &Program) -> PblResult<String> {
        let config = CodeGenConfig {
            no_std: self.options.no_std || self.program_has_no_std(program),
        };
        codegen::generate(program, &config)
    }

    fn program_has_no_std(&self, program: &Program) -> bool {
        program.items.iter().any(|item| {
            if let Item::ProjectConfig(config) = item {
                config.settings.iter().any(|s| {
                    matches!(s, ProjectSetting::Mode(m) if m == "no_std")
                })
            } else {
                false
            }
        })
    }
}

/// Clean build artifacts for a source file.
pub fn clean(source_file: Option<&str>) -> PblResult<()> {
    let dir = if let Some(file) = source_file {
        let stem = Path::new(file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("pbl_output");
        PathBuf::from(format!(".pbl_build/{}", stem))
    } else {
        PathBuf::from(".pbl_build")
    };

    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| {
            PblError::project(format!("failed to clean '{}': {}", dir.display(), e))
        })?;
    }

    Ok(())
}
