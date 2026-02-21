use std::path::Path;
use std::process::Command;

use gitr_core::error::GitrError;

/// Result of a git command execution.
#[derive(Debug)]
pub struct GitOutput {
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

/// Run a git command in the given directory.
fn git(dir: &Path, args: &[&str]) -> Result<GitOutput, GitrError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|e| GitrError::GitError {
            message: format!("failed to run git {}: {e}", args.join(" ")),
        })?;

    Ok(GitOutput {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        success: output.status.success(),
    })
}

/// Run a git command in the given directory, returning an error if it fails.
fn git_ok(dir: &Path, args: &[&str]) -> Result<String, GitrError> {
    let out = git(dir, args)?;
    if !out.success {
        return Err(GitrError::GitError {
            message: format!(
                "git {} failed: {}",
                args.join(" "),
                out.stderr.trim()
            ),
        });
    }
    Ok(out.stdout)
}

/// Clone a repo to a local path.
pub fn clone(url: &str, dest: &Path) -> Result<(), GitrError> {
    let dest_str = dest.to_string_lossy();
    let output = Command::new("git")
        .args(["clone", url, &dest_str])
        .output()
        .map_err(|e| GitrError::GitError {
            message: format!("failed to clone {url}: {e}"),
        })?;

    if !output.status.success() {
        return Err(GitrError::GitError {
            message: format!(
                "git clone failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        });
    }
    Ok(())
}

/// Fetch a remote, pruning deleted branches.
pub fn fetch(dir: &Path, remote: &str) -> Result<(), GitrError> {
    git_ok(dir, &["fetch", remote, "--prune"])?;
    Ok(())
}

/// Checkout a branch.
pub fn checkout(dir: &Path, branch: &str) -> Result<(), GitrError> {
    git_ok(dir, &["checkout", branch])?;
    Ok(())
}

/// Fast-forward merge from a remote branch.
pub fn merge_ff(dir: &Path, remote_branch: &str) -> Result<(), GitrError> {
    let out = git(dir, &["merge", "--ff-only", remote_branch])?;
    if !out.success {
        return Err(GitrError::FastForwardFailed {
            branch: remote_branch.to_string(),
            message: out.stderr.trim().to_string(),
        });
    }
    Ok(())
}

/// Regular merge from a remote branch.
pub fn merge(dir: &Path, remote_branch: &str) -> Result<(), GitrError> {
    let out = git(dir, &["merge", remote_branch, "--no-edit"])?;
    if !out.success {
        return Err(GitrError::MergeConflict {
            branch: remote_branch.to_string(),
            message: out.stderr.trim().to_string(),
        });
    }
    Ok(())
}

/// Rebase onto a remote branch.
pub fn rebase(dir: &Path, remote_branch: &str) -> Result<(), GitrError> {
    let out = git(dir, &["rebase", remote_branch])?;
    if !out.success {
        // Abort on failure
        let _ = git(dir, &["rebase", "--abort"]);
        return Err(GitrError::MergeConflict {
            branch: remote_branch.to_string(),
            message: out.stderr.trim().to_string(),
        });
    }
    Ok(())
}

/// Push a branch to a remote.
pub fn push(dir: &Path, remote: &str, branch: &str) -> Result<(), GitrError> {
    git_ok(dir, &["push", remote, branch])?;
    Ok(())
}

/// Add a remote.
pub fn remote_add(dir: &Path, name: &str, url: &str) -> Result<(), GitrError> {
    let out = git(dir, &["remote", "add", name, url])?;
    if !out.success {
        // Remote might already exist
        if out.stderr.contains("already exists") {
            return Ok(());
        }
        return Err(GitrError::GitError {
            message: format!("failed to add remote {name}: {}", out.stderr.trim()),
        });
    }
    Ok(())
}

/// List remotes.
pub fn remote_list(dir: &Path) -> Result<Vec<String>, GitrError> {
    let stdout = git_ok(dir, &["remote"])?;
    Ok(stdout.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
}

/// Count commits that `a` is behind `b`: `git rev-list --count a..b`
pub fn rev_list_count(dir: &Path, a: &str, b: &str) -> Result<u32, GitrError> {
    let range = format!("{a}..{b}");
    let stdout = git_ok(dir, &["rev-list", "--count", &range])?;
    Ok(stdout.trim().parse().unwrap_or(0))
}

/// Get the current branch name.
pub fn current_branch(dir: &Path) -> Result<String, GitrError> {
    let stdout = git_ok(dir, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    Ok(stdout.trim().to_string())
}

/// Get the SHA of a ref.
pub fn rev_parse(dir: &Path, refspec: &str) -> Result<Option<String>, GitrError> {
    let out = git(dir, &["rev-parse", refspec])?;
    if out.success {
        Ok(Some(out.stdout.trim().to_string()))
    } else {
        Ok(None)
    }
}
