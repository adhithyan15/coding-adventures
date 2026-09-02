//! Reading Anki's **schema 18** collections.
//!
//! Modern `.anki21b` and `.colpkg` packages are not merely V11 with zstd
//! wrapped round them. The compression is the visible difference; the schema
//! underneath is a different database.
//!
//! ## What moved
//!
//! In **V11**, the `col` table's `conf`, `models`, `decks` and `dconf` columns
//! hold JSON blobs carrying every note type, deck, and setting in the
//! collection. In **schema 18** those columns are **empty**, and the same
//! information lives in relational tables:
//!
//! ```text
//!   col.models  ->  notetypes + fields + templates
//!   col.decks   ->  decks + deck_config
//!   col.conf    ->  config
//!   col.tags    ->  tags
//! ```
//!
//! So an importer that parses `col.conf` as JSON does not get a slightly
//! different answer — it gets `EOF while parsing a value at line 1 column 0`,
//! because the string is empty. That was the observed failure.
//!
//! ## Two things that make this awkward
//!
//! **Four of those tables are `WITHOUT ROWID`** (`fields`, `templates`,
//! `config`, `tags`), which SQLite stores as *index* b-trees rather than table
//! b-trees. They need `sqlite_file::read_without_rowid_table`; calling the
//! ordinary reader on one returns `unexpected b-tree page type`, correctly.
//!
//! **The config columns are protobuf**, not JSON. V11 stored templates and CSS
//! as JSON text; schema 18 stores Anki's own protobuf messages. Only a few
//! fields are needed, so this module reads them by number rather than
//! generating message types — Anki's `.proto` definitions move between
//! versions, and a full generated binding would couple us to a specific one.
//!
//! ## Where the field numbers came from
//!
//! **Decoded from real Anki output, not from memory.** Each number below was
//! read out of an archive produced by Anki 26.08.1 and cross-checked against
//! what the collection visibly contained. The cloze discriminator in particular
//! was settled by exporting a collection holding one Basic and one Cloze note
//! and diffing the two `notetypes.config` blobs — see [`NOTETYPE_KIND`].

use serde_json::{json, Value};

use crate::{
    apkg_error, AnkiV11Deck, AnkiV11Field, AnkiV11NoteType, AnkiV11Template, ApkgError,
};

/// `NotetypeConfig.kind`: 0 (or absent) is a normal note type, 1 is cloze.
///
/// The absence is the subtle part. Protobuf omits fields holding their default,
/// so a **normal note type carries no `kind` field at all** — Basic's config has
/// no field 1, while Cloze's has `1`. Treating a missing field as "unknown"
/// rather than "normal" would misclassify every ordinary note type in the
/// collection.
///
/// Established by exporting a collection with one Basic and one Cloze note and
/// comparing the blobs:
///
/// ```text
///   Basic   varint fields: {9: 1}
///   Cloze   varint fields: {1: 1, 9: 5}
/// ```
pub(crate) const NOTETYPE_KIND: u32 = 1;
/// `NotetypeConfig.css`.
const NOTETYPE_CSS: u32 = 3;
/// `CardTemplateConfig.q_format` — the question side.
const TEMPLATE_QFMT: u32 = 1;
/// `CardTemplateConfig.a_format` — the answer side.
const TEMPLATE_AFMT: u32 = 2;

/// Anki's schema version at which configuration left the `col` table.
///
/// Collections at or above this are read by this module; below it, the V11
/// path applies. Anki has shipped 11 → 18 with no stable intermediate that
/// appears in exported packages, so this is a boundary rather than a range.
pub(crate) const SCHEMA_18: i64 = 18;

/// Read one length-delimited protobuf field as a UTF-8 string.
///
/// Returns `None` when the field is absent, which for protobuf means it held
/// its default — the empty string. Callers that want `""` should say so, rather
/// than this function guessing on their behalf.
fn string_field(blob: &[u8], field: u32) -> Option<String> {
    let mut reader = protobuf::Reader::new(blob);
    while let Ok(Some(found)) = reader.next_field() {
        if found.number == field {
            if let Some(bytes) = found.value.as_bytes() {
                return Some(String::from_utf8_lossy(bytes).into_owned());
            }
        }
    }
    None
}

/// Read one varint protobuf field.
///
/// `None` means absent, and absent means the protobuf default of zero. That
/// distinction matters for [`NOTETYPE_KIND`], where "absent" is how every
/// normal note type is encoded.
fn varint_field(blob: &[u8], field: u32) -> Option<u64> {
    let mut reader = protobuf::Reader::new(blob);
    while let Ok(Some(found)) = reader.next_field() {
        if found.number == field {
            if let Some(value) = found.value.as_varint() {
                return Some(value);
            }
        }
    }
    None
}

/// Pull a text column out of a decoded SQLite row.
fn text(row: &[sqlite_file::SqlValue], index: usize, label: &str) -> Result<String, ApkgError> {
    match row.get(index) {
        Some(sqlite_file::SqlValue::Text(value)) => Ok(value.clone()),
        Some(sqlite_file::SqlValue::Null) | None => Ok(String::new()),
        Some(other) => Err(apkg_error(format!(
            "Anki schema 18 {label} should be text, found {other:?}"
        ))),
    }
}

/// Pull an integer column out of a decoded SQLite row.
fn int(row: &[sqlite_file::SqlValue], index: usize, label: &str) -> Result<i64, ApkgError> {
    match row.get(index) {
        Some(sqlite_file::SqlValue::Int(value)) => Ok(*value),
        Some(sqlite_file::SqlValue::Null) | None => Ok(0),
        Some(other) => Err(apkg_error(format!(
            "Anki schema 18 {label} should be an integer, found {other:?}"
        ))),
    }
}

/// Pull a blob column out of a decoded SQLite row.
fn blob(row: &[sqlite_file::SqlValue], index: usize) -> Vec<u8> {
    match row.get(index) {
        Some(sqlite_file::SqlValue::Blob(bytes)) => bytes.clone(),
        Some(sqlite_file::SqlValue::Text(value)) => value.as_bytes().to_vec(),
        _ => Vec::new(),
    }
}

/// Read the note types from `notetypes` + `fields` + `templates`.
///
/// Three tables, and two of them are `WITHOUT ROWID`, so they come through a
/// different reader. Names and ordinals live in ordinary columns; only the CSS,
/// the two template formats, and the cloze discriminator need protobuf.
pub(crate) fn read_note_types(db: &[u8]) -> Result<Vec<AnkiV11NoteType>, ApkgError> {
    let notetypes = sqlite_file::read_table(db, "notetypes")
        .map_err(|err| apkg_error(format!("Anki schema 18 notetypes table: {err:?}")))?;
    // `fields` and `templates` are WITHOUT ROWID -- index b-trees, not table
    // b-trees. The ordinary reader refuses them, correctly.
    let fields = sqlite_file::read_without_rowid_table(db, "fields")
        .map_err(|err| apkg_error(format!("Anki schema 18 fields table: {err:?}")))?;
    let templates = sqlite_file::read_without_rowid_table(db, "templates")
        .map_err(|err| apkg_error(format!("Anki schema 18 templates table: {err:?}")))?;

    let mut out = Vec::with_capacity(notetypes.len());
    for (id, row) in &notetypes {
        let name = text(row, 1, "notetypes.name")?;
        let config = blob(row, 4);

        // Absent `kind` means normal -- protobuf omits defaults, so every
        // ordinary note type simply has no field 1.
        let kind = varint_field(&config, NOTETYPE_KIND).unwrap_or(0) as i64;
        let css = string_field(&config, NOTETYPE_CSS).unwrap_or_default();

        let mut note_fields: Vec<AnkiV11Field> = fields
            .iter()
            .filter(|row| int(row, 0, "fields.ntid").map(|v| v == *id).unwrap_or(false))
            .map(|row| {
                Ok(AnkiV11Field {
                    ordinal: int(row, 1, "fields.ord")?,
                    name: text(row, 2, "fields.name")?,
                })
            })
            .collect::<Result<_, ApkgError>>()?;
        note_fields.sort_by_key(|field| field.ordinal);

        let mut note_templates: Vec<AnkiV11Template> = templates
            .iter()
            .filter(|row| {
                int(row, 0, "templates.ntid")
                    .map(|v| v == *id)
                    .unwrap_or(false)
            })
            .map(|row| {
                let config = blob(row, 5);
                Ok(AnkiV11Template {
                    ordinal: int(row, 1, "templates.ord")?,
                    name: text(row, 2, "templates.name")?,
                    question_format: string_field(&config, TEMPLATE_QFMT).unwrap_or_default(),
                    answer_format: string_field(&config, TEMPLATE_AFMT).unwrap_or_default(),
                    deck_id: None,
                })
            })
            .collect::<Result<_, ApkgError>>()?;
        note_templates.sort_by_key(|template| template.ordinal);

        out.push(AnkiV11NoteType {
            id: *id,
            name,
            kind,
            css,
            fields: note_fields,
            templates: note_templates,
            // The V11 path carries the note type's original JSON here so
            // nothing is silently dropped on re-export. There is no equivalent
            // JSON in schema 18, so this records what was actually read rather
            // than fabricating a V11-shaped object that was never in the file.
            raw: json!({ "schema": 18, "id": *id }),
        });
    }
    out.sort_by_key(|note_type| note_type.id);
    Ok(out)
}

/// Read the decks from the `decks` table.
///
/// `decks.common` and `decks.kind` are protobuf, but the deck's **name** — the
/// only field the V11 shape needs beyond its id — is an ordinary column.
///
/// One difference worth knowing: schema 18 stores nested deck names with `\x1f`
/// as the separator, where V11 used `::`. Importers that split on `::` see one
/// deck named `Parent\x1fChild` rather than a hierarchy, so the separator is
/// normalised here.
pub(crate) fn read_decks(db: &[u8]) -> Result<Vec<AnkiV11Deck>, ApkgError> {
    let rows = sqlite_file::read_table(db, "decks")
        .map_err(|err| apkg_error(format!("Anki schema 18 decks table: {err:?}")))?;
    let mut out = Vec::with_capacity(rows.len());
    for (id, row) in &rows {
        let raw_name = text(row, 1, "decks.name")?;
        out.push(AnkiV11Deck {
            id: *id,
            name: raw_name.replace('\u{1f}', "::"),
            description: String::new(),
            raw: json!({ "schema": 18, "id": *id }),
        });
    }
    out.sort_by_key(|deck| deck.id);
    Ok(out)
}

/// Read the `config` table into the JSON object V11 kept in `col.conf`.
///
/// `config.val` is the one column in this area that really is JSON, so values
/// pass through as parsed JSON where they parse and as strings where they do
/// not — a config value we cannot interpret is still worth carrying, since
/// dropping it silently loses collection settings on a round trip.
pub(crate) fn read_config(db: &[u8]) -> Result<Value, ApkgError> {
    let rows = sqlite_file::read_without_rowid_table(db, "config")
        .map_err(|err| apkg_error(format!("Anki schema 18 config table: {err:?}")))?;
    let mut map = serde_json::Map::new();
    for row in &rows {
        let key = text(row, 0, "config.KEY")?;
        let raw = blob(row, 3);
        let text_value = String::from_utf8_lossy(&raw);
        let value = serde_json::from_str::<Value>(&text_value)
            .unwrap_or_else(|_| Value::String(text_value.into_owned()));
        map.insert(key, value);
    }
    Ok(Value::Object(map))
}

/// Read the `tags` table into the JSON object V11 kept in `col.tags`.
pub(crate) fn read_tags(db: &[u8]) -> Result<Value, ApkgError> {
    let rows = sqlite_file::read_without_rowid_table(db, "tags")
        .map_err(|err| apkg_error(format!("Anki schema 18 tags table: {err:?}")))?;
    let mut map = serde_json::Map::new();
    for row in &rows {
        let tag = text(row, 0, "tags.tag")?;
        map.insert(tag, Value::from(0));
    }
    Ok(Value::Object(map))
}

/// Read the `graves` table, which is `WITHOUT ROWID` at schema 18.
///
/// V11 gives `graves` an ordinary rowid; schema 18 declares it `WITHOUT ROWID`,
/// so the same table needs a different reader. That is easy to miss precisely
/// because the *columns* are identical — the V11 reader parses the row layout
/// correctly and fails one level down, on the b-tree page type, with
/// `unexpected b-tree page type`.
///
/// Worth noting which tables did **not** move: `notes`, `cards` and `revlog`
/// keep their rowids, so the readers for the bulk of a collection are shared
/// between the two schemas unchanged.
pub(crate) fn read_graves(db: &[u8]) -> Result<Vec<crate::AnkiV11Grave>, ApkgError> {
    let rows = sqlite_file::read_without_rowid_table(db, "graves")
        .map_err(|err| apkg_error(format!("Anki schema 18 graves table: {err:?}")))?;
    rows.iter()
        .map(|row| {
            Ok(crate::AnkiV11Grave {
                // Column order matches V11's: (usn, oid, type).
                update_sequence_number: int(row, 0, "graves.usn")?,
                object_id: int(row, 1, "graves.oid")?,
                kind: int(row, 2, "graves.type")?,
            })
        })
        .collect()
}
