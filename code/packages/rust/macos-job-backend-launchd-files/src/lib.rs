//! # macos-job-backend-launchd-files
//!
//! This crate renders per-user `launchd` job definitions as plist text.
//!
//! The goal is not to execute installation directly. Instead, we produce a
//! deterministic [`os_job_core::InstallPlan`] that higher layers can inspect,
//! preview, diff, and eventually apply.
//!
//! ## Why a file-rendering backend?
//!
//! `launchd` is configured through plist files placed in known directories such
//! as `~/Library/LaunchAgents/`. That makes it a strong fit for a pure-Rust
//! backend:
//!
//! ```text
//! JobSpec ──► plist text ──► install plan ──► launchctl load/bootstrap
//! ```
//!
//! ## Supported trigger mapping
//!
//! - `interval`      → `StartInterval`
//! - `daily`         → `StartCalendarInterval`
//! - `weekly`        → array of `StartCalendarInterval` dictionaries
//! - `monthly`       → `StartCalendarInterval`
//! - `at_login`      → `RunAtLoad`
//!
//! ## Intentionally unsupported in this first pass
//!
//! - `once`
//!   `launchd` calendar schedules do not accept a year field, so an exact
//!   one-shot timestamp cannot be represented faithfully.
//! - `at_boot`
//!   This crate targets per-user LaunchAgents rather than system-wide
//!   LaunchDaemons.
//! - stdin payloads
//!   `launchd` can launch executables, but there is no clean native field for
//!   piping arbitrary stdin content without inventing a shell wrapper.

use std::fmt::Write;

use os_job_core::{
    BackendKind, InstallCommand, InstallFile, InstallPlan, JobBackend, JobError, JobSpec,
    JobTrigger, PermissionRequirement,
};

/// Render LaunchAgent plists and install plans.
#[derive(Debug, Default, Clone, Copy)]
pub struct LaunchdFileBackend;

impl LaunchdFileBackend {
    fn ensure_supported(&self, spec: &JobSpec) -> Result<(), JobError> {
        if spec.action.has_input() {
            return Err(JobError::UnsupportedAction {
                backend: BackendKind::Launchd,
                action: spec.action.kind_name().to_string(),
                reason: "launchd does not expose a native stdin field for scheduled jobs"
                    .to_string(),
            });
        }

        match &spec.trigger {
            JobTrigger::Once { .. } => Err(JobError::UnsupportedTrigger {
                backend: BackendKind::Launchd,
                trigger: spec.trigger.kind_name().to_string(),
                reason: "LaunchAgents do not support an exact year-qualified one-shot schedule"
                    .to_string(),
            }),
            JobTrigger::AtBoot => Err(JobError::UnsupportedTrigger {
                backend: BackendKind::Launchd,
                trigger: spec.trigger.kind_name().to_string(),
                reason: "this crate targets per-user LaunchAgents, not system LaunchDaemons"
                    .to_string(),
            }),
            JobTrigger::Interval {
                anchor: Some(_), ..
            } => Err(JobError::UnsupportedFeature {
                backend: BackendKind::Launchd,
                feature: "interval anchor".to_string(),
                reason:
                    "StartInterval repeats every N seconds but does not preserve a portable anchor"
                        .to_string(),
            }),
            _ => Ok(()),
        }
    }

    fn launch_agent_label(&self, spec: &JobSpec) -> String {
        format!("dev.codingadventures.chief-of-staff.{}", spec.job_id)
    }

    fn plist_path(&self, spec: &JobSpec) -> String {
        format!(
            "~/Library/LaunchAgents/{}.plist",
            self.launch_agent_label(spec)
        )
    }

    fn render_plist(&self, spec: &JobSpec) -> Result<String, JobError> {
        self.ensure_supported(spec)?;

        let label = self.launch_agent_label(spec);
        let command = spec.action.command_line();
        let mut plist = String::new();

        writeln!(&mut plist, r#"<?xml version="1.0" encoding="UTF-8"?>"#).unwrap();
        writeln!(
            &mut plist,
            r#"<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">"#
        )
        .unwrap();
        writeln!(&mut plist, r#"<plist version="1.0">"#).unwrap();
        writeln!(&mut plist, "<dict>").unwrap();

        write_key_string(&mut plist, "Label", &label);
        write_program_arguments(&mut plist, &command.program, &command.args);
        write_key_bool(
            &mut plist,
            "RunAtLoad",
            matches!(spec.trigger, JobTrigger::AtLogin),
        );
        write_key_bool(&mut plist, "Disabled", !spec.enabled);

        if let Some(working_directory) = &spec.working_directory {
            write_key_string(&mut plist, "WorkingDirectory", working_directory);
        }

        if let Some(stdout_path) = &spec.output_policy.stdout_path {
            write_key_string(&mut plist, "StandardOutPath", stdout_path);
        }

        if let Some(stderr_path) = &spec.output_policy.stderr_path {
            write_key_string(&mut plist, "StandardErrorPath", stderr_path);
        }

        if !spec.env.is_empty() {
            writeln!(&mut plist, "  <key>EnvironmentVariables</key>").unwrap();
            writeln!(&mut plist, "  <dict>").unwrap();
            for entry in &spec.env {
                write_key_string_with_indent(&mut plist, 4, &entry.key, &entry.value);
            }
            writeln!(&mut plist, "  </dict>").unwrap();
        }

        match &spec.trigger {
            JobTrigger::Interval { every_seconds, .. } => {
                write_key_integer(&mut plist, "StartInterval", *every_seconds);
            }
            JobTrigger::Daily { hour, minute } => {
                write_start_calendar_dict(
                    &mut plist,
                    &[("Hour", *hour as u32), ("Minute", *minute as u32)],
                );
            }
            JobTrigger::Weekly { days, hour, minute } => {
                writeln!(&mut plist, "  <key>StartCalendarInterval</key>").unwrap();
                writeln!(&mut plist, "  <array>").unwrap();
                let mut sorted_days = days.clone();
                sorted_days.sort();
                for day in sorted_days {
                    writeln!(&mut plist, "    <dict>").unwrap();
                    write_key_integer_with_indent(
                        &mut plist,
                        6,
                        "Weekday",
                        day.launchd_weekday() as u32,
                    );
                    write_key_integer_with_indent(&mut plist, 6, "Hour", *hour as u32);
                    write_key_integer_with_indent(&mut plist, 6, "Minute", *minute as u32);
                    writeln!(&mut plist, "    </dict>").unwrap();
                }
                writeln!(&mut plist, "  </array>").unwrap();
            }
            JobTrigger::Monthly { day, hour, minute } => {
                write_start_calendar_dict(
                    &mut plist,
                    &[
                        ("Day", *day as u32),
                        ("Hour", *hour as u32),
                        ("Minute", *minute as u32),
                    ],
                );
            }
            JobTrigger::AtLogin => {}
            JobTrigger::Once { .. } | JobTrigger::AtBoot => {
                unreachable!("checked in ensure_supported")
            }
        }

        writeln!(&mut plist, "</dict>").unwrap();
        writeln!(&mut plist, "</plist>").unwrap();
        Ok(plist)
    }
}

impl JobBackend for LaunchdFileBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Launchd
    }

    fn install_plan(&self, spec: &JobSpec) -> Result<InstallPlan, JobError> {
        spec.validate().into_result()?;
        let plist = self.render_plist(spec)?;
        let plist_path = self.plist_path(spec);

        Ok(InstallPlan {
            backend: BackendKind::Launchd,
            files_to_write: vec![InstallFile {
                path: plist_path.clone(),
                contents: plist,
                mode: Some(0o644),
                reason: "LaunchAgent plist consumed by launchd".to_string(),
            }],
            commands_to_run: if spec.enabled {
                vec![
                    InstallCommand {
                        program: "launchctl".to_string(),
                        args: vec!["unload".to_string(), plist_path.clone()],
                        description:
                            "Unload any previous definition before reloading the new LaunchAgent"
                                .to_string(),
                    },
                    InstallCommand {
                        program: "launchctl".to_string(),
                        args: vec!["load".to_string(), plist_path.clone()],
                        description: "Load the LaunchAgent into the current user launchd domain"
                            .to_string(),
                    },
                ]
            } else {
                vec![InstallCommand {
                    program: "launchctl".to_string(),
                    args: vec!["unload".to_string(), plist_path.clone()],
                    description: "Ensure the disabled LaunchAgent is not currently loaded"
                        .to_string(),
                }]
            },
            permissions_needed: vec![PermissionRequirement {
                scope: "filesystem".to_string(),
                detail: "Write to ~/Library/LaunchAgents and invoke launchctl for the current user"
                    .to_string(),
            }],
        })
    }
}

fn write_start_calendar_dict(plist: &mut String, entries: &[(&str, u32)]) {
    writeln!(plist, "  <key>StartCalendarInterval</key>").unwrap();
    writeln!(plist, "  <dict>").unwrap();
    for (key, value) in entries {
        write_key_integer_with_indent(plist, 4, key, *value);
    }
    writeln!(plist, "  </dict>").unwrap();
}

fn write_program_arguments(plist: &mut String, program: &str, args: &[String]) {
    writeln!(plist, "  <key>ProgramArguments</key>").unwrap();
    writeln!(plist, "  <array>").unwrap();
    writeln!(plist, "    <string>{}</string>", xml_escape(program)).unwrap();
    for arg in args {
        writeln!(plist, "    <string>{}</string>", xml_escape(arg)).unwrap();
    }
    writeln!(plist, "  </array>").unwrap();
}

fn write_key_string(plist: &mut String, key: &str, value: &str) {
    write_key_string_with_indent(plist, 2, key, value);
}

fn write_key_string_with_indent(plist: &mut String, indent: usize, key: &str, value: &str) {
    let prefix = " ".repeat(indent);
    writeln!(plist, "{prefix}<key>{}</key>", xml_escape(key)).unwrap();
    writeln!(plist, "{prefix}<string>{}</string>", xml_escape(value)).unwrap();
}

fn write_key_integer(plist: &mut String, key: &str, value: u32) {
    write_key_integer_with_indent(plist, 2, key, value);
}

fn write_key_integer_with_indent(plist: &mut String, indent: usize, key: &str, value: u32) {
    let prefix = " ".repeat(indent);
    writeln!(plist, "{prefix}<key>{}</key>", xml_escape(key)).unwrap();
    writeln!(plist, "{prefix}<integer>{value}</integer>").unwrap();
}

fn write_key_bool(plist: &mut String, key: &str, value: bool) {
    writeln!(plist, "  <key>{}</key>", xml_escape(key)).unwrap();
    writeln!(plist, "  <{} />", if value { "true" } else { "false" }).unwrap();
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use os_job_core::{
        ConcurrencyPolicy, EnvironmentEntry, JobAction, JobTrigger, OutputPolicy, RetryPolicy,
        Weekday,
    };

    use super::*;

    fn weekly_job() -> JobSpec {
        JobSpec {
            job_id: "weekly-digest".to_string(),
            name: "Weekly Digest".to_string(),
            description: "Send an end-of-week executive digest".to_string(),
            action: JobAction::Command {
                program: "/usr/local/bin/chief-of-staff".to_string(),
                args: vec![
                    "digest".to_string(),
                    "--audience".to_string(),
                    "exec".to_string(),
                ],
                input: None,
            },
            trigger: JobTrigger::Weekly {
                days: vec![Weekday::Monday, Weekday::Friday],
                hour: 8,
                minute: 30,
            },
            concurrency_policy: ConcurrencyPolicy::Skip,
            retry_policy: RetryPolicy::default(),
            timeout_seconds: Some(300),
            env: vec![EnvironmentEntry {
                key: "COS_PROFILE".to_string(),
                value: "prod".to_string(),
            }],
            working_directory: Some("/Users/example/chief-of-staff".to_string()),
            output_policy: OutputPolicy {
                stdout_path: Some("/tmp/weekly-digest.log".to_string()),
                stderr_path: Some("/tmp/weekly-digest.err".to_string()),
                append: true,
            },
            enabled: true,
        }
    }

    #[test]
    fn weekly_plan_renders_multiple_calendar_entries() {
        let backend = LaunchdFileBackend;
        let plan = backend
            .install_plan(&weekly_job())
            .expect("launchd plan should render");
        let plist = &plan.files_to_write[0].contents;

        assert_eq!(plan.backend, BackendKind::Launchd);
        assert!(plist.contains("<key>StartCalendarInterval</key>"));
        assert!(plist.contains("<key>Weekday</key>\n      <integer>1</integer>"));
        assert!(plist.contains("<key>Weekday</key>\n      <integer>5</integer>"));
        assert!(plist.contains("<key>RunAtLoad</key>\n  <false />"));
    }

    #[test]
    fn once_trigger_is_rejected() {
        let mut job = weekly_job();
        job.trigger = JobTrigger::Once {
            at: os_job_core::DateTimeParts {
                year: 2026,
                month: 4,
                day: 17,
                hour: 9,
                minute: 0,
                second: 0,
            },
        };

        let error = LaunchdFileBackend
            .install_plan(&job)
            .expect_err("once schedules should be rejected for launchd");

        assert!(matches!(error, JobError::UnsupportedTrigger { .. }));
    }
}
