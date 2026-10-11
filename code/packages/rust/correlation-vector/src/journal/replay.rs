//! Rebuild nesting and schedule progress from the retained stream. References
//! must name the active TOP scope, not merely some existing or earlier scope.
//! A failed child pipeline does not infer failure of a caller that caught it.
use super::*;
use crate::checked::string_bytes;

enum Frame<'a> {
    Pipeline {
        begin: JournalSequence,
        schedule: Option<&'a [JournalPass]>,
        progress: Option<Progress>,
    },
    Pass {
        begin: JournalSequence,
        policy: JournalPolicy,
        name: &'a str,
    },
}
impl Frame<'_> {
    fn begin(&self) -> JournalSequence {
        match self {
            Self::Pipeline { begin, .. } | Self::Pass { begin, .. } => *begin,
        }
    }
}
#[derive(Default)]
pub(super) struct Scopes<'a> {
    stack: Vec<Frame<'a>>,
}
impl<'a> Scopes<'a> {
    pub(super) fn record(
        &mut self,
        record: &'a JournalRecord,
        usage: &mut Usage,
        limits: &GraphLimits,
        work: &mut Work,
    ) -> Result<(), String> {
        if record.context != self.stack.last().map(Frame::begin) {
            return Err("CV journal references a non-top or closed context".into());
        }
        match &record.event {
            JournalEvent::ContextBegin {
                scope: JournalScope::Pipeline,
            } => {
                if self
                    .stack
                    .last()
                    .is_some_and(|parent| !matches!(parent, Frame::Pass { .. }))
                {
                    return Err("CV pipeline nesting is inconsistent".into());
                }
                self.stack.push(Frame::Pipeline {
                    begin: record.sequence,
                    schedule: None,
                    progress: None,
                });
            }
            JournalEvent::ContextBegin {
                scope: JournalScope::Pass { sweep, slot },
            } => {
                let Some(Frame::Pipeline {
                    schedule: Some(schedule),
                    progress: Some(progress),
                    ..
                }) = self.stack.last()
                else {
                    return Err("CV pass has no enclosing accepted schedule".into());
                };
                let index = progress.pass(sweep.0, slot.0)?;
                self.stack.push(Frame::Pass {
                    begin: record.sequence,
                    policy: schedule[index].policy,
                    name: &schedule[index].name,
                });
            }
            JournalEvent::Schedule { passes, sweep_cap } => {
                let Some(Frame::Pipeline {
                    schedule, progress, ..
                }) = self.stack.last_mut()
                else {
                    return Err("CV schedule is outside its pipeline".into());
                };
                if schedule.is_some() {
                    return Err("duplicate CV pipeline schedule".into());
                }
                add_bounded(&mut usage.events, passes.len(), limits.max_events, "events")?;
                let mut names = HashSet::new();
                for pass in passes {
                    work.take(1)?;
                    if pass.name.len() > limits.max_metadata_bytes {
                        return Err("CV metadata bytes limit exceeded".into());
                    }
                    add_bounded(
                        &mut usage.bytes,
                        string_bytes(&pass.name)?,
                        limits.max_metadata_bytes,
                        "metadata bytes",
                    )?;
                    if !names.insert(pass.name.as_str()) {
                        return Err("duplicate CV scheduled pass name".into());
                    }
                }
                *progress = Some(Progress::new(passes.len(), sweep_cap.0)?);
                *schedule = Some(passes.as_slice());
            }
            JournalEvent::ContextEnd { begin, outcome } => {
                let frame = self.stack.pop().ok_or("CV end has no active context")?;
                if frame.begin() != *begin {
                    return Err("CV end references the wrong begin".into());
                }
                match frame {
                    Frame::Pipeline {
                        progress: Some(progress),
                        ..
                    } => progress.end(*outcome)?,
                    Frame::Pipeline { progress: None, .. }
                        if matches!(
                            outcome,
                            JournalOutcome::SchedulingFailure | JournalOutcome::RecordingFailure
                        ) => {}
                    Frame::Pipeline { .. } => {
                        return Err("CV pipeline ended without an accepted schedule".into())
                    }
                    Frame::Pass { policy, .. } => {
                        let Some(Frame::Pipeline {
                            progress: Some(progress),
                            ..
                        }) = self.stack.last_mut()
                        else {
                            return Err("CV pass lost its parent pipeline".into());
                        };
                        progress.complete(policy, *outcome)?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
    pub(super) fn finish(self) -> Result<(), String> {
        if self.stack.is_empty() {
            Ok(())
        } else {
            Err("CV journal has unclosed contexts".into())
        }
    }
    pub(super) fn source(&self) -> Option<&str> {
        match self.stack.last() {
            Some(Frame::Pass { name, .. }) => Some(*name),
            _ => None,
        }
    }
}
