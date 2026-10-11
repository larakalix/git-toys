use crate::types::{GitError, Result};
use std::{
    io::{Read, Write},
    path::Path,
    process::{Command, Stdio},
};

pub type Progress = std::sync::Arc<dyn Fn(String) + Send + Sync>;

#[derive(Clone, Default)]
pub struct Auth {
    pub executable: String,
    pub address: String,
    pub token: String,
    pub progress: Option<Progress>,
}

pub fn command(path: &Path, trusted: bool, auth: Option<&Auth>) -> Command {
    let mut c = Command::new("git");
    c.current_dir(path)
        .args([
            "--no-pager",
            "-c",
            "color.ui=false",
            "-c",
            "protocol.ext.allow=never",
        ])
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_EDITOR", "true")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_PAGER", "cat")
        .env("GIT_LITERAL_PATHSPECS", "1");
    if !trusted {
        c.args([
            "-c",
            "core.hooksPath=",
            "-c",
            "core.fsmonitor=false",
            "-c",
            "credential.helper=",
            "-c",
            "core.sshCommand=ssh",
        ]);
    }
    if let Some(auth) = auth {
        c.env("GIT_ASKPASS", &auth.executable)
            .env("SSH_ASKPASS", &auth.executable)
            .env("SSH_ASKPASS_REQUIRE", "prefer")
            .env("GIT_TOYS_AUTH_ADDRESS", &auth.address)
            .env("GIT_TOYS_AUTH_TOKEN", &auth.token);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x08000000);
    }
    c
}

pub fn run_output(
    path: &Path,
    trusted: bool,
    auth: Option<&Auth>,
    args: &[&str],
    input: Option<&[u8]>,
) -> Result<(Vec<u8>, Vec<u8>)> {
    run_output_editor(path, trusted, auth, args, input, None)
}
pub fn run_output_editor(
    path: &Path,
    trusted: bool,
    auth: Option<&Auth>,
    args: &[&str],
    input: Option<&[u8]>,
    editor: Option<(&Path, &Path)>,
) -> Result<(Vec<u8>, Vec<u8>)> {
    let mut c = command(path, trusted, auth);
    if let Some((executable, plan)) = editor {
        let path = executable.to_string_lossy();
        let path = if cfg!(windows) {
            path.replace('\\', "/")
        } else {
            path.into_owned()
        };
        let quoted = format!("'{}'", path.replace('\'', "'\\''"));
        c.env("GIT_EDITOR", &quoted)
            .env("GIT_SEQUENCE_EDITOR", &quoted)
            .env("GIT_TOYS_REBASE_PLAN", plan);
    }

    c.args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = c
        .spawn()
        .map_err(|e| GitError::new("gitUnavailable", format!("Cannot run Git: {e}")))?;
    // Drain both pipes before feeding stdin to avoid deadlocks on verbose hooks or patches.
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| GitError::new("io", "Missing stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| GitError::new("io", "Missing stderr"))?;
    let progress = auth.and_then(|auth| auth.progress.clone());
    let read = |mut pipe: Box<dyn Read + Send>, progress: Option<Progress>| {
        let mut bytes = Vec::new();
        let mut buffer = [0u8; 8192];
        loop {
            match pipe.read(&mut buffer) {
                Ok(0) => return (Ok(()), bytes),
                Ok(count) => {
                    if let Some(callback) = &progress {
                        callback(String::from_utf8_lossy(&buffer[..count]).into());
                    }
                    let remaining = 16_000_000usize.saturating_sub(bytes.len());
                    bytes.extend_from_slice(&buffer[..count.min(remaining)]);
                }
                Err(error) => return (Err(error), bytes),
            }
        }
    };
    let out_thread = std::thread::spawn(move || read(Box::new(stdout), None));
    let err_thread = std::thread::spawn(move || read(Box::new(stderr), progress));
    let write_result = if let Some(bytes) = input {
        child
            .stdin
            .take()
            .ok_or_else(|| GitError::new("io", "Missing stdin"))?
            .write_all(bytes)
    } else {
        Ok(())
    };
    let status = child.wait()?;
    let (out_result, out) = out_thread
        .join()
        .map_err(|_| GitError::new("io", "Output reader failed"))?;
    let (err_result, err) = err_thread
        .join()
        .map_err(|_| GitError::new("io", "Error reader failed"))?;
    out_result?;
    err_result?;
    if !status.success() {
        return Err(GitError {
            code: "gitFailed".into(),
            message: format!(
                "Git {} failed ({}).",
                args.first().unwrap_or(&"command"),
                status.code().unwrap_or(-1)
            ),
            detail: format!(
                "{}{}",
                String::from_utf8_lossy(&out),
                String::from_utf8_lossy(&err)
            ),
        });
    }
    write_result?;
    Ok((out, err))
}

pub fn run(
    path: &Path,
    trusted: bool,
    auth: Option<&Auth>,
    args: &[&str],
    input: Option<&[u8]>,
) -> Result<Vec<u8>> {
    run_output(path, trusted, auth, args, input).map(|(out, _)| out)
}
pub fn text(bytes: Vec<u8>) -> String {
    String::from_utf8_lossy(&bytes)
        .trim_end_matches('\n')
        .to_string()
}
