//! Checked presentation views borrow evidence rather than cloning a JSON tree.
//!
//! A filter changes presentation, never the trust boundary: validate the complete
//! log first, then charge the reference index, comparisons, sorting and encoding
//! against that same operation. Every byte goes through one bounded sink.
use super::canonical::{BoundedWriter, Canonical, Context, Identity};
use super::{CVEntry, CVLog, JournalRecord, JournalSequence};
use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Serialize, Serializer};
use std::cell::RefCell;
use std::io::Write;

/// Exact contribution-source selection, optionally including origins or inverted.
/// An empty source list selects the complete log, regardless of the other flags.
#[derive(Clone, Copy, Debug, Default)]
pub struct SourceFilter<'a> {
    pub sources: &'a [String],
    pub include_origin: bool,
    pub invert: bool,
}

/// Snapshot wire representation. All variants use the same canonical records.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SnapshotFormat {
    #[default]
    CompactJson,
    PrettyJson,
    Ndjson,
}

/// Existing CLI summary shapes, each bounded while it is produced.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SummaryFormat {
    #[default]
    Text,
    Json,
    Kv,
}

struct View<'a> {
    log: &'a CVLog,
    entries: Vec<(&'a String, &'a CVEntry)>,
    records: Vec<&'a JournalRecord>,
    filtered: bool,
    context: Context,
    output_cap: usize,
}
impl<'a> View<'a> {
    fn new(log: &'a CVLog, filter: SourceFilter<'_>) -> Result<Self, String> {
        let (limits, work) = log.checked_export_work()?;
        let context = Context(RefCell::new(work));
        // Charge before allocating a reference slot or sorting any index. Filter
        // input itself is borrowed; its traversal also requires a work allowance.
        context.0.borrow_mut().take(filter.sources.len())?;
        context.0.borrow_mut().take(log.entries.len())?;
        let mut entries = Vec::with_capacity(log.entries.len());
        for (id, entry) in &log.entries {
            if filter.sources.is_empty() || selected(entry, filter, &context)? {
                entries.push((id, entry));
            }
        }
        context.0.borrow_mut().take(entries.len())?;
        entries.sort_unstable_by(|a, b| a.0.cmp(b.0));
        // The selection index borrows fixed-size entity keys. Charging its
        // construction and each journal visit keeps filtering inside the same
        // work allowance as validation and serialization.
        let mut records = Vec::new();
        if let Some(journal) = &log.journal {
            context.0.borrow_mut().take(entries.len())?;
            let selected: std::collections::HashSet<_> =
                entries.iter().map(|(id, _)| id.as_str()).collect();
            for record in journal.events() {
                context.0.borrow_mut().take(1)?;
                let entity = record.event.entity();
                let id = format!("cv1.{:016x}", entity.sequence());
                if filter.sources.is_empty() || selected.contains(id.as_str()) {
                    records.push(record);
                }
            }
        }
        Ok(Self {
            log,
            entries,
            records,
            filtered: !filter.sources.is_empty(),
            context,
            output_cap: limits.max_output_bytes,
        })
    }
    fn writer(&self) -> BoundedWriter {
        BoundedWriter {
            bytes: Vec::new(),
            cap: self.output_cap,
        }
    }
}

fn matches(source: &str, filter: SourceFilter<'_>, context: &Context) -> Result<bool, String> {
    for candidate in filter.sources {
        context.0.borrow_mut().take(1)?;
        if source == candidate {
            return Ok(true);
        }
    }
    Ok(false)
}
fn selected(entry: &CVEntry, filter: SourceFilter<'_>, context: &Context) -> Result<bool, String> {
    context.0.borrow_mut().take(1)?;
    let mut found = false;
    for event in &entry.contributions {
        context.0.borrow_mut().take(1)?;
        if matches(&event.source, filter, context)? {
            found = true;
            break;
        }
    }
    if !found && filter.include_origin {
        if let Some(origin) = &entry.origin {
            context.0.borrow_mut().take(1)?;
            found = matches(&origin.source, filter, context)?;
        }
    }
    Ok(if filter.invert { !found } else { found })
}

struct Root<'a>(&'a View<'a>, bool);
impl Serialize for Root<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let view = self.0;
        view.context.take::<S::Error>(1)?;
        let fields = 2
            + usize::from(self.1)
            + usize::from(view.log.compact_sequence.is_some())
            + usize::from(view.log.journal.is_some())
            + usize::from(view.filtered);
        let mut map = serializer.serialize_map(Some(fields))?;
        map.serialize_entry("enabled", &view.log.enabled)?;
        if self.1 {
            map.serialize_entry("entries", &Entries(view))?;
        }
        if let Some(last) = view.log.compact_sequence {
            map.serialize_entry("identity", &Identity(last, &view.context))?;
        }
        if view.log.journal.is_some() {
            map.serialize_entry("journal", &JournalView(view, self.1))?;
        }
        view.context.take::<S::Error>(view.log.pass_order.len())?;
        map.serialize_entry("pass_order", &view.log.pass_order)?;
        if view.filtered {
            map.serialize_entry("view", &Projection(&view.context))?;
        }
        map.end()
    }
}
struct Entries<'a>(&'a View<'a>);
/// JSON retains selected records; the NDJSON footer retains only their common
/// version, declared coverage and original watermark. Gaps never get renumbered.
struct JournalView<'a>(&'a View<'a>, bool);
impl Serialize for JournalView<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let view = self.0;
        view.context.take::<S::Error>(1)?;
        let mut map = serializer.serialize_map(Some(3 + usize::from(self.1)))?;
        map.serialize_entry("coverage", if view.filtered { "partial" } else { "full" })?;
        if self.1 {
            map.serialize_entry("events", &Records(view))?;
        }
        map.serialize_entry(
            "last_sequence",
            &JournalSequence(
                view.log
                    .journal
                    .as_ref()
                    .expect("journal view")
                    .last_sequence(),
            ),
        )?;
        map.serialize_entry("version", "chronology-v1")?;
        map.end()
    }
}
struct Records<'a>(&'a View<'a>);
impl Serialize for Records<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.context.take::<S::Error>(self.0.records.len())?;
        let mut seq = serializer.serialize_seq(Some(self.0.records.len()))?;
        for record in &self.0.records {
            seq.serialize_element(&Canonical(*record, &self.0.context))?;
        }
        seq.end()
    }
}
struct EventFrame<'a>(&'a JournalRecord, &'a Context);
impl Serialize for EventFrame<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.1.take::<S::Error>(1)?;
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry("_event", &Canonical(self.0, self.1))?;
        map.end()
    }
}
impl Serialize for Entries<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.context.take::<S::Error>(self.0.entries.len())?;
        let mut map = serializer.serialize_map(Some(self.0.entries.len()))?;
        for (id, entry) in &self.0.entries {
            map.serialize_entry(id, &Canonical(*entry, &self.0.context))?;
        }
        map.end()
    }
}
struct Projection<'a>(&'a Context);
impl Serialize for Projection<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.take::<S::Error>(3)?;
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("complete", &false)?;
        map.serialize_entry("filtered", &true)?;
        map.end()
    }
}
struct Footer<'a>(&'a View<'a>);
impl Serialize for Footer<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.context.take::<S::Error>(1)?;
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry("_meta", &Root(self.0, false))?;
        map.end()
    }
}

fn serialization(error: impl std::fmt::Display) -> String {
    format!("CV serialization error: {error}")
}
fn finish(writer: BoundedWriter) -> Result<String, String> {
    String::from_utf8(writer.bytes).map_err(serialization)
}

impl CVLog {
    /// Validate complete evidence, then export a canonical, byte-bounded view.
    /// A filtered result is explicitly partial and cannot be checked-imported.
    pub fn export_snapshot(
        &self,
        format: SnapshotFormat,
        filter: SourceFilter<'_>,
    ) -> Result<String, String> {
        let view = View::new(self, filter)?;
        let mut writer = view.writer();
        match format {
            SnapshotFormat::CompactJson => {
                serde_json::to_writer(&mut writer, &Root(&view, true)).map_err(serialization)?
            }
            SnapshotFormat::PrettyJson => {
                serde_json::to_writer_pretty(&mut writer, &Root(&view, true))
                    .map_err(serialization)?
            }
            SnapshotFormat::Ndjson => {
                for (_, entry) in &view.entries {
                    serde_json::to_writer(&mut writer, &Canonical(*entry, &view.context))
                        .map_err(serialization)?;
                    writer.write_all(b"\n").map_err(serialization)?;
                }
                for record in &view.records {
                    serde_json::to_writer(&mut writer, &EventFrame(record, &view.context))
                        .map_err(serialization)?;
                    writer.write_all(b"\n").map_err(serialization)?;
                }
                serde_json::to_writer(&mut writer, &Footer(&view)).map_err(serialization)?;
                writer.write_all(b"\n").map_err(serialization)?;
            }
        }
        finish(writer)
    }

    /// Count a checked view and stream the existing sidecar summary wire shape.
    /// The caller supplies a path only if its publication transaction succeeds.
    pub fn export_summary(
        &self,
        format: SummaryFormat,
        filter: SourceFilter<'_>,
        wrote_path: Option<&str>,
    ) -> Result<String, String> {
        let view = View::new(self, filter)?;
        let mut contributions = 0usize;
        let mut tombstones = 0usize;
        for (_, entry) in &view.entries {
            view.context.0.borrow_mut().take(1)?;
            contributions = contributions
                .checked_add(entry.contributions.len())
                .ok_or("CV summary contribution count overflow")?;
            tombstones = tombstones
                .checked_add(usize::from(entry.deleted.is_some()))
                .ok_or("CV summary tombstone count overflow")?;
        }
        view.context.0.borrow_mut().take(self.pass_order.len())?;
        let summary = Summary {
            view: &view,
            contributions,
            tombstones,
            path: wrote_path,
        };
        let mut writer = view.writer();
        match format {
            SummaryFormat::Json => {
                serde_json::to_writer(&mut writer, &summary).map_err(serialization)?
            }
            SummaryFormat::Text => {
                match wrote_path {
                    Some(path) => write!(writer, "cv sidecar: {path}"),
                    None => writer.write_all(b"cv sidecar: skipped (format=NONE)"),
                }
                .map_err(serialization)?;
                write!(writer, ": {} entries, {contributions} contributions, {tombstones} tombstones, pass_order=[",view.entries.len()).map_err(serialization)?;
                write_stages(&mut writer, &self.pass_order, false)?;
                writer.write_all(b"]").map_err(serialization)?;
            }
            SummaryFormat::Kv => {
                writer
                    .write_all(b"cv_sidecar.path=")
                    .map_err(serialization)?;
                serde_json::to_writer(&mut writer, wrote_path.unwrap_or(""))
                    .map_err(serialization)?;
                write!(writer," cv_sidecar.skipped={} cv_sidecar.entries={} cv_sidecar.contributions={contributions} cv_sidecar.tombstones={tombstones} cv_sidecar.pass_order=\"",wrote_path.is_none(),view.entries.len()).map_err(serialization)?;
                write_stages(&mut writer, &self.pass_order, true)?;
                writer.write_all(b"\"").map_err(serialization)?;
            }
        }
        writer.write_all(b"\n").map_err(serialization)?;
        finish(writer)
    }
}

// Text and KV preserve comma-joined stage order without allocating that joined
// string. KV escapes each character inside a single JSON string literal.
fn write_stages(writer: &mut BoundedWriter, stages: &[String], quoted: bool) -> Result<(), String> {
    for (index, stage) in stages.iter().enumerate() {
        if index != 0 {
            writer.write_all(b",").map_err(serialization)?;
        }
        if !quoted {
            writer.write_all(stage.as_bytes()).map_err(serialization)?;
            continue;
        }
        for ch in stage.chars() {
            match ch {
                '"' => writer.write_all(b"\\\""),
                '\\' => writer.write_all(b"\\\\"),
                '\n' => writer.write_all(b"\\n"),
                '\r' => writer.write_all(b"\\r"),
                '\t' => writer.write_all(b"\\t"),
                '\u{8}' => writer.write_all(b"\\b"),
                '\u{c}' => writer.write_all(b"\\f"),
                ch if ch < '\u{20}' => write!(writer, "\\u{:04x}", ch as u32),
                ch => writer.write_all(ch.encode_utf8(&mut [0; 4]).as_bytes()),
            }
            .map_err(serialization)?;
        }
    }
    Ok(())
}

struct Summary<'a> {
    view: &'a View<'a>,
    contributions: usize,
    tombstones: usize,
    path: Option<&'a str>,
}
impl Serialize for Summary<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.view.context.take::<S::Error>(1)?;
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry("cv_sidecar", &SummaryFields(self))?;
        map.end()
    }
}
struct SummaryFields<'a>(&'a Summary<'a>);
impl Serialize for SummaryFields<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.view.context.take::<S::Error>(1)?;
        let mut map = serializer.serialize_map(Some(6))?;
        map.serialize_entry("contributions", &self.0.contributions)?;
        map.serialize_entry("entries", &self.0.view.entries.len())?;
        map.serialize_entry("pass_order", &self.0.view.log.pass_order)?;
        map.serialize_entry("path", &self.0.path)?;
        map.serialize_entry("skipped", &self.0.path.is_none())?;
        map.serialize_entry("tombstones", &self.0.tombstones)?;
        map.end()
    }
}
