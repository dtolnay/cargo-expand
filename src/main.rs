use std::env;
use std::io::{self, Write};
use std::process::{self, Command, Stdio};

extern crate isatty;
use isatty::{stdout_isatty, stderr_isatty};

fn main() {
    // Build cargo command
    let mut cargo = Command::new("cargo");
    cargo.args(&wrap_args(env::args()));

    // Run cargo command, print errors, exit if failed
    let expanded = cargo.output().unwrap();
    for line in String::from_utf8_lossy(&expanded.stderr).lines() {
        writeln!(io::stderr(), "{}", line).unwrap();
    }
    if !expanded.status.success() {
        process::exit(expanded.status.code().unwrap_or(1));
    }

    let rustfmt = env::var("RUSTFMT").unwrap_or("rustfmt".to_string());

    // Just print the expanded output if rustfmt is not available
    if rustfmt == "" || !have_rustfmt() {
        io::stdout().write_all(&expanded.stdout).unwrap();
        return;
    }

    // Build rustfmt command and give it the expanded code
    let mut rustfmt = Command::new(rustfmt)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    rustfmt.stdin.as_mut().unwrap().write_all(&expanded.stdout).unwrap();

    // Print rustfmt errors and exit if failed
    let formatted = rustfmt.wait_with_output().unwrap();
    for line in String::from_utf8_lossy(&formatted.stderr).lines() {
        if !ignore_rustfmt_err(line) {
            writeln!(io::stderr(), "{}", line).unwrap();
        }
    }
    if !formatted.status.success() {
        let code = formatted.status.code().unwrap_or(1);
        // Ignore code 3 which is formatting errors
        if code != 3 {
            process::exit(code);
        }
    }

    // Print formatted output
    io::stdout().write_all(&formatted.stdout).unwrap();
}

// Based on https://github.com/rsolomo/cargo-check
fn wrap_args<T, I>(it: I) -> Vec<String>
    where T: AsRef<str>,
          I: IntoIterator<Item=T>
{
    let mut args = vec!["rustc".to_string()];
    let mut has_color = false;
    let mut has_double_hyphen = false;

    for arg in it.into_iter().skip(2) {
        let arg = arg.as_ref().to_string();
        has_color |= arg.starts_with("--color");
        has_double_hyphen |= arg == "--";
        args.push(arg);
    }

    if !has_color {
        let color = stdout_isatty() && stderr_isatty();
        let setting = if color { "always" } else { "never" };
        args.push(format!("--color={}", setting));
    }

    if !has_double_hyphen {
        args.push("--".to_string());
    }

    args.push("-Zunstable-options".to_string());
    args.push("--pretty=expanded".to_string());

    args
}

fn have_rustfmt() -> bool {
    Command::new("rustfmt")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_ok()
}

fn ignore_rustfmt_err(line: &str) -> bool {
    line.is_empty()
        || line.ends_with("line exceeded maximum length (sorry)")
        || line.ends_with("left behind trailing whitespace (sorry)")
}
