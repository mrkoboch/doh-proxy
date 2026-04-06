use std::path::Path;
use std::process::Command;

fn main() {
    // Tell cargo to rerun if frontend sources change
    println!("cargo:rerun-if-changed=../dns-dashboard-ui/src");
    println!("cargo:rerun-if-changed=../dns-dashboard-ui/index.html");
    println!("cargo:rerun-if-changed=../dns-dashboard-ui/vite.config.js");
    println!("cargo:rerun-if-changed=../dns-dashboard-ui/package.json");

    let (npm, node_bin_dir) = find_npm();

    println!("cargo:warning=Running npm run build (in dns-dashboard-ui/)");

    // Build PATH that includes the node bin dir so `node` is resolvable
    let path = match node_bin_dir {
        Some(ref dir) => {
            let existing = std::env::var("PATH").unwrap_or_default();
            format!("{dir}:{existing}")
        }
        None => std::env::var("PATH").unwrap_or_default(),
    };

    let status = Command::new(&npm)
        .args(["run", "build"])
        .current_dir("../dns-dashboard-ui")
        .env("PATH", path)
        .status()
        .unwrap_or_else(|e| panic!("Failed to run npm ({npm}): {e}"));

    if !status.success() {
        panic!("npm run build failed — check dns-dashboard-ui/ for errors");
    }
}

/// Returns `(npm_path, Option<node_bin_dir>)`.
fn find_npm() -> (String, Option<String>) {
    // Try PATH first
    if Command::new("npm").arg("--version").output().is_ok() {
        return ("npm".to_string(), None);
    }

    // Try NVM default location (~/.nvm/versions/node/<version>/bin/)
    if let Ok(home) = std::env::var("HOME") {
        let nvm_dir = format!("{home}/.nvm/versions/node");
        if let Ok(entries) = std::fs::read_dir(&nvm_dir) {
            let mut versions: Vec<_> = entries
                .flatten()
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .collect();
            versions.sort_by_key(|e| e.file_name());
            if let Some(latest) = versions.last() {
                let bin_dir = format!("{}/{}/bin", nvm_dir, latest.file_name().to_string_lossy());
                let npm = format!("{bin_dir}/npm");
                if Path::new(&npm).exists() {
                    return (npm, Some(bin_dir));
                }
            }
        }
    }

    panic!(
        "npm not found on PATH or in ~/.nvm. \
         Install Node.js (https://nodejs.org) before building dns-dashboard."
    );
}
