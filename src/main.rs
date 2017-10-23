use std::env;
use std::ffi::{OsStr, OsString};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{self, Command};

#[cfg(unix)]
use std::process::{Child, Stdio};

extern crate isatty;
use isatty::{stdout_isatty, stderr_isatty};

#[cfg(unix)]
extern crate tempdir;
#[cfg(unix)]
use tempdir::TempDir;

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
    cmd.args(&wrap_args(env::args_os(), None));
    run(cmd)
}

#[cfg(unix)]
fn cargo_expand() -> io::Result<i32> {
    let args: Vec<_> = env::args_os().collect();
    match args.last().unwrap().to_str().unwrap_or("") {
        "--filter-cargo" => filter_err(ignore_cargo_err),
        "--filter-rustfmt" => filter_err(ignore_rustfmt_err),
        _ => {}
    }

    macro_rules! shell {
        ($($arg:expr)*) => {
            &[$(OsStr::new(&$arg)),*]
        };
    }

    let which_rustfmt = which(&["rustfmt"]);
    let which_pygmentize = if stdout_isatty() {
        which(&["pygmentize", "-l", "rust"])
    } else {
        None
    };

    let outdir = if which_rustfmt.is_some() || which_pygmentize.is_some() {
        Some(TempDir::new("cargo-expand").expect("failed to create tmp file"))
    } else {
        None
    };
    let outfile = outdir.as_ref().map(|dir| dir.path().join("expanded"));

    // Build cargo command
    let mut cmd = Command::new("cargo");
    cmd.args(&wrap_args(args.clone(), outfile.as_ref()));

    // Pipe to a tmp file to separate out any println output from build scripts
    if let Some(outfile) = outfile {
        let mut filter_cargo = Vec::new();
        filter_cargo.extend(args.iter().map(OsString::as_os_str));
        filter_cargo.push(OsStr::new("--filter-cargo"));

        let _wait = cmd.pipe_to(shell!("cat"), Some(&filter_cargo))?;
        run(cmd)?;
        drop(_wait);

        cmd = Command::new("cat");
        cmd.arg(outfile);
    }

    // Pipe to rustfmt
    let _wait = match which_rustfmt {
        Some(ref fmt) => {
            let args: Vec<_> = env::args_os().collect();
            let mut filter_rustfmt = Vec::new();
            filter_rustfmt.extend(args.iter().map(OsString::as_os_str));
            filter_rustfmt.push(OsStr::new("--filter-rustfmt"));

            Some(cmd.pipe_to(shell!(fmt), Some(&filter_rustfmt))?)
        }
        None => None,
    };

    // Pipe to pygmentize
    let _wait = match which_pygmentize {
        Some(pyg) => Some(cmd.pipe_to(shell!(pyg "-l" "rust"), None)?),
        None => None,
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
    fn pipe_to(&mut self, out: &[&OsStr], err: Option<&[&OsStr]>) -> io::Result<Wait>;
}

#[cfg(unix)]
impl PipeTo for Command {
    fn pipe_to(&mut self, out: &[&OsStr], err: Option<&[&OsStr]>) -> io::Result<Wait> {
        use std::os::unix::io::{AsRawFd, FromRawFd};

        self.stdout(Stdio::piped());
        if err.is_some() {
            self.stderr(Stdio::piped());
        }

        let child = self.spawn()?;

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
                let spawn = errcmd.spawn()?;
                Ok(Wait(vec![spawn, child]))
            }
        }
    }
}

// Based on https://github.com/rsolomo/cargo-check
fn wrap_args<I>(it: I, outfile: Option<&PathBuf>) -> Vec<OsString>
    where I: IntoIterator<Item=OsString>
{
    let mut args = vec!["rustc".into()];
    let mut ends_with_test = false;
    let mut ends_with_example = false;
    let mut has_color = false;

    let mut it = it.into_iter().skip(2);
    for arg in &mut it {
        if arg == *"--" {
            break;
        }
        ends_with_test = arg == *"--test";
        ends_with_example = arg == *"--example";
        has_color |= arg.to_str().unwrap_or("").starts_with("--color");
        args.push(arg.into());
    }

    if ends_with_test {
        // Expand the `test.rs` test by default.
        args.push("test".into());
    }

    if ends_with_example {
        // Expand the `example.rs` example by default.
        args.push("example".into());
    }

    if !has_color {
        let color = stderr_isatty();
        let setting = if color { "always" } else { "never" };
        args.push(format!("--color={}", setting).into());
    }

    args.push("--".into());
    if let Some(path) = outfile {
        args.push("-o".into());
        args.push(path.into());
    }
    args.push("-Zunstable-options".into());
    args.push("--pretty=expanded".into());
    args.extend(it);
    args
}

#[cfg(unix)]
fn which(cmd: &[&str]) -> Option<OsString> {
    if env::args_os().find(|arg| arg == "--help").is_some() {
        None
    } else {
        match env::var_os(&cmd[0].to_uppercase()) {
            Some(which) => {
                if which.is_empty() {
                    None
                } else {
                    Some(which)
                }
            }
            None => {
                let in_path = Command::new(cmd[0])
                    .args(&cmd[1..])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .is_ok();
                if in_path {
                    Some(cmd[0].into())
                } else {
                    None
                }
            }
        }
    }
}

#[cfg(unix)]
fn filter_err(ignore: fn(&str) -> bool) -> ! {
    let mut line = String::new();
    while let Ok(n) = io::stdin().read_line(&mut line) {
        if n == 0 {
            break;
        }
        if !ignore(&line) {
            let _ = write!(&mut io::stderr(), "{}", line);
        }
        line.clear();
    }
    process::exit(0);
}

#[cfg(unix)]
fn ignore_rustfmt_err(line: &str) -> bool {
    line.trim().is_empty()
        || line.contains(": line exceeded maximum length (")
        || line.trim_right().ends_with("left behind trailing whitespace (sorry)")
}

#[cfg(unix)]
fn ignore_cargo_err(line: &str) -> bool {
    line.trim().is_empty()
        || line.contains("ignoring specified output filename because multiple outputs were requested")
        || line.contains("ignoring specified output filename for 'link' output because multiple outputs were requested")
        || line.contains("ignoring --out-dir flag due to -o flag.")
        || line.contains("due to multiple output types requested, the explicitly specified output file name will be adapted for each output type")
}
