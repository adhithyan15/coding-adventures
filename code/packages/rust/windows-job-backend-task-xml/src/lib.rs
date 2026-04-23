//! # windows-job-backend-task-xml
//!
//! This crate renders Windows Task Scheduler definitions as XML plus the
//! `schtasks` commands needed to register them.
//!
//! The Windows scheduler is more schema-heavy than `launchd` or `systemd`, but
//! the same high-level idea applies:
//!
//! ```text
//! JobSpec ──► Task Scheduler XML ──► schtasks /Create /XML
//! ```
//!
//! A pure XML backend is especially useful here because it lets us keep the
//! authoring surface portable while leaving room for a future COM-backed native
//! accelerator crate.

use std::fmt::Write;

use os_job_core::{
    BackendKind, ConcurrencyPolicy, InstallCommand, InstallFile, InstallPlan, JobBackend, JobError,
    JobSpec, JobTrigger, PermissionRequirement,
};

/// Render Task Scheduler XML and install plans.
#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsTaskSchedulerXmlBackend;

impl WindowsTaskSchedulerXmlBackend {
    fn xml_path(&self, spec: &JobSpec) -> String {
        format!(r#"%APPDATA%\ChiefOfStaff\Tasks\{}.xml"#, spec.job_id)
    }

    fn task_name(&self, spec: &JobSpec) -> String {
        format!(r#"\ChiefOfStaff\{}"#, spec.job_id)
    }

    fn ensure_supported(&self, spec: &JobSpec) -> Result<(), JobError> {
        if spec.action.has_input() {
            return Err(JobError::UnsupportedAction {
                backend: BackendKind::WindowsTaskScheduler,
                action: spec.action.kind_name().to_string(),
                reason: "Task Scheduler does not expose a native stdin payload for Exec actions"
                    .to_string(),
            });
        }

        if !spec.env.is_empty() {
            return Err(JobError::UnsupportedFeature {
                backend: BackendKind::WindowsTaskScheduler,
                feature: "environment injection".to_string(),
                reason: "the pure XML backend rejects env injection until a shell-free launcher exists"
                    .to_string(),
            });
        }

        if let JobTrigger::Interval { every_seconds, .. } = spec.trigger {
            if every_seconds < 60 {
                return Err(JobError::UnsupportedTrigger {
                    backend: BackendKind::WindowsTaskScheduler,
                    trigger: "interval".to_string(),
                    reason: "Task Scheduler repetition intervals must be at least 60 seconds"
                        .to_string(),
                });
            }
        }

        Ok(())
    }

    fn render_xml(&self, spec: &JobSpec) -> Result<String, JobError> {
        self.ensure_supported(spec)?;

        let mut xml = String::new();
        let execution_limit = duration_xml(spec.timeout_seconds.unwrap_or(0));
        let (command, arguments) = executable_payload(spec);

        writeln!(&mut xml, r#"<?xml version="1.0" encoding="UTF-8"?>"#).unwrap();
        writeln!(
            &mut xml,
            r#"<Task version="1.3" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">"#
        )
        .unwrap();
        writeln!(&mut xml, "  <RegistrationInfo>").unwrap();
        writeln!(&mut xml, "    <Author>Chief of Staff</Author>").unwrap();
        writeln!(
            &mut xml,
            "    <URI>{}</URI>",
            xml_escape(&self.task_name(spec))
        )
        .unwrap();
        writeln!(
            &mut xml,
            "    <Description>{}</Description>",
            xml_escape(&spec.description)
        )
        .unwrap();
        writeln!(&mut xml, "  </RegistrationInfo>").unwrap();

        writeln!(&mut xml, "  <Triggers>").unwrap();
        render_trigger(&mut xml, spec)?;
        writeln!(&mut xml, "  </Triggers>").unwrap();

        writeln!(&mut xml, "  <Principals>").unwrap();
        writeln!(&mut xml, "    <Principal id=\"Author\">").unwrap();
        writeln!(&mut xml, "      <LogonType>InteractiveToken</LogonType>").unwrap();
        writeln!(&mut xml, "      <RunLevel>LeastPrivilege</RunLevel>").unwrap();
        writeln!(&mut xml, "    </Principal>").unwrap();
        writeln!(&mut xml, "  </Principals>").unwrap();

        writeln!(&mut xml, "  <Settings>").unwrap();
        writeln!(
            &mut xml,
            "    <MultipleInstancesPolicy>{}</MultipleInstancesPolicy>",
            concurrency_policy_xml(spec.concurrency_policy)
        )
        .unwrap();
        writeln!(
            &mut xml,
            "    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>"
        )
        .unwrap();
        writeln!(
            &mut xml,
            "    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>"
        )
        .unwrap();
        writeln!(
            &mut xml,
            "    <AllowHardTerminate>true</AllowHardTerminate>"
        )
        .unwrap();
        writeln!(
            &mut xml,
            "    <StartWhenAvailable>true</StartWhenAvailable>"
        )
        .unwrap();
        writeln!(
            &mut xml,
            "    <Enabled>{}</Enabled>",
            if spec.enabled { "true" } else { "false" }
        )
        .unwrap();
        writeln!(&mut xml, "    <Hidden>false</Hidden>").unwrap();
        writeln!(
            &mut xml,
            "    <AllowStartOnDemand>true</AllowStartOnDemand>"
        )
        .unwrap();
        writeln!(
            &mut xml,
            "    <ExecutionTimeLimit>{execution_limit}</ExecutionTimeLimit>"
        )
        .unwrap();
        writeln!(&mut xml, "    <Priority>7</Priority>").unwrap();
        writeln!(&mut xml, "  </Settings>").unwrap();

        writeln!(&mut xml, "  <Actions Context=\"Author\">").unwrap();
        writeln!(&mut xml, "    <Exec>").unwrap();
        writeln!(
            &mut xml,
            "      <Command>{}</Command>",
            xml_escape(&command)
        )
        .unwrap();
        if !arguments.is_empty() {
            writeln!(
                &mut xml,
                "      <Arguments>{}</Arguments>",
                xml_escape(&arguments)
            )
            .unwrap();
        }
        if let Some(working_directory) = &spec.working_directory {
            writeln!(
                &mut xml,
                "      <WorkingDirectory>{}</WorkingDirectory>",
                xml_escape(working_directory)
            )
            .unwrap();
        }
        writeln!(&mut xml, "    </Exec>").unwrap();
        writeln!(&mut xml, "  </Actions>").unwrap();
        writeln!(&mut xml, "</Task>").unwrap();

        Ok(xml)
    }
}

impl JobBackend for WindowsTaskSchedulerXmlBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::WindowsTaskScheduler
    }

    fn install_plan(&self, spec: &JobSpec) -> Result<InstallPlan, JobError> {
        spec.validate().into_result()?;
        let xml = self.render_xml(spec)?;
        let xml_path = self.xml_path(spec);

        Ok(InstallPlan {
            backend: BackendKind::WindowsTaskScheduler,
            files_to_write: vec![InstallFile {
                path: xml_path.clone(),
                contents: xml,
                mode: None,
                reason: "Task Scheduler XML consumed by schtasks.exe".to_string(),
            }],
            commands_to_run: vec![InstallCommand {
                program: "schtasks".to_string(),
                args: vec![
                    "/Create".to_string(),
                    "/TN".to_string(),
                    self.task_name(spec),
                    "/XML".to_string(),
                    xml_path,
                    "/F".to_string(),
                ],
                description: "Register or replace the Windows scheduled task from XML".to_string(),
            }],
            permissions_needed: vec![PermissionRequirement {
                scope: "scheduler".to_string(),
                detail: "Write an XML definition and register it through schtasks.exe".to_string(),
            }],
        })
    }
}

fn render_trigger(xml: &mut String, spec: &JobSpec) -> Result<(), JobError> {
    match &spec.trigger {
        JobTrigger::Once { at } => {
            writeln!(xml, "    <TimeTrigger>").unwrap();
            writeln!(xml, "      <Enabled>true</Enabled>").unwrap();
            writeln!(
                xml,
                "      <StartBoundary>{}</StartBoundary>",
                at.to_iso8601_local()
            )
            .unwrap();
            writeln!(xml, "    </TimeTrigger>").unwrap();
        }
        JobTrigger::Interval {
            every_seconds,
            anchor,
        } => {
            writeln!(xml, "    <TimeTrigger>").unwrap();
            writeln!(xml, "      <Enabled>true</Enabled>").unwrap();
            writeln!(
                xml,
                "      <StartBoundary>{}</StartBoundary>",
                anchor
                    .unwrap_or(os_job_core::DateTimeParts {
                        year: 2024,
                        month: 1,
                        day: 1,
                        hour: 0,
                        minute: 0,
                        second: 0,
                    })
                    .to_iso8601_local()
            )
            .unwrap();
            writeln!(xml, "      <Repetition>").unwrap();
            writeln!(
                xml,
                "        <Interval>{}</Interval>",
                duration_xml(*every_seconds)
            )
            .unwrap();
            writeln!(xml, "      </Repetition>").unwrap();
            writeln!(xml, "    </TimeTrigger>").unwrap();
        }
        JobTrigger::Daily { hour, minute } => {
            writeln!(xml, "    <CalendarTrigger>").unwrap();
            writeln!(xml, "      <Enabled>true</Enabled>").unwrap();
            writeln!(
                xml,
                "      <StartBoundary>2024-01-01T{:02}:{:02}:00</StartBoundary>",
                hour, minute
            )
            .unwrap();
            writeln!(xml, "      <ScheduleByDay>").unwrap();
            writeln!(xml, "        <DaysInterval>1</DaysInterval>").unwrap();
            writeln!(xml, "      </ScheduleByDay>").unwrap();
            writeln!(xml, "    </CalendarTrigger>").unwrap();
        }
        JobTrigger::Weekly { days, hour, minute } => {
            writeln!(xml, "    <CalendarTrigger>").unwrap();
            writeln!(xml, "      <Enabled>true</Enabled>").unwrap();
            writeln!(
                xml,
                "      <StartBoundary>2024-01-01T{:02}:{:02}:00</StartBoundary>",
                hour, minute
            )
            .unwrap();
            writeln!(xml, "      <ScheduleByWeek>").unwrap();
            writeln!(xml, "        <WeeksInterval>1</WeeksInterval>").unwrap();
            writeln!(xml, "        <DaysOfWeek>").unwrap();
            let mut sorted_days = days.clone();
            sorted_days.sort();
            for day in sorted_days {
                writeln!(xml, "          <{} />", day.windows_tag()).unwrap();
            }
            writeln!(xml, "        </DaysOfWeek>").unwrap();
            writeln!(xml, "      </ScheduleByWeek>").unwrap();
            writeln!(xml, "    </CalendarTrigger>").unwrap();
        }
        JobTrigger::Monthly { day, hour, minute } => {
            writeln!(xml, "    <CalendarTrigger>").unwrap();
            writeln!(xml, "      <Enabled>true</Enabled>").unwrap();
            writeln!(
                xml,
                "      <StartBoundary>2024-01-01T{:02}:{:02}:00</StartBoundary>",
                hour, minute
            )
            .unwrap();
            writeln!(xml, "      <ScheduleByMonth>").unwrap();
            writeln!(xml, "        <DaysOfMonth>").unwrap();
            writeln!(xml, "          <Day>{day}</Day>").unwrap();
            writeln!(xml, "        </DaysOfMonth>").unwrap();
            writeln!(xml, "        <Months>").unwrap();
            for month in [
                "January",
                "February",
                "March",
                "April",
                "May",
                "June",
                "July",
                "August",
                "September",
                "October",
                "November",
                "December",
            ] {
                writeln!(xml, "          <{month} />").unwrap();
            }
            writeln!(xml, "        </Months>").unwrap();
            writeln!(xml, "      </ScheduleByMonth>").unwrap();
            writeln!(xml, "    </CalendarTrigger>").unwrap();
        }
        JobTrigger::AtLogin => {
            writeln!(xml, "    <LogonTrigger>").unwrap();
            writeln!(xml, "      <Enabled>true</Enabled>").unwrap();
            writeln!(xml, "    </LogonTrigger>").unwrap();
        }
        JobTrigger::AtBoot => {
            writeln!(xml, "    <BootTrigger>").unwrap();
            writeln!(xml, "      <Enabled>true</Enabled>").unwrap();
            writeln!(xml, "    </BootTrigger>").unwrap();
        }
    }

    Ok(())
}

fn executable_payload(spec: &JobSpec) -> (String, String) {
    let command = spec.action.command_line();
    (
        command.program,
        command
            .args
            .iter()
            .map(|arg| quote_windows_argument(arg))
            .collect::<Vec<_>>()
            .join(" "),
    )
}

fn quote_windows_argument(value: &str) -> String {
    if value.is_empty() {
        return "\"\"".to_string();
    }

    if value
        .chars()
        .all(|ch| !ch.is_whitespace() && ch != '"' && ch != '^' && ch != '&' && ch != '|')
    {
        return value.to_string();
    }

    format!("\"{}\"", value.replace('"', "\\\""))
}

fn duration_xml(seconds: u32) -> String {
    if seconds == 0 {
        "PT0S".to_string()
    } else {
        format!("PT{}S", seconds)
    }
}

fn concurrency_policy_xml(policy: ConcurrencyPolicy) -> &'static str {
    match policy {
        ConcurrencyPolicy::Allow => "Parallel",
        ConcurrencyPolicy::Skip => "IgnoreNew",
        ConcurrencyPolicy::Replace => "StopExisting",
    }
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
    use os_job_core::{ConcurrencyPolicy, JobAction, OutputPolicy, RetryPolicy, Weekday};

    use super::*;

    fn sample_job(trigger: JobTrigger) -> JobSpec {
        JobSpec {
            job_id: "digest-email".to_string(),
            name: "Digest Email".to_string(),
            description: "Send a daily chief-of-staff email digest".to_string(),
            action: JobAction::Command {
                program: r#"C:\ChiefOfStaff\chief-of-staff.exe"#.to_string(),
                args: vec![
                    "digest".to_string(),
                    "--channel".to_string(),
                    "email".to_string(),
                ],
                input: None,
            },
            trigger,
            concurrency_policy: ConcurrencyPolicy::Skip,
            retry_policy: RetryPolicy::default(),
            timeout_seconds: Some(900),
            env: Vec::new(),
            working_directory: Some(r#"C:\ChiefOfStaff"#.to_string()),
            output_policy: OutputPolicy::default(),
            enabled: true,
        }
    }

    #[test]
    fn daily_schedule_renders_calendar_trigger() {
        let plan = WindowsTaskSchedulerXmlBackend
            .install_plan(&sample_job(JobTrigger::Daily {
                hour: 7,
                minute: 30,
            }))
            .expect("daily job should render");

        let xml = &plan.files_to_write[0].contents;
        assert!(xml.contains("<CalendarTrigger>"));
        assert!(xml.contains("<DaysInterval>1</DaysInterval>"));
        assert!(xml.contains("<StartBoundary>2024-01-01T07:30:00</StartBoundary>"));
    }

    #[test]
    fn weekly_schedule_renders_days_of_week() {
        let plan = WindowsTaskSchedulerXmlBackend
            .install_plan(&sample_job(JobTrigger::Weekly {
                days: vec![Weekday::Monday, Weekday::Thursday],
                hour: 9,
                minute: 0,
            }))
            .expect("weekly job should render");

        let xml = &plan.files_to_write[0].contents;
        assert!(xml.contains("<Monday />"));
        assert!(xml.contains("<Thursday />"));
    }

    #[test]
    fn environment_variables_are_rejected() {
        let mut job = sample_job(JobTrigger::AtLogin);
        job.env = vec![os_job_core::EnvironmentEntry {
            key: "COS_PROFILE".to_string(),
            value: "prod".to_string(),
        }];

        let error = WindowsTaskSchedulerXmlBackend
            .install_plan(&job)
            .expect_err("env-backed job should be rejected until a shell-safe launcher exists");

        assert!(matches!(error, JobError::UnsupportedFeature { .. }));
    }

    #[test]
    fn short_interval_is_rejected() {
        let error = WindowsTaskSchedulerXmlBackend
            .install_plan(&sample_job(JobTrigger::Interval {
                every_seconds: 30,
                anchor: None,
            }))
            .expect_err("sub-minute intervals should be rejected");

        assert!(matches!(error, JobError::UnsupportedTrigger { .. }));
    }
}
