#![allow(dead_code)]

use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{self, Command};

#[test]
fn test_issue_quota() {
    let mut cmd = cmd_for_example("test_issue_quota");
    let out = cmd_output(&mut cmd);
    let _: String = out.stdout().to_string();
}

#[test]
fn test_quota_distribution() {
    let mut cmd = cmd_for_example("test_quota_distribution");
    let out = cmd_output(&mut cmd);
    let _: String = out.stdout().to_string();
}

#[test]
fn test_currency() {
    let mut cmd = cmd_for_example("test_currency");
    let out = cmd_output(&mut cmd);
    let _: String = out.stdout().to_string();
}

#[test]
fn test_recycle_quota() {
    let mut cmd = cmd_for_example("test_recycle_quota");
    let out = cmd_output(&mut cmd);
    let _: String = out.stdout().to_string();
}

#[test]
fn test_convert_quota() {
    let mut cmd = cmd_for_example("test_convert_quota");
    let out = cmd_output(&mut cmd);
    let _: String = out.stdout().to_string();
}

#[test]
fn test_transaction() {
    let mut cmd = cmd_for_example("test_transaction");
    let out = cmd_output(&mut cmd);
    let _: String = out.stdout().to_string();
}

#[test]
fn test_currency_convert() {
    let mut cmd = cmd_for_example("test_currency_convert");
    let out = cmd_output(&mut cmd);
    let _: String = out.stdout().to_string();
}

// Helper functions follow.

/// Return the target/debug directory path.
fn debug_dir() -> PathBuf {
    env::current_exe()
        .expect("test binary path")
        .parent()
        .expect("test binary directory")
        .parent()
        .expect("example binary directory")
        .to_path_buf()
}

/// Return the directory containing the example test binaries.
fn example_bin_dir() -> PathBuf {
    debug_dir().join("examples")
}

/// Return the repo root directory path.
fn repo_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Return the directory containing the example data.
fn data_dir() -> PathBuf {
    repo_dir().join("examples").join("data")
}

/// Return a command ready to execute the given example test binary.
///
/// The command's current directory is set to the repo root.
fn cmd_for_example(name: &str) -> Command {
    let mut cmd = Command::new(example_bin_dir().join(name));
    cmd.current_dir(repo_dir());
    cmd
}

/// Return the (stdout, stderr) of running the command as a string.
///
/// If the command has a non-zero exit code, then this function panics.
fn cmd_output(cmd: &mut Command) -> Output {
    cmd.stdout(process::Stdio::piped());
    cmd.stderr(process::Stdio::piped());
    let child = cmd.spawn().expect("command spawns successfully");
    Output::new(cmd, child)
}

/// Like cmd_output, but sends the given data as stdin to the given child.
fn cmd_output_with(cmd: &mut Command, data: &[u8]) -> Output {
    cmd.stdin(process::Stdio::piped());
    cmd.stdout(process::Stdio::piped());
    cmd.stderr(process::Stdio::piped());
    let mut child = cmd.spawn().expect("command spawns successfully");
    {
        let stdin = child.stdin.as_mut().expect("failed to get stdin");
        stdin.write_all(data).expect("failed to write to stdin");
    }
    Output::new(cmd, child)
}

struct Output {
    stdout: String,
    stderr: String,
    command: String,
    status: process::ExitStatus,
}

impl Output {
    /// Return the (stdout, stderr) of running the given child as a string.
    ///
    /// If the command has a non-zero exit code, then this function panics.
    fn new(cmd: &mut Command, child: process::Child) -> Output {
        let out = child.wait_with_output().expect("command runs successfully");
        let stdout = String::from_utf8(out.stdout).expect("valid utf-8 (stdout)");
        let stderr = String::from_utf8(out.stderr).expect("valid utf-8 (stderr)");
        Output {
            stdout: stdout,
            stderr: stderr,
            command: format!("{:?}", cmd),
            status: out.status,
        }
    }

    fn stdout(&self) -> &str {
        if !self.status.success() {
            panic!(
                "\n\n==== {:?} ====\n\
                 command failed but expected success!\
                 \n\ncwd: {}\
                 \n\nstatus: {}\
                 \n\nstdout: {}\
                 \n\nstderr: {}\
                 \n\n=====\n",
                self.command,
                repo_dir().display(),
                self.status,
                self.stdout,
                self.stderr
            );
        }
        &self.stdout
    }

    fn stdout_failed(&self) -> &str {
        if self.status.success() {
            panic!(
                "\n\n==== {:?} ====\n\
                 command succeeded but expected failure!\
                 \n\ncwd: {}\
                 \n\nstatus: {}\
                 \n\nstdout: {}\
                 \n\nstderr: {}\
                 \n\n=====\n",
                self.command,
                repo_dir().display(),
                self.status,
                self.stdout,
                self.stderr
            );
        }
        &self.stdout
    }

    fn stderr(&self) -> &str {
        if self.status.success() {
            panic!(
                "\n\n==== {:?} ====\n\
                 command succeeded but expected failure!\
                 \n\ncwd: {}\
                 \n\nstatus: {}\
                 \n\nstdout: {}\
                 \n\nstderr: {}\
                 \n\n=====\n",
                self.command,
                repo_dir().display(),
                self.status,
                self.stdout,
                self.stderr
            );
        }
        &self.stderr
    }
}

/// Consume the reader given into a string.
fn read_to_string<R: io::Read>(mut rdr: R) -> String {
    let mut s = String::new();
    rdr.read_to_string(&mut s).unwrap();
    s
}
