use std::env;
use std::io::{self, Write};
use std::process::{self, Command};

#[cfg(unix)]
use std::process::{Child, Stdio};

extern crate isatty;
use isatty::{stdout_isatty, stderr_isatty};

fn main() {
    let result = cargo_expand();
    process::exit(match result {
        Ok(code) => code,
        Err(err) => {
            let _ = writeln!(&mut io::stderr(), "{}", err);
            1
        }
    });
}

#[cfg(windows)]
fn cargo_expand() -> io::Result<i32> {
    // Build cargo command
    let mut cmd = Command::new("cargo");
    cmd.args(&wrap_args(env::args()));
    run(cmd)
}

#[cfg(unix)]
fn cargo_expand() -> io::Result<i32> {
    if env::args().last().unwrap() == "--filter-rustfmt" {
        filter_rustfmt();
    }

    // Build cargo command
    let mut cmd = Command::new("cargo");
    cmd.args(&wrap_args(env::args()));

    // Pipe to rustfmt
    let _wait = match which(&["rustfmt"]) {
        Some(ref fmt) => {
            let args: Vec<_> = env::args().collect();
            let mut filter_args = Vec::new();
            for i in 0..args.len() {
                filter_args.push(args[i].as_str());
            }
            filter_args.push("--filter-rustfmt");

            Some((
                // Work around $crate issue https://github.com/rust-lang/rust/issues/38016
                try!(cmd.pipe_to(&["sed", "s/$crate/XCRATE/g"], None)),
                try!(cmd.pipe_to(&[fmt], None)),
                try!(cmd.pipe_to(&["sed", "s/XCRATE/$crate/g"], Some(&filter_args))),
            ))
        }
        None => None,
    };

    // Pipe to pygmentize
    let _wait = if stdout_isatty() {
        match which(&["pygmentize", "-l", "rust"]) {
            Some(pyg) => Some(try!(cmd.pipe_to(&[&pyg, "-l", "rust"], None))),
            None => None,
        }
    } else {
        None
    };

    run(cmd)
}

fn run(mut cmd: Command) -> io::Result<i32> {
    cmd.status().map(|status| status.code().unwrap_or(1))
}

#[cfg(unix)]
struct Wait(Vec<Child>);

#[cfg(unix)]
impl Drop for Wait {
    fn drop(&mut self) {
        for child in &mut self.0 {
            if let Err(err) = child.wait() {
                let _ = writeln!(&mut io::stderr(), "{}", err);
            }
        }
    }
}

#[cfg(unix)]
trait PipeTo {
    fn pipe_to(&mut self, out: &[&str], err: Option<&[&str]>) -> io::Result<Wait>;
}

#[cfg(unix)]
impl PipeTo for Command {
    fn pipe_to(&mut self, out: &[&str], err: Option<&[&str]>) -> io::Result<Wait> {
        use std::os::unix::io::{AsRawFd, FromRawFd};

        self.stdout(Stdio::piped());
        if err.is_some() {
            self.stderr(Stdio::piped());
        }

        let child = try!(self.spawn());

        *self = Command::new(out[0]);
        self.args(&out[1..]);
        self.stdin(unsafe {
            Stdio::from_raw_fd(child.stdout.as_ref().map(AsRawFd::as_raw_fd).unwrap())
        });

        match err {
            None => {
                Ok(Wait(vec![child]))
            }
            Some(err) => {
                let mut errcmd = Command::new(err[0]);
                errcmd.args(&err[1..]);
                errcmd.stdin(unsafe {
                    Stdio::from_raw_fd(child.stderr.as_ref().map(AsRawFd::as_raw_fd).unwrap())
                });
                errcmd.stdout(Stdio::null());
                errcmd.stderr(Stdio::inherit());
                let spawn = try!(errcmd.spawn());
                Ok(Wait(vec![spawn, child]))
            }
        }
    }
}

// Based on https://github.com/rsolomo/cargo-check
fn wrap_args<T, I>(it: I) -> Vec<String>
    where T: AsRef<str>,
          I: IntoIterator<Item=T>
{
    let mut args = vec!["rustc".to_string()];
    let mut has_color = false;
    let mut has_double_hyphen = false;
    let mut ends_with_test = false;
    let mut ends_with_example = false;

    for arg in it.into_iter().skip(2) {
        let arg = arg.as_ref().to_string();
        has_color |= arg.starts_with("--color");
        has_double_hyphen |= arg == "--";
        ends_with_test = arg == "--test";
        ends_with_example = arg == "--example";
        args.push(arg);
    }

    if !has_double_hyphen && ends_with_test {
        // Expand the `test.rs` test by default.
        args.push("test".to_string());
    }

    if !has_double_hyphen && ends_with_example {
        // Expand the `example.rs` example by default.
        args.push("example".to_string());
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

#[cfg(unix)]
fn which(cmd: &[&str]) -> Option<String> {
    if env::args().find(|arg| arg == "--help").is_some() {
        None
    } else {
        match env::var(&cmd[0].to_uppercase()) {
            Ok(which) => {
                if which.is_empty() {
                    None
                } else {
                    Some(which)
                }
            }
            Err(_) => {
                let in_path = Command::new(cmd[0])
                    .args(&cmd[1..])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .is_ok();
                if in_path {
                    Some(cmd[0].to_string())
                } else {
                    None
                }
            }
        }
    }
}

#[cfg(unix)]
fn filter_rustfmt() -> ! {
    let mut line = String::new();
    while let Ok(n) = io::stdin().read_line(&mut line) {
        if n == 0 {
            break;
        }
        if !ignore_rustfmt_err(&line) {
            let _ = write!(&mut io::stderr(), "{}", line);
        }
        line.clear();
    }
    process::exit(0);
}

#[cfg(unix)]
fn ignore_rustfmt_err(line: &str) -> bool {
    line.trim().is_empty()
        || line.trim_right().ends_with("line exceeded maximum length (sorry)")
        || line.trim_right().ends_with("left behind trailing whitespace (sorry)")
}
