/// PBL Project Manager — Handles Cargo project generation and dependency management.

use crate::ast::*;
use crate::errors::{PblError, PblResult};
use std::fs;
use std::path::{Path, PathBuf};

/// Represents the generated Rust/Cargo project.
pub struct ProjectManager {
    /// Output directory for the generated project.
    pub output_dir: PathBuf,
    /// Crate dependencies collected from the PBL source.
    pub dependencies: Vec<CrateDep>,
    /// Whether to use no_std mode.
    pub no_std: bool,
    /// Project name (derived from the source file).
    pub name: String,
}

/// A resolved crate dependency for Cargo.toml.
#[derive(Debug, Clone)]
pub struct CrateDep {
    pub name: String,
    pub version: String,
    pub features: Vec<String>,
    pub source: CrateSource,
}

impl ProjectManager {
    pub fn new(source_file: &str, output_dir: &Path) -> Self {
        let name = Path::new(source_file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("pbl_project")
            .to_string();

        Self {
            output_dir: output_dir.to_path_buf(),
            dependencies: Vec::new(),
            no_std: false,
            name,
        }
    }

    /// Extract dependencies and project settings from the program.
    pub fn collect_from_program(&mut self, program: &Program) {
        for item in &program.items {
            match item {
                Item::CrateDecl(c) => {
                    self.dependencies.push(CrateDep {
                        name: c.name.clone(),
                        version: c.version.clone(),
                        features: c.features.clone(),
                        source: c.source.clone(),
                    });
                }
                Item::ProjectConfig(config) => {
                    for setting in &config.settings {
                        if let ProjectSetting::Mode(mode) = setting {
                            if mode == "no_std" {
                                self.no_std = true;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Generate the Cargo project structure.
    pub fn generate_project(&self, rust_source: &str) -> PblResult<()> {
        // Create output directories
        let src_dir = self.output_dir.join("src");
        fs::create_dir_all(&src_dir).map_err(|e| {
            PblError::project(format!(
                "failed to create output directory '{}': {}",
                src_dir.display(),
                e
            ))
        })?;

        // Write Cargo.toml
        self.write_cargo_toml()?;

        // Write main.rs
        let main_path = src_dir.join("main.rs");
        fs::write(&main_path, rust_source).map_err(|e| {
            PblError::project(format!(
                "failed to write '{}': {}",
                main_path.display(),
                e
            ))
        })?;

        Ok(())
    }

    fn write_cargo_toml(&self) -> PblResult<()> {
        let mut toml = String::new();

        toml.push_str(&format!(
            "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
            self.name
        ));

        if !self.dependencies.is_empty() {
            toml.push_str("\n[dependencies]\n");

            for dep in &self.dependencies {
                match &dep.source {
                    CrateSource::CratesIo => {
                        if dep.features.is_empty() {
                            toml.push_str(&format!("{} = \"{}\"\n", dep.name, dep.version));
                        } else {
                            let features: Vec<String> =
                                dep.features.iter().map(|f| format!("\"{}\"", f)).collect();
                            toml.push_str(&format!(
                                "{} = {{ version = \"{}\", features = [{}] }}\n",
                                dep.name,
                                dep.version,
                                features.join(", ")
                            ));
                        }
                    }
                    CrateSource::Path(path) => {
                        toml.push_str(&format!("{} = {{ path = \"{}\" }}\n", dep.name, path));
                    }
                    CrateSource::Git(url) => {
                        toml.push_str(&format!("{} = {{ git = \"{}\" }}\n", dep.name, url));
                    }
                }
            }
        }

        // Write profile settings for no_std
        if self.no_std {
            toml.push_str("\n[profile.dev]\npanic = \"abort\"\n");
            toml.push_str("\n[profile.release]\npanic = \"abort\"\n");
        }

        let cargo_path = self.output_dir.join("Cargo.toml");
        fs::write(&cargo_path, &toml).map_err(|e| {
            PblError::project(format!(
                "failed to write '{}': {}",
                cargo_path.display(),
                e
            ))
        })?;

        Ok(())
    }

    /// Run cargo build on the generated project.
    pub fn cargo_build(&self, release: bool, target: Option<&str>) -> PblResult<std::process::Output> {
        let mut cmd = std::process::Command::new("cargo");
        cmd.arg("build");
        cmd.current_dir(&self.output_dir);

        if release {
            cmd.arg("--release");
        }

        if let Some(t) = target {
            cmd.args(["--target", t]);
        }

        let output = cmd.output().map_err(|e| {
            PblError::project(format!("failed to run cargo build: {}", e))
        })?;

        Ok(output)
    }

    /// Run cargo check on the generated project.
    pub fn cargo_check(&self) -> PblResult<std::process::Output> {
        let mut cmd = std::process::Command::new("cargo");
        cmd.arg("check");
        cmd.current_dir(&self.output_dir);

        let output = cmd.output().map_err(|e| {
            PblError::project(format!("failed to run cargo check: {}", e))
        })?;

        Ok(output)
    }

    /// Run the generated project.
    pub fn cargo_run(&self) -> PblResult<std::process::Output> {
        let mut cmd = std::process::Command::new("cargo");
        cmd.arg("run");
        cmd.current_dir(&self.output_dir);

        let output = cmd.output().map_err(|e| {
            PblError::project(format!("failed to run cargo run: {}", e))
        })?;

        Ok(output)
    }

    /// Clean the generated project.
    pub fn clean(&self) -> PblResult<()> {
        if self.output_dir.exists() {
            fs::remove_dir_all(&self.output_dir).map_err(|e| {
                PblError::project(format!(
                    "failed to clean '{}': {}",
                    self.output_dir.display(),
                    e
                ))
            })?;
        }
        Ok(())
    }

    /// Get the path to the built executable.
    pub fn executable_path(&self, release: bool, target: Option<&str>) -> PathBuf {
        let mut path = self.output_dir.join("target");
        if let Some(t) = target {
            path = path.join(t);
        }
        if release {
            path = path.join("release");
        } else {
            path = path.join("debug");
        }
        path.join(&self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use std::env;

    #[test]
    fn test_project_name_from_file() {
        let pm = ProjectManager::new("hello.pbl", Path::new("/tmp/pbl_test"));
        assert_eq!(pm.name, "hello");
    }

    #[test]
    fn test_collect_dependencies() {
        let program = Program {
            items: vec![
                Item::CrateDecl(CrateDecl {
                    name: "serde".to_string(),
                    version: "1".to_string(),
                    features: vec!["derive".to_string()],
                    source: CrateSource::CratesIo,
                    span: Span::unknown(),
                }),
            ],
        };

        let mut pm = ProjectManager::new("test.pbl", Path::new("/tmp/pbl_test"));
        pm.collect_from_program(&program);
        assert_eq!(pm.dependencies.len(), 1);
        assert_eq!(pm.dependencies[0].name, "serde");
    }

    #[test]
    fn test_no_std_detection() {
        let program = Program {
            items: vec![
                Item::ProjectConfig(ProjectConfig {
                    settings: vec![ProjectSetting::Mode("no_std".to_string())],
                    span: Span::unknown(),
                }),
            ],
        };

        let mut pm = ProjectManager::new("test.pbl", Path::new("/tmp/pbl_test"));
        pm.collect_from_program(&program);
        assert!(pm.no_std);
    }
}
