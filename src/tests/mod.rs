mod arrays;
mod basic;
mod collections;

use std::{
    fs,
    io::{stderr, stdout, Write},
};

fn assert_command(cmd: &mut std::process::Command) {
    let output = cmd.output().expect("Command should succeed");
    if output.status.success() {
        return;
    }
    stdout()
        .write_all(&output.stdout)
        .expect("Failed to write to stdout");
    stderr()
        .write_all(&output.stderr)
        .expect("Failed to write to stderr");
    panic!("Command failed.");
}

pub fn test(testname: &str, code: &str) {
    fs::write(
        format!("java_tests/{testname}.java"),
        format!("public class {testname}{{public static void main(String[]args){{{code}}}}}"),
    )
    .expect("File write should succeed");
    assert_command(
        std::process::Command::new("javac")
            .arg(format!("java_tests/{testname}.java"))
            .args(["-d", "java_tests"]),
    );
    assert_command(
        std::process::Command::new("cargo")
            .args(["run", "--", "-v"])
            .arg(format!("java_tests/{testname}.class")),
    );
}

pub fn test_fail(testname: &str, code: &str) {
    fs::write(
        format!("java_tests/{testname}.java"),
        format!("public class {testname}{{public static void main(String[] args){{{code}}}}}"),
    )
    .expect("file write should succeed");
    assert_command(
        std::process::Command::new("javac")
            .arg(format!("java_tests/{testname}.java"))
            .args(["-d", "java_tests"]),
    );
    let output = std::process::Command::new("cargo")
        .args(["run", "--", "-v"])
        .arg(format!("java_tests/{testname}.class"))
        .output();
    let Ok(output) = output else { return };
    if output.status.success() {
        stdout()
            .write_all(&output.stdout)
            .expect("Failed to write to stdout");
        stderr()
            .write_all(&output.stderr)
            .expect("Failed to write to stderr");
        panic!("Test should produce exception");
    }
}
