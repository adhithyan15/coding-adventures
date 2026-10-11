//! An operation clock is different from the identity allocator. Two entities
//! can be created first, then receive contributions in either order. Retaining
//! fixed-size references lets us record that order without copying their owned
//! metadata. Admission purchases a record before the graph changes; replay
//! checks the resulting evidence independently at every full trust boundary.
use super::checked::{add_bounded, Usage, Work};
use super::{compact_id_sequence, CVLog, GraphLimits};
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer};
use std::collections::{HashMap, HashSet};
use std::fmt;
mod replay;
mod scopes;
#[cfg(test)]
mod tests;

/// Precision-safe sequence scalar; its wire form is always sixteen hex digits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JournalSequence(pub(super) u64);
impl JournalSequence {
    pub fn value(self) -> u64 {
        self.0
    }
}
impl<'de> Deserialize<'de> for JournalSequence {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Hex;
        impl Visitor<'_> for Hex {
            type Value = JournalSequence;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("sixteen lowercase hexadecimal digits")
            }
            fn visit_str<E: Error>(self, text: &str) -> Result<Self::Value, E> {
                super::fixed_hex_sequence(text)
                    .map(JournalSequence)
                    .map_err(E::custom)
            }
        }
        d.deserialize_str(Hex)
    }
}

/// Fixed-size reference to a compact graph identity, rather than another copy
/// of its string or payload. Encoding retains the original compact ID spelling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JournalEntity(pub(super) u64);
impl JournalEntity {
    pub fn sequence(self) -> u64 {
        self.0
    }
}
impl<'de> Deserialize<'de> for JournalEntity {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Entity;
        impl Visitor<'_> for Entity {
            type Value = JournalEntity;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a compact CV identity")
            }
            fn visit_str<E: Error>(self, text: &str) -> Result<Self::Value, E> {
                compact_id_sequence(text)
                    .map(JournalEntity)
                    .map_err(E::custom)
            }
        }
        d.deserialize_str(Entity)
    }
}

/// Closed operation tags reference graph facts without duplicating metadata.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum JournalEvent {
    Create {
        entity: JournalEntity,
    },
    Derive {
        entity: JournalEntity,
    },
    Merge {
        entity: JournalEntity,
    },
    Contribution {
        entity: JournalEntity,
        index: JournalSequence,
    },
    Deletion {
        entity: JournalEntity,
    },
    ContextBegin {
        scope: JournalScope,
    },
    Schedule {
        passes: Vec<JournalPass>,
        sweep_cap: JournalSequence,
    },
    ContextEnd {
        begin: JournalSequence,
        outcome: JournalOutcome,
    },
}
impl JournalEvent {
    pub(super) fn entity(&self) -> Option<JournalEntity> {
        match *self {
            Self::Create { entity }
            | Self::Derive { entity }
            | Self::Merge { entity }
            | Self::Contribution { entity, .. }
            | Self::Deletion { entity } => Some(entity),
            Self::ContextBegin { .. } | Self::Schedule { .. } | Self::ContextEnd { .. } => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize, PartialEq, Eq)]
pub enum JournalPolicy {
    #[serde(rename = "one-shot")]
    OneShot,
    #[serde(rename = "fixed-point")]
    FixedPoint,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JournalPass {
    pub(super) name: String,
    pub(super) policy: JournalPolicy,
}
impl JournalPass {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn policy(&self) -> JournalPolicy {
        self.policy
    }
}
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum JournalScope {
    Pipeline,
    Pass {
        sweep: JournalSequence,
        slot: JournalSequence,
    },
}
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum JournalOutcome {
    Accepted { changed: bool },
    Converged,
    Cap,
    SchedulingFailure,
    CallbackFailure,
    AcceptanceFailure,
    RecordingFailure,
}
#[derive(Clone, Copy, Debug)]
pub enum PipelineOutcome {
    Converged,
    Cap,
    SchedulingFailure,
    CallbackFailure,
    AcceptanceFailure,
    RecordingFailure,
}
impl From<PipelineOutcome> for JournalOutcome {
    fn from(outcome: PipelineOutcome) -> Self {
        match outcome {
            PipelineOutcome::Converged => Self::Converged,
            PipelineOutcome::Cap => Self::Cap,
            PipelineOutcome::SchedulingFailure => Self::SchedulingFailure,
            PipelineOutcome::CallbackFailure => Self::CallbackFailure,
            PipelineOutcome::AcceptanceFailure => Self::AcceptanceFailure,
            PipelineOutcome::RecordingFailure => Self::RecordingFailure,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub enum PassOutcome {
    Accepted { changed: bool },
    CallbackFailure,
    AcceptanceFailure,
}
impl From<PassOutcome> for JournalOutcome {
    fn from(outcome: PassOutcome) -> Self {
        match outcome {
            PassOutcome::Accepted { changed } => Self::Accepted { changed },
            PassOutcome::CallbackFailure => Self::CallbackFailure,
            PassOutcome::AcceptanceFailure => Self::AcceptanceFailure,
        }
    }
}
#[derive(Debug, PartialEq, Eq)]
pub enum ScopeError<E> {
    Recording(String),
    Callback(E),
}

/// Fixed-size running schedule state. Replay builds this state afresh from
/// records; the stored live cursor is never trusted by import or export.
#[derive(Debug)]
struct Progress {
    pass_count: usize,
    cap: u64,
    sweep: u64,
    slot: usize,
    completed: u64,
    changed: bool,
    last_changed: bool,
    terminal: bool,
    failed: Option<JournalOutcome>,
}
impl Progress {
    fn new(pass_count: usize, cap: u64) -> Result<Self, String> {
        if cap == 0 {
            return Err("CV schedule sweep cap must be positive".into());
        }
        Ok(Self {
            pass_count,
            cap,
            sweep: 0,
            slot: 0,
            completed: u64::from(pass_count == 0),
            changed: false,
            last_changed: false,
            terminal: pass_count == 0,
            failed: None,
        })
    }
    fn pass(&self, sweep: u64, slot: u64) -> Result<usize, String> {
        let slot = usize::try_from(slot).map_err(|_| "CV pass slot exceeds platform width")?;
        if self.terminal
            || self.failed.is_some()
            || sweep != self.sweep
            || slot != self.slot
            || slot >= self.pass_count
        {
            return Err("CV pass disagrees with schedule progress".into());
        }
        Ok(slot)
    }
    fn complete(&mut self, policy: JournalPolicy, outcome: JournalOutcome) -> Result<(), String> {
        match outcome {
            JournalOutcome::Accepted { changed } => {
                self.changed |= changed && policy == JournalPolicy::FixedPoint;
                self.slot = self.slot.checked_add(1).ok_or("CV pass slot overflow")?;
                if self.slot == self.pass_count {
                    self.completed = self.completed.checked_add(1).ok_or("CV sweep overflow")?;
                    self.last_changed = self.changed;
                    self.terminal = !self.changed || self.completed == self.cap;
                    if !self.terminal {
                        self.sweep = self.sweep.checked_add(1).ok_or("CV sweep overflow")?;
                        self.slot = 0;
                        self.changed = false;
                    }
                }
            }
            JournalOutcome::CallbackFailure | JournalOutcome::AcceptanceFailure => {
                self.failed = Some(outcome);
                self.terminal = true;
            }
            _ => return Err("CV pass terminal outcome has the wrong scope kind".into()),
        }
        Ok(())
    }
    fn end(&self, outcome: JournalOutcome) -> Result<(), String> {
        let valid = match outcome {
            JournalOutcome::Converged => {
                self.terminal && self.failed.is_none() && !self.last_changed && self.completed > 0
            }
            JournalOutcome::Cap => {
                self.terminal
                    && self.failed.is_none()
                    && self.last_changed
                    && self.completed == self.cap
            }
            JournalOutcome::CallbackFailure | JournalOutcome::AcceptanceFailure => {
                self.failed == Some(outcome)
            }
            JournalOutcome::RecordingFailure => !self.terminal && self.failed.is_none(),
            _ => false,
        };
        if valid {
            Ok(())
        } else {
            Err("CV pipeline outcome contradicts schedule progress".into())
        }
    }
}
#[derive(Debug)]
struct Active {
    begin: JournalSequence,
    scope: JournalScope,
    schedule: Option<usize>,
    progress: Option<Progress>,
    policy: Option<JournalPolicy>,
}

/// Read-only retained record. A context reference belongs to the actual active
/// scope; graph callers cannot assign it through arbitrary contribution metadata.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JournalRecord {
    pub(super) sequence: JournalSequence,
    pub(super) context: Option<JournalSequence>,
    pub(super) event: JournalEvent,
}
impl JournalRecord {
    pub fn sequence(&self) -> u64 {
        self.sequence.0
    }
    pub fn context(&self) -> Option<u64> {
        self.context.map(|v| v.0)
    }
    pub fn event(&self) -> &JournalEvent {
        &self.event
    }
}

#[derive(Deserialize)]
enum Version {
    #[serde(rename = "chronology-v1")]
    V1,
}
#[derive(Deserialize)]
enum FullCoverage {
    #[serde(rename = "full")]
    Full,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    version: Version,
    coverage: FullCoverage,
    last_sequence: JournalSequence,
    events: Vec<JournalRecord>,
}

/// Accepted global chronology. Absence on CVLog means unavailable, rather than
/// an empty or reconstructed execution history. Only CVLog can extend it.
#[derive(Debug, Default)]
pub struct Journal {
    pub(super) last_sequence: u64,
    pub(super) events: Vec<JournalRecord>,
    active: Vec<Active>,
    poisoned: bool,
}
impl Journal {
    pub fn last_sequence(&self) -> u64 {
        self.last_sequence
    }
    pub fn events(&self) -> &[JournalRecord] {
        &self.events
    }
}
impl<'de> Deserialize<'de> for Journal {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let Wire {
            version: Version::V1,
            coverage: FullCoverage::Full,
            last_sequence,
            events,
        } = Wire::deserialize(d)?;
        Ok(Self {
            last_sequence: last_sequence.0,
            events,
            active: Vec::new(),
            poisoned: false,
        })
    }
}

impl CVLog {
    /// Start complete chronology on a fresh checked compact allocator. Existing
    /// logs cannot retrospectively manufacture the operations they never stored.
    pub fn new_checked_chronology(limits: GraphLimits) -> Result<Self, String> {
        let mut log = Self::new_checked_compact(limits)?;
        log.journal = Some(Journal::default());
        Ok(log)
    }
    pub fn journal(&self) -> Option<&Journal> {
        self.journal.as_ref()
    }

    pub(super) fn prepare_journal_record(
        &mut self,
        mut usage: Usage,
        limits: &GraphLimits,
        work: &mut Work,
    ) -> Result<Usage, String> {
        if let Some(journal) = &mut self.journal {
            journal.prepare(&mut usage, limits, work, 1, 0)?;
        }
        Ok(usage)
    }
    pub(super) fn append_graph_record(&mut self, event: impl FnOnce() -> JournalEvent) {
        if let Some(journal) = &mut self.journal {
            // Admission proved arithmetic and purchased capacity before any
            // graph/allocator mutation. This completion step cannot reject.
            journal.last_sequence += 1;
            journal.events.push(JournalRecord {
                sequence: JournalSequence(journal.last_sequence),
                context: journal.active.last().map(|scope| scope.begin),
                event: event(),
            });
        }
    }

    pub(super) fn validate_journal(
        &self,
        usage: &mut Usage,
        limits: &GraphLimits,
        work: &mut Work,
    ) -> Result<(), String> {
        let Some(journal) = &self.journal else {
            return Ok(());
        };
        if journal.poisoned || !journal.active.is_empty() {
            return Err("CV journal has an active or abandoned context".into());
        }
        if self.compact_sequence.is_none() || self.checked.is_none() {
            return Err("CV journal requires checked compact evidence".into());
        }
        add_bounded(
            &mut usage.events,
            journal.events.len(),
            limits.max_events,
            "events",
        )?;
        if u64::try_from(journal.events.len()).ok() != Some(journal.last_sequence) {
            return Err("CV journal sequence coverage is incomplete".into());
        }
        // Replay state is bounded by already validated nodes and journal records.
        // Each entity must be created exactly once, and each contribution must
        // reference its next entry-local index in actual global call order.
        let mut states: HashMap<u64, (usize, bool)> = HashMap::new();
        let mut sources = HashSet::new();
        let mut source_order = Vec::new();
        let mut expected = 0u64;
        let mut scopes = replay::Scopes::default();
        for record in &journal.events {
            work.take(1)?;
            expected = expected
                .checked_add(1)
                .ok_or("CV journal sequence overflow")?;
            if record.sequence.0 != expected {
                return Err("CV journal sequence or active context is inconsistent".into());
            }
            scopes.record(record, usage, limits, work)?;
            let Some(entity) = record.event.entity().map(|entity| entity.0) else {
                continue;
            };
            let id = format!("cv1.{entity:016x}");
            let entry = self
                .entries
                .get(&id)
                .ok_or("CV journal has an unknown entity")?;
            match record.event {
                JournalEvent::Create { .. }
                | JournalEvent::Derive { .. }
                | JournalEvent::Merge { .. } => {
                    if states.contains_key(&entity) {
                        return Err("duplicate CV journal creation".into());
                    }
                    match record.event {
                        JournalEvent::Create { .. } if !entry.parent_ids.is_empty() => {
                            return Err("CV create journal has parents".into())
                        }
                        JournalEvent::Derive { .. } if entry.parent_ids.len() != 1 => {
                            return Err("CV derive journal requires one parent".into())
                        }
                        _ => {}
                    }
                    for parent in &entry.parent_ids {
                        work.take(1)?;
                        if !states.contains_key(&compact_id_sequence(parent)?) {
                            return Err("CV journal parent was not created earlier".into());
                        }
                    }
                    // Entity allocator order is independently contiguous, even
                    // when other record kinds intervene between node creations.
                    if u64::try_from(states.len())
                        .ok()
                        .and_then(|v| v.checked_add(1))
                        != Some(entity)
                    {
                        return Err("CV journal disagrees with node allocation order".into());
                    }
                    states.insert(entity, (0, false));
                }
                JournalEvent::Contribution { index, .. } => {
                    let state = states
                        .get_mut(&entity)
                        .ok_or("CV contribution precedes creation")?;
                    let index = usize::try_from(index.0)
                        .map_err(|_| "CV contribution index exceeds platform width")?;
                    if state.1 || index != state.0 || entry.contributions.get(index).is_none() {
                        return Err("CV journal contribution order is inconsistent".into());
                    }
                    state.0 = state
                        .0
                        .checked_add(1)
                        .ok_or("CV contribution index overflow")?;
                    work.take(1)?;
                    let source = entry.contributions[index].source.as_str();
                    if scopes.source().is_some_and(|expected| source != expected) {
                        return Err("CV contribution source disagrees with journal pass".into());
                    }
                    if sources.insert(source) {
                        source_order.push(source);
                    }
                }
                JournalEvent::Deletion { .. } => {
                    let state = states
                        .get_mut(&entity)
                        .ok_or("CV deletion precedes creation")?;
                    if state.1 || entry.deleted.is_none() || state.0 != entry.contributions.len() {
                        return Err("CV journal deletion order is inconsistent".into());
                    }
                    state.1 = true;
                    work.take(1)?;
                    let source = entry
                        .deleted
                        .as_ref()
                        .expect("validated deletion")
                        .source
                        .as_str();
                    if scopes.source().is_some_and(|expected| source != expected) {
                        return Err("CV deletion source disagrees with journal pass".into());
                    }
                    if sources.insert(source) {
                        source_order.push(source);
                    }
                }
                JournalEvent::ContextBegin { .. }
                | JournalEvent::Schedule { .. }
                | JournalEvent::ContextEnd { .. } => {
                    unreachable!("non-entity records were replayed above")
                }
            }
        }
        scopes.finish()?;
        if states.len() != self.entries.len() {
            return Err("CV journal omitted graph creations".into());
        }
        if !source_order
            .iter()
            .copied()
            .eq(self.pass_order.iter().map(String::as_str))
        {
            return Err("CV contributor inventory disagrees with journal chronology".into());
        }
        for (id, entry) in &self.entries {
            work.take(1)?;
            let (contributions, deleted) = states[&compact_id_sequence(id)?];
            if contributions != entry.contributions.len() || deleted != entry.deleted.is_some() {
                return Err("CV journal omitted graph facts".into());
            }
        }
        Ok(())
    }
}
