//! Every open scope owns one future terminal slot. Admission checks retained
//! charges PLUS all open reservations and reserves vector/sequence capacity.
//! An end uses the next actual sequence, never a preselected future number.
use super::*;
use crate::checked::string_bytes;

impl Journal {
    pub(super) fn prepare(
        &mut self,
        usage: &mut Usage,
        limits: &GraphLimits,
        work: &mut Work,
        records: usize,
        extra_reservations: usize,
    ) -> Result<(), String> {
        if self.poisoned {
            return Err("CV journal has an abandoned context".into());
        }
        work.take(
            records
                .checked_add(extra_reservations)
                .ok_or("CV terminal work reservation overflow")?,
        )?;
        add_bounded(&mut usage.events, records, limits.max_events, "events")?;
        let reservations = self
            .active
            .len()
            .checked_add(extra_reservations)
            .ok_or("CV terminal reservation overflow")?;
        let mut effective = usage.events;
        add_bounded(&mut effective, reservations, limits.max_events, "events")?;
        let capacity = records
            .checked_add(reservations)
            .ok_or("CV journal capacity overflow")?;
        let sequence_capacity =
            u64::try_from(capacity).map_err(|_| "CV journal sequence capacity overflow")?;
        self.last_sequence
            .checked_add(sequence_capacity)
            .ok_or("CV journal sequence exhausted")?;
        self.events
            .try_reserve(capacity)
            .map_err(|_| "CV journal record allocation failed")?;
        Ok(())
    }
}

impl CVLog {
    pub(crate) fn validate_journal_source(&self, source: &str) -> Result<(), String> {
        let Some(journal) = &self.journal else {
            return Ok(());
        };
        let Some(active) = journal.active.last() else {
            return Ok(());
        };
        let JournalScope::Pass { slot, .. } = active.scope else {
            return Ok(());
        };
        let parent = &journal.active[journal.active.len() - 2];
        let JournalEvent::Schedule { passes, .. } =
            &journal.events[parent.schedule.expect("active pass schedule")].event
        else {
            unreachable!("schedule index")
        };
        let slot = usize::try_from(slot.0).map_err(|_| "CV pass slot exceeds platform width")?;
        if source != passes[slot].name {
            return Err("CV contribution source disagrees with active pass".into());
        }
        Ok(())
    }
    fn begin_scope(&mut self, scope: JournalScope) -> Result<Option<JournalSequence>, String> {
        let Some(journal) = &self.journal else {
            return Ok(None);
        };
        let policy = match scope {
            JournalScope::Pipeline => {
                if journal
                    .active
                    .last()
                    .is_some_and(|parent| !matches!(parent.scope, JournalScope::Pass { .. }))
                {
                    return Err("CV nested pipeline must be a direct child of a pass".into());
                }
                None
            }
            JournalScope::Pass { sweep, slot } => {
                let parent = journal
                    .active
                    .last()
                    .ok_or("CV pass requires an active pipeline")?;
                if !matches!(parent.scope, JournalScope::Pipeline) {
                    return Err("CV pass parent is not a pipeline".into());
                }
                let index = parent
                    .progress
                    .as_ref()
                    .ok_or("CV pass requires an accepted schedule")?
                    .pass(sweep.0, slot.0)?;
                let JournalEvent::Schedule { passes, .. } =
                    &journal.events[parent.schedule.expect("accepted schedule")].event
                else {
                    unreachable!("schedule index")
                };
                Some(passes[index].policy)
            }
        };
        let state = self.checked.as_ref().expect("chronology is checked");
        let mut usage = state.usage;
        let mut work = Work::new(state.limits.max_work);
        let journal = self.journal.as_mut().expect("chronology present");
        journal.prepare(&mut usage, &state.limits, &mut work, 1, 1)?;
        journal
            .active
            .try_reserve(1)
            .map_err(|_| "CV scope allocation failed")?;
        let context = journal.active.last().map(|parent| parent.begin);
        journal.last_sequence += 1;
        let begin = JournalSequence(journal.last_sequence);
        journal.events.push(JournalRecord {
            sequence: begin,
            context,
            event: JournalEvent::ContextBegin { scope },
        });
        journal.active.push(Active {
            begin,
            scope,
            schedule: None,
            progress: None,
            policy,
        });
        self.commit_usage(Some(usage));
        Ok(Some(begin))
    }

    fn end_scope(
        &mut self,
        begin: Option<JournalSequence>,
        outcome: JournalOutcome,
        success: bool,
    ) {
        let Some(begin) = begin else {
            return;
        };
        let journal = self.journal.as_mut().expect("scoped chronology");
        if journal.active.last().map(|scope| scope.begin) != Some(begin) {
            // Catching a panic from an unfinished inner callback cannot recover
            // a complete journal. Preserve the returned owned value and leave
            // explicit unusable state; do not fabricate the missing boundary.
            journal.poisoned = true;
            return;
        }
        let active = journal.active.pop().expect("active scope");
        let outcome_success = matches!(
            outcome,
            JournalOutcome::Accepted { .. } | JournalOutcome::Converged | JournalOutcome::Cap
        );
        journal.poisoned |= success != outcome_success;
        let valid = match active.scope {
            JournalScope::Pipeline => match active.progress {
                Some(progress) => progress.end(outcome),
                None if matches!(
                    outcome,
                    JournalOutcome::SchedulingFailure | JournalOutcome::RecordingFailure
                ) =>
                {
                    Ok(())
                }
                None => Err("pipeline ended without a schedule".to_owned()),
            },
            JournalScope::Pass { .. } => journal
                .active
                .last_mut()
                .and_then(|parent| parent.progress.as_mut())
                .ok_or_else(|| "pass lost its enclosing schedule".to_owned())
                .and_then(|progress| {
                    progress.complete(active.policy.expect("pass policy"), outcome)
                }),
        };
        journal.poisoned |= valid.is_err();
        // All arithmetic, retained charges and vector capacity were purchased
        // at begin and protected by intervening mutations. No fallible action
        // here can recursively drop an arbitrary deep callback return value.
        journal.last_sequence += 1;
        journal.events.push(JournalRecord {
            sequence: JournalSequence(journal.last_sequence),
            context: Some(begin),
            event: JournalEvent::ContextEnd { begin, outcome },
        });
        self.checked
            .as_mut()
            .expect("checked chronology")
            .usage
            .events += 1;
    }

    /// Record one actual pipeline invocation, restoring its caller on every
    /// ordinary result. Begin failure occurs before the callback is entered.
    pub fn with_pipeline<T, E>(
        &mut self,
        callback: impl FnOnce(&mut CVLog) -> Result<(T, PipelineOutcome), (E, PipelineOutcome)>,
    ) -> Result<T, ScopeError<E>> {
        let begin = self
            .begin_scope(JournalScope::Pipeline)
            .map_err(ScopeError::Recording)?;
        match callback(self) {
            Ok((value, outcome)) => {
                self.end_scope(begin, outcome.into(), true);
                Ok(value)
            }
            Err((error, outcome)) => {
                self.end_scope(begin, outcome.into(), false);
                Err(ScopeError::Callback(error))
            }
        }
    }
    /// Scope callback and candidate acceptance together at the real scheduled
    /// position. Pass name/policy resolve from the accepted parent schedule.
    pub fn with_pass<T, E>(
        &mut self,
        sweep: u64,
        slot: u64,
        callback: impl FnOnce(&mut CVLog) -> Result<(T, PassOutcome), (E, PassOutcome)>,
    ) -> Result<T, ScopeError<E>> {
        let begin = self
            .begin_scope(JournalScope::Pass {
                sweep: JournalSequence(sweep),
                slot: JournalSequence(slot),
            })
            .map_err(ScopeError::Recording)?;
        match callback(self) {
            Ok((value, outcome)) => {
                self.end_scope(begin, outcome.into(), true);
                Ok(value)
            }
            Err((error, outcome)) => {
                self.end_scope(begin, outcome.into(), false);
                Err(ScopeError::Callback(error))
            }
        }
    }
    /// Retain the actual accepted ordered schedule once per pipeline. Charges
    /// buy each descriptor and its encoded name before any ownership copy.
    pub fn record_schedule(
        &mut self,
        passes: &[(&str, JournalPolicy)],
        sweep_cap: u64,
    ) -> Result<(), String> {
        let Some(journal) = &self.journal else {
            return Ok(());
        };
        let active = journal
            .active
            .last()
            .ok_or("CV schedule requires an active pipeline")?;
        if !matches!(active.scope, JournalScope::Pipeline) || active.schedule.is_some() {
            return Err("CV schedule has the wrong scope or is already recorded".into());
        }
        let progress = Progress::new(passes.len(), sweep_cap)?;
        let state = self.checked.as_ref().expect("checked chronology");
        let mut usage = state.usage;
        add_bounded(
            &mut usage.events,
            passes.len(),
            state.limits.max_events,
            "events",
        )?;
        let mut work = Work::new(state.limits.max_work);
        for (name, _) in passes {
            work.take(1)?;
            if name.len() > state.limits.max_metadata_bytes {
                return Err("CV metadata bytes limit exceeded".into());
            }
            add_bounded(
                &mut usage.bytes,
                string_bytes(name)?,
                state.limits.max_metadata_bytes,
                "metadata bytes",
            )?;
        }
        // Descriptor counts and name bytes are bounded before allocating this
        // borrowed duplicate-name index. Same names in other runs remain valid.
        let mut names = HashSet::new();
        for (name, _) in passes {
            work.take(1)?;
            if !names.insert(*name) {
                return Err("duplicate CV scheduled pass name".into());
            }
        }
        let journal = self.journal.as_mut().expect("chronology present");
        journal.prepare(&mut usage, &state.limits, &mut work, 1, 0)?;
        let mut descriptors = Vec::new();
        descriptors
            .try_reserve(passes.len())
            .map_err(|_| "CV schedule allocation failed")?;
        for (name, policy) in passes {
            descriptors.push(JournalPass {
                name: (*name).to_owned(),
                policy: *policy,
            });
        }
        let index = journal.events.len();
        journal.last_sequence += 1;
        let active = journal.active.last_mut().expect("active pipeline");
        journal.events.push(JournalRecord {
            sequence: JournalSequence(journal.last_sequence),
            context: Some(active.begin),
            event: JournalEvent::Schedule {
                passes: descriptors,
                sweep_cap: JournalSequence(sweep_cap),
            },
        });
        active.schedule = Some(index);
        active.progress = Some(progress);
        self.commit_usage(Some(usage));
        Ok(())
    }
}
