mod test_conditionals;
mod test_converters;
mod test_custom_commands;
mod test_engine;
mod test_env;
mod test_hiding;
mod test_iteration;
mod test_math;
mod test_modules;
mod test_parser;
mod test_ranges;
mod test_strings;
mod test_table_operations;
mod test_type_check;

use assert_cmd::prelude::*;
use pretty_assertions::assert_eq;
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

#[cfg(test)]
pub fn run_test(input: &str, expected: &str) -> TestResult {
    let mut file = NamedTempFile::new()?;
    let name = file.path();

    let mut cmd = Command::cargo_bin("engine-q")?;
    cmd.arg(name);

    writeln!(file, "{}", input)?;

    let output = cmd.output()?;

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    println!("stdout: {}", stdout);
    println!("stderr: {}", stderr);

    assert!(output.status.success());

    assert_eq!(stdout.trim(), expected);

    Ok(())
}

#[cfg(test)]
pub fn fail_test(input: &str, expected: &str) -> TestResult {
    let mut file = NamedTempFile::new()?;
    let name = file.path();

    let mut cmd = Command::cargo_bin("engine-q")?;
    cmd.arg(name);

    writeln!(file, "{}", input)?;

    let output = cmd.output()?;

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    println!("stdout: {}", stdout);
    println!("stderr: {}", stderr);

    assert!(stderr.contains(expected));

    Ok(())
}

#[cfg(test)]
pub fn not_found_msg() -> &'static str {
    if cfg!(windows) {
        "cannot find"
    } else {
        "No such"
    }
}
