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
}
impl JournalEvent {
    pub(super) fn entity(&self) -> JournalEntity {
        match *self {
            Self::Create { entity }
            | Self::Derive { entity }
            | Self::Merge { entity }
            | Self::Contribution { entity, .. }
            | Self::Deletion { entity } => entity,
        }
    }
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
            work.take(1)?;
            add_bounded(&mut usage.events, 1, limits.max_events, "events")?;
            journal
                .last_sequence
                .checked_add(1)
                .ok_or("CV journal sequence exhausted")?;
            journal
                .events
                .try_reserve(1)
                .map_err(|_| "CV journal record allocation failed")?;
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
                context: None,
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
        for record in &journal.events {
            work.take(1)?;
            expected = expected
                .checked_add(1)
                .ok_or("CV journal sequence overflow")?;
            if record.sequence.0 != expected || record.context.is_some() {
                return Err("CV journal sequence or active context is inconsistent".into());
            }
            let entity = record.event.entity().0;
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
                    if sources.insert(source) {
                        source_order.push(source);
                    }
                }
            }
        }
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
