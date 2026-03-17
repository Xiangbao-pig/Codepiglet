use std::path::Path;

pub struct GitInfo {
    pub branch: Option<String>,
    pub dirty_count: u32,
}

/// Reads git state directly from the .git directory — no extension needed.
pub fn read_git_state(workspace: &Path) -> GitInfo {
    let branch = read_branch(workspace);
    let dirty_count = count_dirty(workspace);
    GitInfo {
        branch,
        dirty_count,
    }
}

fn read_branch(workspace: &Path) -> Option<String> {
    let head = std::fs::read_to_string(workspace.join(".git/HEAD")).ok()?;
    let head = head.trim();
    if let Some(refname) = head.strip_prefix("ref: refs/heads/") {
        Some(refname.to_string())
    } else {
        // Detached HEAD — return short hash
        Some(head.chars().take(8).collect())
    }
}

fn count_dirty(workspace: &Path) -> u32 {
    // Fast: use git status --porcelain via Command (avoids heavy gix dep for this)
    let output = std::process::Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=no"])
        .current_dir(workspace)
        .output();

    match output {
        Ok(o) if o.status.success() => {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter(|l| !l.is_empty())
                .count() as u32
        }
        _ => 0,
    }
}
