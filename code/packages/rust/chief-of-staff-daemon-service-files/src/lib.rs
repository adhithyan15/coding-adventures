//! Pure user-scoped service-definition renderers for the D18 Chief daemon.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::fmt::{self, Display, Formatter};
use std::path::{Component, Path};

/// Stable launchd label used by the D18 specification.
pub const LAUNCHD_LABEL: &str = "dev.chiefofstaff";
/// Per-user launchd definition path.
pub const LAUNCHD_INSTALL_PATH: &str = "~/Library/LaunchAgents/dev.chiefofstaff.plist";
/// Per-user systemd service name.
pub const SYSTEMD_UNIT_NAME: &str = "chief-of-staff.service";
/// Per-user systemd definition path.
pub const SYSTEMD_INSTALL_PATH: &str = "~/.config/systemd/user/chief-of-staff.service";
/// Task Scheduler registration name.
pub const WINDOWS_TASK_NAME: &str = r"\ChiefOfStaff\Daemon";
/// Per-user Task Scheduler XML path.
pub const WINDOWS_INSTALL_PATH: &str = r"%APPDATA%\ChiefOfStaff\Tasks\daemon.xml";

/// Platform whose native service manager consumes a rendered file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServicePlatform {
    /// macOS per-user launchd LaunchAgent.
    Launchd,
    /// Linux systemd user service.
    SystemdUser,
    /// Windows per-user Task Scheduler task.
    WindowsTaskScheduler,
}

/// A deterministic service definition ready for an authorized installer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceFile {
    /// Native service manager that consumes this file.
    pub platform: ServicePlatform,
    /// Documented per-user destination, expressed with the platform placeholder.
    pub install_path: &'static str,
    /// Complete plist, unit, or Task Scheduler XML contents.
    pub contents: String,
    /// Required Unix file mode, or `None` on Windows.
    pub mode: Option<u32>,
}

/// Stable validation failure that never includes caller-controlled path text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceFileError {
    /// The daemon executable path was not a normalized absolute platform path.
    InvalidExecutablePath,
    /// The configuration path was not a normalized absolute platform path.
    InvalidConfigPath,
}

impl Display for ServiceFileError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidExecutablePath => "service file: invalid executable path",
            Self::InvalidConfigPath => "service file: invalid config path",
        })
    }
}

impl std::error::Error for ServiceFileError {}

/// Render the macOS per-user LaunchAgent definition.
///
/// `launchd` receives a tokenized `ProgramArguments` array, starts the daemon at
/// login, restarts only after an unsuccessful exit, and sends SIGTERM during
/// logout or unload. The owner-only mode follows Apple's LaunchAgent guidance.
pub fn render_launchd(
    executable: &str,
    config_path: &str,
) -> Result<ServiceFile, ServiceFileError> {
    validate_unix_paths(executable, config_path)?;

    let executable = xml_escape(executable);
    let config_path = xml_escape(config_path);
    let contents = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{LAUNCHD_LABEL}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{executable}</string>
    <string>{config_path}</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <dict>
    <key>SuccessfulExit</key>
    <false/>
  </dict>
  <key>ThrottleInterval</key>
  <integer>10</integer>
  <key>ProcessType</key>
  <string>Background</string>
  <key>Umask</key>
  <integer>63</integer>
</dict>
</plist>
"#
    );

    Ok(ServiceFile {
        platform: ServicePlatform::Launchd,
        install_path: LAUNCHD_INSTALL_PATH,
        contents,
        mode: Some(0o600),
    })
}

/// Render the Linux per-user systemd service definition.
///
/// The service stays in the foreground, is restarted only after abnormal exit,
/// receives SIGTERM for cooperative shutdown, and applies an owner-only umask.
pub fn render_systemd_user(
    executable: &str,
    config_path: &str,
) -> Result<ServiceFile, ServiceFileError> {
    validate_unix_paths(executable, config_path)?;

    let command = format!(
        "{} {}",
        quote_systemd_word(executable),
        quote_systemd_word(config_path)
    );
    let contents = format!(
        "[Unit]\n\
Description=Chief of Staff orchestrator daemon\n\n\
[Service]\n\
Type=simple\n\
ExecStart={command}\n\
Restart=on-failure\n\
RestartSec=5s\n\
KillSignal=SIGTERM\n\
TimeoutStopSec=30s\n\
NoNewPrivileges=true\n\
UMask=0077\n\n\
[Install]\n\
WantedBy=default.target\n"
    );

    Ok(ServiceFile {
        platform: ServicePlatform::SystemdUser,
        install_path: SYSTEMD_INSTALL_PATH,
        contents,
        mode: Some(0o600),
    })
}

/// Render the Windows per-user Task Scheduler definition.
///
/// The task uses the current interactive token at least privilege, starts at
/// login, never overlaps itself, has no execution time limit, and retries a
/// failed daemon once per minute up to the schema's unsigned-byte maximum.
pub fn render_windows_task(
    executable: &str,
    config_path: &str,
) -> Result<ServiceFile, ServiceFileError> {
    validate_windows_path(executable).map_err(|()| ServiceFileError::InvalidExecutablePath)?;
    validate_windows_path(config_path).map_err(|()| ServiceFileError::InvalidConfigPath)?;

    let arguments = quote_windows_argument(config_path);
    let executable = xml_escape(executable);
    let arguments = xml_escape(&arguments);
    let contents = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Task version="1.3" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Author>Chief of Staff</Author>
    <URI>{WINDOWS_TASK_NAME}</URI>
    <Description>Chief of Staff orchestrator daemon</Description>
  </RegistrationInfo>
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
    </LogonTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>LeastPrivilege</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>true</AllowHardTerminate>
    <StartWhenAvailable>true</StartWhenAvailable>
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <Enabled>true</Enabled>
    <Hidden>false</Hidden>
    <RestartOnFailure>
      <Interval>PT1M</Interval>
      <Count>255</Count>
    </RestartOnFailure>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{executable}</Command>
      <Arguments>{arguments}</Arguments>
    </Exec>
  </Actions>
</Task>
"#
    );

    Ok(ServiceFile {
        platform: ServicePlatform::WindowsTaskScheduler,
        install_path: WINDOWS_INSTALL_PATH,
        contents,
        mode: None,
    })
}

fn validate_unix_paths(executable: &str, config_path: &str) -> Result<(), ServiceFileError> {
    validate_unix_path(executable).map_err(|()| ServiceFileError::InvalidExecutablePath)?;
    validate_unix_path(config_path).map_err(|()| ServiceFileError::InvalidConfigPath)
}

fn validate_unix_path(value: &str) -> Result<(), ()> {
    // POSIX absoluteness ("starts with `/`"), not `Path::is_absolute()`: this
    // renders launchd/systemd definitions from POSIX-style paths regardless of
    // which platform the crate itself was built and tested on, and `Path`'s
    // notion of "absolute" is platform-dependent — on Windows a bare `/…` path
    // has a root but no drive/UNC prefix, so `Path::is_absolute()` returns
    // `false` for exactly the paths this function must accept.
    if has_forbidden_text(value) || !value.starts_with('/') {
        return Err(());
    }
    if Path::new(value)
        .components()
        .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(());
    }
    Ok(())
}

fn validate_windows_path(value: &str) -> Result<(), ()> {
    if has_forbidden_text(value) || value.contains('"') || value.contains('/') {
        return Err(());
    }

    let drive_absolute = value
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_alphabetic)
        && value.as_bytes().get(1) == Some(&b':')
        && value.as_bytes().get(2) == Some(&b'\\');
    let unc_absolute = value.starts_with(r"\\")
        && value
            .trim_start_matches('\\')
            .split('\\')
            .filter(|part| !part.is_empty())
            .take(2)
            .count()
            == 2;
    if !drive_absolute && !unc_absolute {
        return Err(());
    }
    if value
        .split('\\')
        .any(|component| matches!(component, "." | ".."))
    {
        return Err(());
    }
    Ok(())
}

fn has_forbidden_text(value: &str) -> bool {
    value.is_empty() || value.chars().any(char::is_control)
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn quote_systemd_word(value: &str) -> String {
    let mut quoted = String::with_capacity(value.len() + 2);
    quoted.push('"');
    for character in value.chars() {
        match character {
            '\\' => quoted.push_str(r"\\"),
            '"' => quoted.push_str(r#"\""#),
            '$' => quoted.push_str("$$"),
            '%' => quoted.push_str("%%"),
            other => quoted.push(other),
        }
    }
    quoted.push('"');
    quoted
}

fn quote_windows_argument(value: &str) -> String {
    let mut quoted = String::with_capacity(value.len() + 2);
    quoted.push('"');
    let mut backslashes = 0usize;
    for character in value.chars() {
        match character {
            '\\' => backslashes += 1,
            '"' => {
                for _ in 0..(backslashes * 2 + 1) {
                    quoted.push('\\');
                }
                quoted.push('"');
                backslashes = 0;
            }
            other => {
                for _ in 0..backslashes {
                    quoted.push('\\');
                }
                backslashes = 0;
                quoted.push(other);
            }
        }
    }
    for _ in 0..(backslashes * 2) {
        quoted.push('\\');
    }
    quoted.push('"');
    quoted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launchd_is_owner_only_tokenized_and_restart_on_failure() {
        let service = render_launchd(
            "/Users/A & B/bin/chief-of-staff-daemon",
            "/Users/A & B/.chief-of-staff/config.toml",
        )
        .unwrap();

        assert_eq!(service.platform, ServicePlatform::Launchd);
        assert_eq!(service.install_path, LAUNCHD_INSTALL_PATH);
        assert_eq!(service.mode, Some(0o600));
        assert!(service.contents.contains("<string>/Users/A &amp; B/bin/"));
        assert!(service.contents.contains(
            "<key>KeepAlive</key>\n  <dict>\n    <key>SuccessfulExit</key>\n    <false/>"
        ));
        assert!(service.contents.contains("<key>RunAtLoad</key>\n  <true/>"));
        assert!(service
            .contents
            .contains("<key>Umask</key>\n  <integer>63</integer>"));
    }

    #[test]
    fn systemd_is_foreground_hardened_and_restart_on_failure() {
        let service = render_systemd_user(
            "/home/alice/bin/chief of staff $prod%1",
            "/home/alice/.chief-of-staff/config.toml",
        )
        .unwrap();

        assert_eq!(service.platform, ServicePlatform::SystemdUser);
        assert_eq!(service.install_path, SYSTEMD_INSTALL_PATH);
        assert!(service.contents.contains(
            "ExecStart=\"/home/alice/bin/chief of staff $$prod%%1\" \"/home/alice/.chief-of-staff/config.toml\""
        ));
        assert!(service.contents.contains("Type=simple\n"));
        assert!(service.contents.contains("Restart=on-failure\n"));
        assert!(service.contents.contains("KillSignal=SIGTERM\n"));
        assert!(service.contents.contains("WantedBy=default.target\n"));
    }

    #[test]
    fn windows_task_is_login_scoped_single_instance_and_restartable() {
        let service = render_windows_task(
            r"C:\Program Files\Chief & Staff\chief-of-staff-daemon.exe",
            r"C:\Users\Alice\AppData\Roaming\Chief of Staff\config.toml",
        )
        .unwrap();

        assert_eq!(service.platform, ServicePlatform::WindowsTaskScheduler);
        assert_eq!(service.install_path, WINDOWS_INSTALL_PATH);
        assert_eq!(service.mode, None);
        assert!(service.contents.contains("<LogonTrigger>"));
        assert!(service
            .contents
            .contains("<RunLevel>LeastPrivilege</RunLevel>"));
        assert!(service
            .contents
            .contains("<MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>"));
        assert!(service.contents.contains(
            "<RestartOnFailure>\n      <Interval>PT1M</Interval>\n      <Count>255</Count>"
        ));
        assert!(service.contents.contains(
            "<Command>C:\\Program Files\\Chief &amp; Staff\\chief-of-staff-daemon.exe</Command>"
        ));
        assert!(service.contents.contains(
            "<Arguments>&quot;C:\\Users\\Alice\\AppData\\Roaming\\Chief of Staff\\config.toml&quot;</Arguments>"
        ));
    }

    #[test]
    fn invalid_or_ambiguous_paths_are_rejected_without_echoing_them() {
        assert_eq!(
            render_launchd("relative/daemon", "/home/alice/config.toml"),
            Err(ServiceFileError::InvalidExecutablePath)
        );
        assert_eq!(
            render_systemd_user("/home/alice/../daemon", "/home/alice/config.toml"),
            Err(ServiceFileError::InvalidExecutablePath)
        );
        assert_eq!(
            render_windows_task(r"C:\Chief\chief-of-staff-daemon.exe", r"config.toml"),
            Err(ServiceFileError::InvalidConfigPath)
        );
        assert_eq!(
            render_windows_task(
                r"C:\Chief\chief-of-staff-daemon.exe",
                "C:\\Chief\\config.toml\nInjected"
            ),
            Err(ServiceFileError::InvalidConfigPath)
        );
        assert_eq!(
            ServiceFileError::InvalidConfigPath.to_string(),
            "service file: invalid config path"
        );
        assert_eq!(
            ServiceFileError::InvalidExecutablePath.to_string(),
            "service file: invalid executable path"
        );
        assert_eq!(
            render_windows_task(
                r"C:\Chief\..\chief-of-staff-daemon.exe",
                r"C:\Chief\config.toml"
            ),
            Err(ServiceFileError::InvalidExecutablePath)
        );
    }

    #[test]
    fn unc_paths_and_trailing_backslashes_are_quoted_correctly() {
        let service = render_windows_task(
            r"\\server\share\chief-of-staff-daemon.exe",
            "C:\\Chief Config\\",
        )
        .unwrap();

        assert!(service
            .contents
            .contains("<Arguments>&quot;C:\\Chief Config\\\\&quot;</Arguments>"));
    }

    #[test]
    fn platform_escaping_covers_native_metacharacters() {
        assert_eq!(xml_escape("<&>\"'"), "&lt;&amp;&gt;&quot;&apos;");
        assert_eq!(
            quote_systemd_word("/tmp/a\\b\"c$d%e"),
            r#""/tmp/a\\b\"c$$d%%e""#
        );
        assert_eq!(
            quote_windows_argument(r#"C:\before\"quoted"#),
            r#""C:\before\\\"quoted""#
        );
    }
}
