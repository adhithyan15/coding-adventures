//! # linux-job-backend-systemd-files
//!
//! This crate renders `systemd --user` unit files from portable [`os_job_core::JobSpec`]
//! values.
//!
//! Linux gives us two useful building blocks:
//!
//! - a `.service` unit that says *what to execute*
//! - a `.timer` unit that says *when to execute it*
//!
//! Splitting the rendering this way mirrors how `systemd` itself thinks:
//!
//! ```text
//! job action  ──► foo.service
//! job trigger ──► foo.timer
//! ```
//!
//! That separation is nice for Chief of Staff because it means `run_now` can
//! eventually target the service directly while recurring schedules still use
//! the timer.

use std::fmt::Write;

use os_job_core::{
    BackendKind, InstallCommand, InstallFile, InstallPlan, JobBackend, JobError, JobSpec,
    JobTrigger, PermissionRequirement,
};

/// Render user-scoped `systemd` unit files.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemdUserFileBackend;

impl SystemdUserFileBackend {
    fn service_name(&self, spec: &JobSpec) -> String {
        format!("chief-of-staff-{}", spec.job_id)
    }

    fn service_unit_path(&self, spec: &JobSpec) -> String {
        format!("~/.config/systemd/user/{}.service", self.service_name(spec))
    }

    fn timer_unit_path(&self, spec: &JobSpec) -> String {
        format!("~/.config/systemd/user/{}.timer", self.service_name(spec))
    }

    fn ensure_supported(&self, spec: &JobSpec) -> Result<(), JobError> {
        if spec.action.has_input() {
            return Err(JobError::UnsupportedAction {
                backend: BackendKind::SystemdUser,
                action: spec.action.kind_name().to_string(),
                reason: "systemd services do not expose a native stdin payload for timers"
                    .to_string(),
            });
        }

        match &spec.trigger {
            JobTrigger::AtBoot => Err(JobError::UnsupportedTrigger {
                backend: BackendKind::SystemdUser,
                trigger: spec.trigger.kind_name().to_string(),
                reason: "user managers start at login, so a true machine-boot trigger requires a system service"
                    .to_string(),
            }),
            JobTrigger::Interval {
                anchor: Some(_), ..
            } => Err(JobError::UnsupportedFeature {
                backend: BackendKind::SystemdUser,
                feature: "interval anchor".to_string(),
                reason: "this first pass uses monotonic timers, which do not preserve a portable anchor"
                    .to_string(),
            }),
            _ => Ok(()),
        }
    }

    fn render_service(&self, spec: &JobSpec) -> String {
        let command = spec.action.command_line();
        let mut unit = String::new();

        writeln!(&mut unit, "[Unit]").unwrap();
        writeln!(&mut unit, "Description={}", escape_unit_value(&spec.name)).unwrap();
        writeln!(&mut unit).unwrap();

        writeln!(&mut unit, "[Service]").unwrap();
        writeln!(&mut unit, "Type=oneshot").unwrap();
        writeln!(
            &mut unit,
            "ExecStart={}",
            quote_command_line(&command.program, &command.args)
        )
        .unwrap();

        if let Some(working_directory) = &spec.working_directory {
            writeln!(
                &mut unit,
                "WorkingDirectory={}",
                escape_unit_value(working_directory)
            )
            .unwrap();
        }

        if let Some(timeout_seconds) = spec.timeout_seconds {
            writeln!(&mut unit, "TimeoutStartSec={timeout_seconds}").unwrap();
        }

        if let Some(stdout_path) = &spec.output_policy.stdout_path {
            writeln!(
                &mut unit,
                "StandardOutput={}:{}",
                if spec.output_policy.append {
                    "append"
                } else {
                    "file"
                },
                escape_unit_value(stdout_path)
            )
            .unwrap();
        }

        if let Some(stderr_path) = &spec.output_policy.stderr_path {
            writeln!(
                &mut unit,
                "StandardError={}:{}",
                if spec.output_policy.append {
                    "append"
                } else {
                    "file"
                },
                escape_unit_value(stderr_path)
            )
            .unwrap();
        }

        for env in &spec.env {
            writeln!(
                &mut unit,
                "Environment=\"{}={}\"",
                escape_systemd_quoted(&env.key),
                escape_systemd_quoted(&env.value)
            )
            .unwrap();
        }

        if matches!(spec.trigger, JobTrigger::AtLogin) {
            writeln!(&mut unit).unwrap();
            writeln!(&mut unit, "[Install]").unwrap();
            writeln!(&mut unit, "WantedBy=default.target").unwrap();
        }

        unit
    }

    fn render_timer(&self, spec: &JobSpec) -> Option<Result<String, JobError>> {
        let mut unit = String::new();

        match &spec.trigger {
            JobTrigger::AtLogin => return None,
            JobTrigger::AtBoot => {
                return Some(Err(JobError::UnsupportedTrigger {
                    backend: BackendKind::SystemdUser,
                    trigger: spec.trigger.kind_name().to_string(),
                    reason: "user managers do not own true boot scheduling".to_string(),
                }))
            }
            JobTrigger::Interval {
                every_seconds: _,
                anchor: Some(_),
            } => {
                return Some(Err(JobError::UnsupportedFeature {
                    backend: BackendKind::SystemdUser,
                    feature: "interval anchor".to_string(),
                    reason: "monotonic timers in this crate use relative time only".to_string(),
                }))
            }
            _ => {}
        }

        writeln!(&mut unit, "[Unit]").unwrap();
        writeln!(
            &mut unit,
            "Description=Timer for {}",
            escape_unit_value(&spec.name)
        )
        .unwrap();
        writeln!(&mut unit).unwrap();
        writeln!(&mut unit, "[Timer]").unwrap();
        writeln!(&mut unit, "Unit={}.service", self.service_name(spec)).unwrap();
        writeln!(
            &mut unit,
            "Persistent={}",
            if spec.enabled { "true" } else { "false" }
        )
        .unwrap();

        match &spec.trigger {
            JobTrigger::Once { at } => {
                writeln!(&mut unit, "OnCalendar={}", at.to_systemd_calendar()).unwrap();
            }
            JobTrigger::Interval {
                anchor: Some(_), ..
            } => unreachable!("handled above"),
            JobTrigger::Interval {
                every_seconds,
                anchor: None,
            } => {
                writeln!(&mut unit, "OnStartupSec={every_seconds}s").unwrap();
                writeln!(&mut unit, "OnUnitActiveSec={every_seconds}s").unwrap();
                writeln!(&mut unit, "AccuracySec=1s").unwrap();
            }
            JobTrigger::Daily { hour, minute } => {
                writeln!(&mut unit, "OnCalendar=*-*-* {:02}:{:02}:00", hour, minute).unwrap();
            }
            JobTrigger::Weekly { days, hour, minute } => {
                let mut sorted_days = days.clone();
                sorted_days.sort();
                for day in sorted_days {
                    writeln!(
                        &mut unit,
                        "OnCalendar={} *-*-* {:02}:{:02}:00",
                        day.systemd_name(),
                        hour,
                        minute
                    )
                    .unwrap();
                }
            }
            JobTrigger::Monthly { day, hour, minute } => {
                writeln!(
                    &mut unit,
                    "OnCalendar=*-*-{:02} {:02}:{:02}:00",
                    day, hour, minute
                )
                .unwrap();
            }
            JobTrigger::AtLogin | JobTrigger::AtBoot => unreachable!("handled above"),
        }

        writeln!(&mut unit).unwrap();
        writeln!(&mut unit, "[Install]").unwrap();
        writeln!(&mut unit, "WantedBy=timers.target").unwrap();

        Some(Ok(unit))
    }
}

impl JobBackend for SystemdUserFileBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::SystemdUser
    }

    fn install_plan(&self, spec: &JobSpec) -> Result<InstallPlan, JobError> {
        spec.validate().into_result()?;
        self.ensure_supported(spec)?;

        let service_contents = self.render_service(spec);
        let mut files_to_write = vec![InstallFile {
            path: self.service_unit_path(spec),
            contents: service_contents,
            mode: Some(0o644),
            reason: "systemd user service unit describing the job action".to_string(),
        }];

        let mut commands_to_run = vec![InstallCommand {
            program: "systemctl".to_string(),
            args: vec!["--user".to_string(), "daemon-reload".to_string()],
            description: "Reload the user systemd manager after writing unit files".to_string(),
        }];

        if let Some(timer) = self.render_timer(spec) {
            files_to_write.push(InstallFile {
                path: self.timer_unit_path(spec),
                contents: timer?,
                mode: Some(0o644),
                reason: "systemd user timer unit describing the job schedule".to_string(),
            });
            if spec.enabled {
                commands_to_run.push(InstallCommand {
                    program: "systemctl".to_string(),
                    args: vec![
                        "--user".to_string(),
                        "enable".to_string(),
                        "--now".to_string(),
                        format!("{}.timer", self.service_name(spec)),
                    ],
                    description: "Enable and start the timer unit for recurring execution"
                        .to_string(),
                });
            }
        } else {
            if spec.enabled {
                commands_to_run.push(InstallCommand {
                    program: "systemctl".to_string(),
                    args: vec![
                        "--user".to_string(),
                        "enable".to_string(),
                        "--now".to_string(),
                        format!("{}.service", self.service_name(spec)),
                    ],
                    description: "Enable the service so it runs when the user manager starts"
                        .to_string(),
                });
            }
        }

        Ok(InstallPlan {
            backend: BackendKind::SystemdUser,
            files_to_write,
            commands_to_run,
            permissions_needed: vec![PermissionRequirement {
                scope: "filesystem".to_string(),
                detail: "Write to ~/.config/systemd/user and invoke systemctl --user".to_string(),
            }],
        })
    }
}

fn quote_command_line(program: &str, args: &[String]) -> String {
    let mut rendered = Vec::with_capacity(args.len() + 1);
    rendered.push(quote_systemd_word(program));
    rendered.extend(args.iter().map(|arg| quote_systemd_word(arg)));
    rendered.join(" ")
}

fn quote_systemd_word(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

fn escape_unit_value(value: &str) -> String {
    value.replace('\n', " ")
}

fn escape_systemd_quoted(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use os_job_core::{ConcurrencyPolicy, JobAction, OutputPolicy, RetryPolicy, Weekday};

    use super::*;

    fn daily_job(trigger: JobTrigger) -> JobSpec {
        JobSpec {
            job_id: "context-compact".to_string(),
            name: "Context Compact".to_string(),
            description: "Compact long-running conversation state".to_string(),
            action: JobAction::Command {
                program: "/usr/local/bin/chief-of-staff".to_string(),
                args: vec!["compact-context".to_string()],
                input: None,
            },
            trigger,
            concurrency_policy: ConcurrencyPolicy::Replace,
            retry_policy: RetryPolicy::default(),
            timeout_seconds: Some(120),
            env: Vec::new(),
            working_directory: Some("/srv/chief-of-staff".to_string()),
            output_policy: OutputPolicy {
                stdout_path: Some("/tmp/context-compact.log".to_string()),
                stderr_path: None,
                append: true,
            },
            enabled: true,
        }
    }

    #[test]
    fn daily_schedule_renders_service_and_timer() {
        let plan = SystemdUserFileBackend
            .install_plan(&daily_job(JobTrigger::Daily {
                hour: 2,
                minute: 45,
            }))
            .expect("daily timer should render");

        assert_eq!(plan.files_to_write.len(), 2);
        let timer = &plan.files_to_write[1].contents;
        assert!(timer.contains("OnCalendar=*-*-* 02:45:00"));
        assert!(timer.contains("WantedBy=timers.target"));
    }

    #[test]
    fn weekly_schedule_emits_multiple_oncalendar_lines() {
        let plan = SystemdUserFileBackend
            .install_plan(&daily_job(JobTrigger::Weekly {
                days: vec![Weekday::Monday, Weekday::Wednesday],
                hour: 9,
                minute: 15,
            }))
            .expect("weekly timer should render");

        let timer = &plan.files_to_write[1].contents;
        assert!(timer.contains("OnCalendar=Mon *-*-* 09:15:00"));
        assert!(timer.contains("OnCalendar=Wed *-*-* 09:15:00"));
    }

    #[test]
    fn login_trigger_uses_install_section_on_service() {
        let plan = SystemdUserFileBackend
            .install_plan(&daily_job(JobTrigger::AtLogin))
            .expect("at-login service should render");

        assert_eq!(plan.files_to_write.len(), 1);
        let service = &plan.files_to_write[0].contents;
        assert!(service.contains("[Install]"));
        assert!(service.contains("WantedBy=default.target"));
    }

    #[test]
    fn boot_trigger_is_rejected_for_user_manager() {
        let error = SystemdUserFileBackend
            .install_plan(&daily_job(JobTrigger::AtBoot))
            .expect_err("boot triggers should be rejected for user systemd");

        assert!(matches!(error, JobError::UnsupportedTrigger { .. }));
    }
}
