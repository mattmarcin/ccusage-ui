use crate::{ccusage::models::NormalizedUsageRequest, errors::AppError, settings::Settings};
use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::{process::Command, time};

const COMMAND_TIMEOUT_SECONDS: u64 = 30;
const STDERR_LIMIT: usize = 4_000;

#[derive(Debug, Clone)]
pub struct CcusageRun {
    pub stdout: String,
    pub stderr: Option<String>,
    pub command: Vec<String>,
    pub ccusage_version: Option<String>,
}

pub async fn run_daily_report(
    settings: &Settings,
    request: &NormalizedUsageRequest,
) -> Result<CcusageRun, AppError> {
    let executable = resolve_ccusage_path(settings).ok_or_else(|| AppError::NotInstalled {
        details: "Checked settings path, PATH, ~/.bun/bin, and common npm global locations."
            .to_string(),
    })?;
    let ccusage_version = ccusage_version(&executable).await.ok();
    let args = build_daily_args(settings, request);
    let output = run_command(&executable, &args).await?;

    Ok(CcusageRun {
        stdout: output.stdout,
        stderr: output.stderr,
        command: std::iter::once(executable.to_string_lossy().to_string())
            .chain(args)
            .collect(),
        ccusage_version,
    })
}

pub fn build_daily_args(settings: &Settings, request: &NormalizedUsageRequest) -> Vec<String> {
    let mut args = vec![
        "daily".to_string(),
        "--json".to_string(),
        "--timezone".to_string(),
        request.timezone.clone(),
    ];

    if settings.offline {
        args.push("--offline".to_string());
    }
    if let Some(since) = &request.since {
        args.push("--since".to_string());
        args.push(since.clone());
    }
    if let Some(until) = &request.until {
        args.push("--until".to_string());
        args.push(until.clone());
    }

    args
}

pub async fn ccusage_version(path: &Path) -> Result<String, AppError> {
    let output = run_command(path, &["--version".to_string()]).await?;
    Ok(output.stdout.trim().to_string())
}

pub fn resolve_ccusage_path(settings: &Settings) -> Option<PathBuf> {
    if let Some(path) = &settings.ccusage_path {
        if path.exists() {
            return Some(path.clone());
        }
    }

    find_on_path().or_else(find_known_install)
}

pub fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut out = value.chars().take(max_chars).collect::<String>();
    out.push_str("...");
    out
}

struct CapturedOutput {
    stdout: String,
    stderr: Option<String>,
}

async fn run_command(path: &Path, args: &[String]) -> Result<CapturedOutput, AppError> {
    let child = Command::new(path)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|err| AppError::Io {
            details: format!("{}: {}", path.display(), err),
        })?;

    let output = time::timeout(
        Duration::from_secs(COMMAND_TIMEOUT_SECONDS),
        child.wait_with_output(),
    )
    .await
    .map_err(|_| AppError::Timeout)?
    .map_err(|err| AppError::Io {
        details: format!("{}: {}", path.display(), err),
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(AppError::NonZeroExit {
            exit_code: output.status.code(),
            stderr: truncate(&stderr, STDERR_LIMIT),
        });
    }

    Ok(CapturedOutput {
        stdout,
        stderr: (!stderr.trim().is_empty()).then(|| truncate(&stderr, STDERR_LIMIT)),
    })
}

fn find_on_path() -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    for dir in env::split_paths(&path_var) {
        for name in executable_names() {
            let candidate = dir.join(name);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}

fn find_known_install() -> Option<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = env::var_os("USERPROFILE").or_else(|| env::var_os("HOME")) {
        dirs.push(PathBuf::from(home).join(".bun").join("bin"));
    }
    if let Some(appdata) = env::var_os("APPDATA") {
        dirs.push(PathBuf::from(appdata).join("npm"));
    }

    for dir in dirs {
        for name in executable_names() {
            let candidate = dir.join(name);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    None
}

fn executable_names() -> Vec<OsString> {
    if cfg!(windows) {
        vec![
            OsString::from("ccusage.exe"),
            OsString::from("ccusage.cmd"),
            OsString::from("ccusage.bat"),
            OsString::from("ccusage"),
        ]
    } else {
        vec![OsString::from("ccusage")]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ccusage::models::NormalizedUsageRequest;

    #[test]
    fn builds_args_without_shell_string() {
        let settings = Settings {
            offline: true,
            ..Settings::default()
        };
        let request = NormalizedUsageRequest {
            since: Some("2026-06-01".to_string()),
            until: Some("2026-06-24".to_string()),
            timezone: "America/Los_Angeles".to_string(),
            force_refresh: false,
        };

        assert_eq!(
            build_daily_args(&settings, &request),
            vec![
                "daily",
                "--json",
                "--timezone",
                "America/Los_Angeles",
                "--offline",
                "--since",
                "2026-06-01",
                "--until",
                "2026-06-24"
            ]
        );
    }

    #[test]
    fn omits_offline_when_disabled() {
        let settings = Settings {
            offline: false,
            ..Settings::default()
        };
        let request = NormalizedUsageRequest {
            since: None,
            until: None,
            timezone: "UTC".to_string(),
            force_refresh: false,
        };

        assert!(!build_daily_args(&settings, &request).contains(&"--offline".to_string()));
    }
}
