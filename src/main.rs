use std::env;
use std::ffi::OsString;
use std::fs::File;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::process::{self, Command, Stdio};

extern crate isatty;
use isatty::{stderr_isatty, stdout_isatty};

extern crate tempfile;

fn main() {
    if env::args_os().any(|arg| arg == *"--version") {
        println!(concat!("cargo-expand ", env!("CARGO_PKG_VERSION")));
        return;
    }

    let result = cargo_expand_or_run_nightly();
    process::exit(match result {
        Ok(code) => code,
        Err(err) => {
            let _ = writeln!(&mut io::stderr(), "{}", err);
            1
        }
    });
}

fn cargo_expand_or_run_nightly() -> io::Result<i32> {
    const NO_RUN_NIGHTLY: &str = "CARGO_EXPAND_NO_RUN_NIGHTLY";

    let maybe_nightly = !definitely_not_nightly();
    if maybe_nightly || env::var_os(NO_RUN_NIGHTLY).is_some() {
        return cargo_expand();
    }

    let mut nightly = Command::new("cargo");
    nightly.arg("+nightly");
    nightly.arg("expand");

    let mut args = env::args_os().peekable();
    args.next().unwrap(); // cargo
    if args.peek().map_or(false, |arg| arg == "expand") {
        args.next().unwrap(); // expand
    }
    nightly.args(args);

    // Hopefully prevent infinite re-run loop.
    nightly.env(NO_RUN_NIGHTLY, "");

    let status = nightly.status()?;

    Ok(match status.code() {
        Some(code) => code,
        None => if status.success() { 0 } else { 1 },
    })
}

fn definitely_not_nightly() -> bool {
    let mut cmd = Command::new(cargo_binary());
    cmd.arg("--version");

    let output = match cmd.output() {
        Ok(output) => output,
        Err(_) => return false,
    };

    let version = match String::from_utf8(output.stdout) {
        Ok(version) => version,
        Err(_) => return false,
    };

    version.starts_with("cargo 1") && !version.contains("nightly")
}

fn cargo_binary() -> OsString {
    env::var_os("CARGO").unwrap_or_else(|| "cargo".to_owned().into())
}

fn cargo_expand() -> io::Result<i32> {
    let args: Vec<_> = env::args_os().collect();

    let which_rustfmt = which(&["rustfmt"]);
    let which_pygmentize = if !color_never(&args) && stdout_isatty() {
        which(&["pygmentize", "-l", "rust"])
    } else {
        None
    };

    let outdir = if which_rustfmt.is_some() || which_pygmentize.is_some() {
        let mut builder = tempfile::Builder::new();
        builder.prefix("cargo-expand");
        Some(builder.tempdir().expect("failed to create tmp file"))
    } else {
        None
    };
    let outfile = outdir.as_ref().map(|dir| dir.path().join("expanded"));

    // Run cargo
    let mut cmd = Command::new(cargo_binary());
    cmd.args(&wrap_args(args.clone(), outfile.as_ref()));
    let code = filter_err(&mut cmd, ignore_cargo_err)?;
    if code != 0 {
        return Ok(code);
    }

    // Run rustfmt
    if let Some(fmt) = which_rustfmt {
        // Ignore any errors.
        let _status = Command::new(fmt)
            .arg(outfile.as_ref().unwrap())
            .stderr(Stdio::null())
            .status();
    }

    // Run pygmentize
    if let Some(pyg) = which_pygmentize {
        let _status = Command::new(pyg)
            .args(&["-l", "rust", "-O", "encoding=utf8"])
            .arg(outfile.as_ref().unwrap())
            .status();
    } else if let Some(outfile) = outfile {
        // Cat outfile if rustfmt was used.
        let mut reader = File::open(outfile)?;
        io::copy(&mut reader, &mut io::stdout())?;
    }
    Ok(0)
}

// Based on https://github.com/rsolomo/cargo-check
fn wrap_args<I>(it: I, outfile: Option<&PathBuf>) -> Vec<OsString>
where
    I: IntoIterator<Item = OsString>,
{
    let mut args = vec!["rustc".into(), "--profile=check".into()];
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

fn color_never(args: &Vec<OsString>) -> bool {
    args.windows(2).any(|pair| pair[0] == *"--color" && pair[1] == *"never")
        || args.iter().any(|arg| *arg == *"--color=never")
}

fn which(cmd: &[&str]) -> Option<OsString> {
    if env::args_os().find(|arg| arg == "--help").is_some() {
        return None;
    }

    if let Some(which) = env::var_os(&cmd[0].to_uppercase()) {
        return if which.is_empty() { None } else { Some(which) };
    }

    let spawn = Command::new(cmd[0])
        .args(&cmd[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    let mut child = match spawn {
        Ok(child) => child,
        Err(_) => {
            return None;
        }
    };

    let exit = match child.wait() {
        Ok(exit) => exit,
        Err(_) => {
            return None;
        }
    };

    if exit.success() {
        Some(cmd[0].into())
    } else {
        None
    }
}

fn filter_err(cmd: &mut Command, ignore: fn(&str) -> bool) -> io::Result<i32> {
    let mut child = cmd.stderr(Stdio::piped()).spawn()?;
    let mut stderr = io::BufReader::new(child.stderr.take().unwrap());
    let mut line = String::new();
    while let Ok(n) = stderr.read_line(&mut line) {
        if n == 0 {
            break;
        }
        if !ignore(&line) {
            let _ = write!(&mut io::stderr(), "{}", line);
        }
        line.clear();
    }
    let code = child.wait()?.code().unwrap_or(1);
    Ok(code)
}

fn ignore_cargo_err(line: &str) -> bool {
    if line.trim().is_empty() {
        return true;
    }

    let blacklist = [
        "ignoring specified output filename because multiple outputs were \
         requested",
        "ignoring specified output filename for 'link' output because multiple \
         outputs were requested",
        "ignoring --out-dir flag due to -o flag",
        "ignoring -C extra-filename flag due to -o flag",
        "due to multiple output types requested, the explicitly specified \
         output file name will be adapted for each output type",
    ];
    for s in &blacklist {
        if line.contains(s) {
            return true;
        }
    }

    false
}
