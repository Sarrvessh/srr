use std::path::Path;
use std::time::Instant;
use std::process::Command;

use crate::error::SrrResult;
use crate::types::VerificationResult;

pub struct VerificationEngine;

impl VerificationEngine {
    fn run_command(tool: &str, program: &str, args: &[&str], path: &Path) -> VerificationResult {
        let start = Instant::now();
        let output = Command::new(program)
            .args(args)
            .current_dir(path)
            .output();
        let duration_ms = start.elapsed().as_millis() as u64;
        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                let combined = if stderr.is_empty() {
                    stdout.to_string()
                } else {
                    format!("{stdout}\n--- stderr ---\n{stderr}")
                };
                VerificationResult {
                    tool: tool.to_string(),
                    passed: out.status.success(),
                    output: combined,
                    duration_ms,
                }
            }
            Err(e) => VerificationResult {
                tool: tool.to_string(),
                passed: false,
                output: format!("Failed to execute {program}: {e}"),
                duration_ms,
            },
        }
    }

    pub fn run_lint(path: &Path) -> VerificationResult {
        Self::run_command("lint", "cargo", &["clippy", "--all-targets", "--", "-D", "warnings"], path)
    }

    pub fn run_tests(path: &Path) -> VerificationResult {
        Self::run_command("test", "cargo", &["test"], path)
    }

    pub fn run_build(path: &Path) -> VerificationResult {
        Self::run_command("build", "cargo", &["build", "--release"], path)
    }

    pub fn run_check(path: &Path) -> VerificationResult {
        Self::run_command("check", "cargo", &["check"], path)
    }

    pub fn run_typecheck(path: &Path) -> VerificationResult {
        let rs_files = Self::find_files(path, "*.rs");
        if rs_files > 0 {
            Self::run_command("typecheck", "cargo", &["check"], path)
        } else {
            let py_files = Self::find_files(path, "*.py");
            if py_files > 0 {
                Self::run_command("typecheck", "python", &["-m", "mypy", "."], path)
            } else {
                let ts_files = Self::find_files(path, "*.ts");
                if ts_files > 0 {
                    Self::run_command("typecheck", "npx", &["tsc", "--noEmit"], path)
                } else {
                    VerificationResult {
                        tool: "typecheck".to_string(),
                        passed: true,
                        output: "No typecheck tool available for this project".to_string(),
                        duration_ms: 0,
                    }
                }
            }
        }
    }

    pub fn compute_confidence(results: &[VerificationResult]) -> f64 {
        if results.is_empty() {
            return 0.5;
        }
        let passed: usize = results.iter().filter(|r| r.passed).count();
        passed as f64 / results.len() as f64
    }

    pub fn run_all(path: &Path) -> SrrResult<Vec<VerificationResult>> {
        let results = vec![
            Self::run_build(path),
            Self::run_lint(path),
            Self::run_typecheck(path),
            Self::run_tests(path),
        ];
        Ok(results)
    }

    pub fn run_python_lint(path: &Path) -> VerificationResult {
        Self::run_command("ruff", "ruff", &["check", "."], path)
    }

    pub fn run_js_lint(path: &Path) -> VerificationResult {
        Self::run_command("eslint", "npx", &["eslint", "."], path)
    }

    fn find_files(path: &Path, pattern: &str) -> usize {
        let pattern_str = format!("{}/{}", path.to_string_lossy(), pattern).replace("//", "/");
        match glob::glob(&pattern_str) {
            Ok(entries) => entries.filter_map(|e| e.ok()).count(),
            Err(_) => 0,
        }
    }
}
