//! Canonical output borrows evidence and writes through a byte-limited sink.
//!
//! Sorting a map changes only its wire representation. Arrays keep their
//! original order, and serde serializes each number without a float conversion.
//! The validation and encoding phases share one structural work allowance.
use super::checked::Work;
use super::{CVEntry, CVLog, Contribution, DeletionRecord, Origin};
use serde::ser::{Error, SerializeMap, SerializeSeq};
use serde::{Serialize, Serializer};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{self, Write};

struct BoundedWriter {
    bytes: Vec<u8>,
    cap: usize,
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

struct Context(RefCell<Work>);
impl Context {
    fn take<E: Error>(&self, amount: usize) -> Result<(), E> {
        self.0.borrow_mut().take(amount).map_err(E::custom)
    }
}
struct Canonical<'a, T: ?Sized>(&'a T, &'a Context);

impl Serialize for Canonical<'_, CVLog> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.1.take::<S::Error>(1)?;
        let log = self.0;
        let fields = 3
            + usize::from(log.compact_sequence.is_some())
            + usize::from(log.allocator_only_import);
        let mut map = s.serialize_map(Some(fields))?;
        map.serialize_entry("enabled", &log.enabled)?;
        map.serialize_entry("entries", &Canonical(&log.entries, self.1))?;
        if let Some(last) = log.compact_sequence {
            map.serialize_entry("identity", &Identity(last, self.1))?;
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
struct Identity<'a>(u64, &'a Context);
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
