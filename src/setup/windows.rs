//! Windows `nbv setup`: append the nbv binary directory to the user's `PATH`
//! (HKCU environment) via PowerShell.
//!
//! The binary directory is passed to PowerShell through the `NBV_SETUP_DIR`
//! environment variable rather than interpolated into the script, so paths with
//! spaces or quotes need no escaping. The script reads its target variable and
//! scope from env too, defaulting to `('Path','User')` in production; tests
//! point it at a throwaway `Process`-scope variable so nothing is persisted.

use std::path::Path;
use std::process::Command;

use super::{binary_dir, confirm, path_already_includes};

/// PowerShell that appends `$env:NBV_SETUP_DIR` to a target environment
/// variable (default `Path`) in a target scope (default `User`), idempotently.
const PS_PERSIST: &str = "\
$d = $env:NBV_SETUP_DIR; \
$var = $env:NBV_SETUP_VAR; if (-not $var) { $var = 'Path' }; \
$scope = $env:NBV_SETUP_SCOPE; if (-not $scope) { $scope = 'User' }; \
$p = [Environment]::GetEnvironmentVariable($var, $scope); if ($null -eq $p) { $p = '' }; \
if (($p -split ';') -notcontains $d) { \
if ($p -ne '' -and -not $p.EndsWith(';')) { $p += ';' }; \
[Environment]::SetEnvironmentVariable($var, $p + $d, $scope) \
}";

fn persist_command(bin_dir: &Path) -> Command {
    let mut c = Command::new("powershell");
    c.args(["-NoProfile", "-NonInteractive", "-Command", PS_PERSIST]);
    c.env("NBV_SETUP_DIR", bin_dir);
    // NBV_SETUP_VAR / NBV_SETUP_SCOPE intentionally unset → defaults ('Path','User').
    c
}

fn print_manual_fallback(bin_dir: &Path) {
    eprintln!("Add it manually in PowerShell:");
    eprintln!(
        "    [Environment]::SetEnvironmentVariable('Path', [Environment]::GetEnvironmentVariable('Path','User') + ';{}', 'User')",
        bin_dir.display()
    );
}

pub fn run(yes: bool) -> i32 {
    let bin_dir = match binary_dir() {
        Some(p) => p,
        None => {
            eprintln!("nbv setup: could not determine the nbv binary directory");
            return 1;
        }
    };

    let path_env = std::env::var("PATH").unwrap_or_default();
    if path_already_includes(&path_env, &bin_dir) {
        println!("'{}' is already in PATH. Nothing to do.", bin_dir.display());
        return 0;
    }

    println!("nbv binary directory: {}", bin_dir.display());
    println!("target:               user environment variables (HKCU\\Environment)");
    println!();
    println!("This will append the directory to your user PATH via PowerShell.");
    println!();

    if !yes && !confirm() {
        println!("Aborted.");
        return 0;
    }

    match persist_command(&bin_dir).status() {
        Ok(s) if s.success() => {
            println!();
            println!("Added to your user PATH. Open a NEW terminal to pick it up.");
            println!();
            println!("To activate it in THIS terminal right now, run:");
            println!("    $env:Path += ';{}'", bin_dir.display());
            0
        }
        Ok(s) => {
            eprintln!(
                "nbv setup: failed to update user PATH (powershell exited with {}).",
                s
            );
            print_manual_fallback(&bin_dir);
            1
        }
        Err(e) => {
            eprintln!("nbv setup: failed to run powershell: {}", e);
            print_manual_fallback(&bin_dir);
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ps_persist_has_expected_shape() {
        assert!(PS_PERSIST.contains("SetEnvironmentVariable"));
        assert!(PS_PERSIST.contains("-notcontains"));
        assert!(PS_PERSIST.contains("'Path'"));
        assert!(PS_PERSIST.contains("'User'"));
        assert!(PS_PERSIST.contains("$env:NBV_SETUP_DIR"));
    }

    #[test]
    fn persist_command_uses_powershell_with_dir_env_and_default_scope() {
        let c = persist_command(Path::new(r"C:\nbv\bin"));
        assert_eq!(c.get_program().to_string_lossy(), "powershell");
        let args: Vec<_> = c
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            args,
            vec!["-NoProfile", "-NonInteractive", "-Command", PS_PERSIST]
        );
        // NBV_SETUP_DIR is set; VAR/SCOPE are NOT (production defaults).
        let envs: Vec<(String, Option<String>)> = c
            .get_envs()
            .map(|(k, v)| {
                (
                    k.to_string_lossy().into_owned(),
                    v.map(|v| v.to_string_lossy().into_owned()),
                )
            })
            .collect();
        assert!(envs
            .iter()
            .any(|(k, v)| k == "NBV_SETUP_DIR" && v.as_deref() == Some(r"C:\nbv\bin")));
        assert!(!envs.iter().any(|(k, _)| k == "NBV_SETUP_VAR"));
        assert!(!envs.iter().any(|(k, _)| k == "NBV_SETUP_SCOPE"));
    }

    // Smoke test: run the REAL PS_PERSIST against a throwaway Process-scope
    // variable (never persisted), then read it back in the same process to
    // prove PowerShell parses + executes the script and the append works.
    #[test]
    fn ps_persist_appends_in_process_scope() {
        let var = format!("NBV_TEST_PATH_{}", std::process::id());
        let dir = r"C:\nbv\test\bin";
        let script = format!(
            "{PS_PERSIST}; [Console]::Out.Write([Environment]::GetEnvironmentVariable($var, $scope))"
        );
        let out = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .env("NBV_SETUP_DIR", dir)
            .env("NBV_SETUP_VAR", &var)
            .env("NBV_SETUP_SCOPE", "Process")
            .output();
        // Skip gracefully if powershell is somehow unavailable.
        let Ok(out) = out else {
            return;
        };
        assert!(out.status.success(), "PS_PERSIST should execute cleanly");
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            stdout.contains(dir),
            "expected appended dir in readback, got: {stdout:?}"
        );
    }
}
