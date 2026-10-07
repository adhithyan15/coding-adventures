//! Canonical output borrows evidence and writes through a byte-limited sink.
//!
//! Sorting a map changes only its wire representation. Arrays keep their
//! original order, and serde serializes each number without a float conversion.
//! The validation and encoding phases share one structural work allowance.
use super::checked::Work;
use super::{CVEntry, CVLog, Contribution, DeletionRecord, Origin};
use super::{Journal, JournalEntity, JournalEvent, JournalRecord, JournalSequence};
use super::{JournalOutcome, JournalPass, JournalScope};
use serde::ser::{Error, SerializeMap, SerializeSeq};
use serde::{Serialize, Serializer};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{self, Write};

pub(super) struct BoundedWriter {
    pub(super) bytes: Vec<u8>,
    pub(super) cap: usize,
}
impl Write for BoundedWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let next = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or_else(|| io::Error::other("CV output bytes accounting overflow"))?;
        if next > self.cap {
            return Err(io::Error::other("CV output bytes limit exceeded"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(super) struct Context(pub(super) RefCell<Work>);
impl Context {
    pub(super) fn take<E: Error>(&self, amount: usize) -> Result<(), E> {
        self.0.borrow_mut().take(amount).map_err(E::custom)
    }
}
pub(super) struct Canonical<'a, T: ?Sized>(pub(super) &'a T, pub(super) &'a Context);

impl Serialize for Canonical<'_, CVLog> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.1.take::<S::Error>(1)?;
        let log = self.0;
        let fields = 3
            + usize::from(log.compact_sequence.is_some())
            + usize::from(log.journal.is_some())
            + usize::from(log.allocator_only_import);
        let mut map = s.serialize_map(Some(fields))?;
        map.serialize_entry("enabled", &log.enabled)?;
        map.serialize_entry("entries", &Canonical(&log.entries, self.1))?;
        if let Some(last) = log.compact_sequence {
            map.serialize_entry("identity", &Identity(last, self.1))?;
        }
        if let Some(journal) = &log.journal {
            map.serialize_entry("journal", &Canonical(journal, self.1))?;
        }
        self.1.take::<S::Error>(log.pass_order.len())?;
        map.serialize_entry("pass_order", &log.pass_order)?;
        if log.allocator_only_import {
            self.1.take::<S::Error>(1)?;
            map.serialize_entry("unchecked_import", &true)?;
        }
        map.end()
    }
}
pub(super) struct Identity<'a>(pub(super) u64, pub(super) &'a Context);

impl Serialize for JournalSequence {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format!("{:016x}", self.0))
    }
}
impl Serialize for JournalEntity {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format!("cv1.{:016x}", self.0))
    }
}
impl Serialize for Canonical<'_, Journal> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.1.take::<S::Error>(1)?;
        let mut map = s.serialize_map(Some(4))?;
        map.serialize_entry("coverage", "full")?;
        map.serialize_entry("events", &Canonical(self.0.events.as_slice(), self.1))?;
        map.serialize_entry("last_sequence", &JournalSequence(self.0.last_sequence))?;
        map.serialize_entry("version", "chronology-v1")?;
        map.end()
    }
}
impl Serialize for Canonical<'_, [JournalRecord]> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.1.take::<S::Error>(self.0.len())?;
        let mut seq = s.serialize_seq(Some(self.0.len()))?;
        for record in self.0 {
            seq.serialize_element(&Canonical(record, self.1))?;
        }
        seq.end()
    }
}
impl Serialize for Canonical<'_, JournalRecord> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.1.take::<S::Error>(1)?;
        let mut map = s.serialize_map(Some(3))?;
        map.serialize_entry("context", &self.0.context)?;
        map.serialize_entry("event", &Canonical(&self.0.event, self.1))?;
        map.serialize_entry("sequence", &self.0.sequence)?;
        map.end()
    }
}
impl Serialize for Canonical<'_, JournalEvent> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.1.take::<S::Error>(1)?;
        match self.0 {
            JournalEvent::ContextBegin { scope } => {
                let mut map = s.serialize_map(Some(2))?;
                map.serialize_entry("kind", "context_begin")?;
                map.serialize_entry("scope", &Canonical(scope, self.1))?;
                return map.end();
            }
            JournalEvent::Schedule { passes, sweep_cap } => {
                let mut map = s.serialize_map(Some(3))?;
                map.serialize_entry("kind", "schedule")?;
                map.serialize_entry("passes", &Canonical(passes.as_slice(), self.1))?;
                map.serialize_entry("sweep_cap", sweep_cap)?;
                return map.end();
            }
            JournalEvent::ContextEnd { begin, outcome } => {
                let mut map = s.serialize_map(Some(3))?;
                map.serialize_entry("begin", begin)?;
                map.serialize_entry("kind", "context_end")?;
                map.serialize_entry("outcome", &Canonical(outcome, self.1))?;
                return map.end();
            }
            _ => {}
        }
        let (kind, entity, index) = match self.0 {
            JournalEvent::Create { entity } => ("create", entity, None),
            JournalEvent::Derive { entity } => ("derive", entity, None),
            JournalEvent::Merge { entity } => ("merge", entity, None),
            JournalEvent::Contribution { entity, index } => ("contribution", entity, Some(index)),
            JournalEvent::Deletion { entity } => ("deletion", entity, None),
            _ => unreachable!("scope events serialized above"),
        };
        let mut map = s.serialize_map(Some(2 + usize::from(index.is_some())))?;
        map.serialize_entry("entity", entity)?;
        if let Some(index) = index {
            map.serialize_entry("index", index)?;
        }
        map.serialize_entry("kind", kind)?;
        map.end()
    }
}
impl Serialize for Canonical<'_, JournalScope> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.1.take::<S::Error>(1)?;
        let pass = matches!(self.0, JournalScope::Pass { .. });
        let mut map = s.serialize_map(Some(if pass { 3 } else { 1 }))?;
        map.serialize_entry("kind", if pass { "pass" } else { "pipeline" })?;
        if let JournalScope::Pass { slot, sweep } = self.0 {
            map.serialize_entry("slot", slot)?;
            map.serialize_entry("sweep", sweep)?;
        }
        map.end()
    }
}
impl Serialize for Canonical<'_, JournalOutcome> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.1.take::<S::Error>(1)?;
        let kind = match self.0 {
            JournalOutcome::Accepted { .. } => "accepted",
            JournalOutcome::Converged => "converged",
            JournalOutcome::Cap => "cap",
            JournalOutcome::SchedulingFailure => "scheduling_failure",
            JournalOutcome::CallbackFailure => "callback_failure",
            JournalOutcome::AcceptanceFailure => "acceptance_failure",
            JournalOutcome::RecordingFailure => "recording_failure",
        };
        let mut map =
            s.serialize_map(Some(if matches!(self.0, JournalOutcome::Accepted { .. }) {
                2
            } else {
                1
            }))?;
        if let JournalOutcome::Accepted { changed } = self.0 {
            map.serialize_entry("changed", changed)?;
        }
        map.serialize_entry("kind", kind)?;
        map.end()
    }
}
impl Serialize for Canonical<'_, [JournalPass]> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.1.take::<S::Error>(self.0.len())?;
        let mut seq = s.serialize_seq(Some(self.0.len()))?;
        for pass in self.0 {
            seq.serialize_element(&Canonical(pass, self.1))?;
        }
        seq.end()
    }
}
impl Serialize for Canonical<'_, JournalPass> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.1.take::<S::Error>(1)?;
        let mut map = s.serialize_map(Some(2))?;
        map.serialize_entry("name", &self.0.name)?;
        map.serialize_entry("policy", &self.0.policy)?;
        map.end()
    }
}
impl Serialize for Identity<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.1.take::<S::Error>(1)?;
        let mut map = s.serialize_map(Some(2))?;
        map.serialize_entry("last_sequence", &format!("{:016x}", self.0))?;
        map.serialize_entry("scheme", "compact-v1")?;
        map.end()
    }
}

impl Serialize for Canonical<'_, HashMap<String, CVEntry>> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.1.take::<S::Error>(self.0.len())?;
        let mut entries: Vec<_> = self.0.iter().collect();
        entries.sort_unstable_by(|a, b| a.0.cmp(b.0));
        let mut map = s.serialize_map(Some(entries.len()))?;
        for (key, value) in entries {
            map.serialize_entry(key, &Canonical(value, self.1))?;
        }
        map.end()
    }
}
impl Serialize for Canonical<'_, CVEntry> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let entry = self.0;
        self.1.take::<S::Error>(1)?;
        let mut map = s.serialize_map(Some(5))?;
        map.serialize_entry(
            "contributions",
            &Canonical(entry.contributions.as_slice(), self.1),
        )?;
        map.serialize_entry(
            "deleted",
            &entry.deleted.as_ref().map(|v| Canonical(v, self.1)),
        )?;
        map.serialize_entry("id", &entry.id)?;
        map.serialize_entry(
            "origin",
            &entry.origin.as_ref().map(|v| Canonical(v, self.1)),
        )?;
        self.1.take::<S::Error>(entry.parent_ids.len())?;
        map.serialize_entry("parent_ids", &entry.parent_ids)?;
        map.end()
    }
}
impl Serialize for Canonical<'_, [Contribution]> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.1.take::<S::Error>(self.0.len())?;
        let mut seq = s.serialize_seq(Some(self.0.len()))?;
        for event in self.0 {
            seq.serialize_element(&Canonical(event, self.1))?;
        }
        seq.end()
    }
}
impl Serialize for Canonical<'_, Origin> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.1.take::<S::Error>(1)?;
        let mut map = s.serialize_map(Some(4))?;
        map.serialize_entry("location", &self.0.location)?;
        map.serialize_entry("meta", &Canonical(&self.0.meta, self.1))?;
        map.serialize_entry("source", &self.0.source)?;
        map.serialize_entry("timestamp", &self.0.timestamp)?;
        map.end()
    }
}
impl Serialize for Canonical<'_, Contribution> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.1.take::<S::Error>(1)?;
        let mut map = s.serialize_map(Some(3))?;
        map.serialize_entry("meta", &Canonical(&self.0.meta, self.1))?;
        map.serialize_entry("source", &self.0.source)?;
        map.serialize_entry("tag", &self.0.tag)?;
        map.end()
    }
}
impl Serialize for Canonical<'_, DeletionRecord> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.1.take::<S::Error>(1)?;
        let mut map = s.serialize_map(Some(3))?;
        map.serialize_entry("meta", &Canonical(&self.0.meta, self.1))?;
        map.serialize_entry("reason", &self.0.reason)?;
        map.serialize_entry("source", &self.0.source)?;
        map.end()
    }
}
impl Serialize for Canonical<'_, HashMap<String, Value>> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.1.take::<S::Error>(1)?;
        self.1.take::<S::Error>(self.0.len())?;
        let mut fields: Vec<_> = self.0.iter().collect();
        fields.sort_unstable_by(|a, b| a.0.cmp(b.0));
        let mut map = s.serialize_map(Some(fields.len()))?;
        for (key, value) in fields {
            map.serialize_entry(key, &Canonical(value, self.1))?;
        }
        map.end()
    }
}
impl Serialize for Canonical<'_, Value> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.1.take::<S::Error>(1)?;
        match self.0 {
            Value::Array(items) => {
                self.1.take::<S::Error>(items.len())?;
                let mut seq = s.serialize_seq(Some(items.len()))?;
                for item in items {
                    seq.serialize_element(&Canonical(item, self.1))?;
                }
                seq.end()
            }
            Value::Object(fields) => {
                self.1.take::<S::Error>(fields.len())?;
                let mut fields: Vec<_> = fields.iter().collect();
                fields.sort_unstable_by(|a, b| a.0.cmp(b.0));
                let mut map = s.serialize_map(Some(fields.len()))?;
                for (key, value) in fields {
                    map.serialize_entry(key, &Canonical(value, self.1))?;
                }
                map.end()
            }
            scalar => scalar.serialize(s),
        }
    }
}

pub(super) fn log(log: &CVLog) -> Result<String, String> {
    let (limits, work) = log.export_work()?;
    let context = Context(RefCell::new(work));
    let mut writer = BoundedWriter {
        bytes: Vec::new(),
        cap: limits.max_output_bytes,
    };
    serde_json::to_writer(&mut writer, &Canonical(log, &context))
        .map_err(|e| format!("CV serialization error: {e}"))?;
    String::from_utf8(writer.bytes).map_err(|e| format!("CV serialization encoding: {e}"))
}
