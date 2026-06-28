//! APKG archive inspection for Engram.
//!
//! This crate intentionally stops at the archive boundary: it identifies the
//! Anki collection member and media mapping inside an `.apkg`/`.colpkg` zip
//! archive, but leaves SQLite collection import/export to a later layer.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use zip::{ZipReader, ZipWriter};

const LEGACY_COLLECTION: &str = "collection.anki2";
const SQLITE_21_COLLECTION: &str = "collection.anki21";
const SQLITE_21B_COLLECTION: &str = "collection.anki21b";
const MEDIA_MAP: &str = "media";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnkiPackageManifest {
    pub collection: CollectionMember,
    pub media: MediaManifest,
    pub entries: Vec<ArchiveEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionMember {
    pub name: String,
    pub format: CollectionFormat,
    pub size: u32,
    pub compressed_size: u32,
    pub compression_method: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CollectionFormat {
    LegacySqlite,
    Sqlite21,
    Sqlite21b,
}

impl CollectionFormat {
    pub fn is_v11_sqlite(self) -> bool {
        matches!(self, Self::LegacySqlite | Self::Sqlite21)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaManifest {
    pub map_present: bool,
    pub mapping: BTreeMap<String, String>,
    pub media_files: Vec<MediaFile>,
    pub missing_files: Vec<String>,
    pub unmapped_files: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaFile {
    pub archive_name: String,
    pub filename: Option<String>,
    pub size: u32,
    pub compressed_size: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedMediaFile {
    pub archive_name: String,
    pub filename: Option<String>,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveEntry {
    pub name: String,
    pub size: u32,
    pub compressed_size: u32,
    pub compression_method: u16,
    pub directory: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApkgError {
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MediaAsset<'a> {
    pub filename: &'a str,
    pub data: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnkiV11Collection {
    pub metadata: AnkiV11CollectionMetadata,
    pub decks: Vec<AnkiV11Deck>,
    pub note_types: Vec<AnkiV11NoteType>,
    pub notes: Vec<AnkiV11Note>,
    pub cards: Vec<AnkiV11Card>,
    pub reviews: Vec<AnkiV11Review>,
    pub graves: Vec<AnkiV11Grave>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnkiV11CollectionMetadata {
    pub id: i64,
    pub created_at_days: i64,
    pub modified_at: i64,
    pub schema_modified_at: i64,
    pub version: i64,
    pub dirty: i64,
    pub update_sequence_number: i64,
    pub last_sync: i64,
    pub config: Value,
    pub deck_config: Value,
    pub tags: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnkiV11Deck {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub raw: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnkiV11NoteType {
    pub id: i64,
    pub name: String,
    pub kind: i64,
    pub css: String,
    pub fields: Vec<AnkiV11Field>,
    pub templates: Vec<AnkiV11Template>,
    pub raw: Value,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnkiV11Field {
    pub ordinal: i64,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnkiV11Template {
    pub ordinal: i64,
    pub name: String,
    pub question_format: String,
    pub answer_format: String,
    pub deck_id: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnkiV11Note {
    pub id: i64,
    pub guid: String,
    pub note_type_id: i64,
    pub modified_at: i64,
    pub update_sequence_number: i64,
    pub tags: Vec<String>,
    pub field_values: Vec<String>,
    pub sort_field: String,
    pub checksum: i64,
    pub flags: i64,
    pub data: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnkiV11Card {
    pub id: i64,
    pub note_id: i64,
    pub deck_id: i64,
    pub ordinal: i64,
    pub modified_at: i64,
    pub update_sequence_number: i64,
    pub kind: i64,
    pub queue: i64,
    pub due: i64,
    pub interval: i64,
    pub factor: i64,
    pub repetitions: i64,
    pub lapses: i64,
    pub left: i64,
    pub original_due: i64,
    pub original_deck_id: i64,
    pub flags: i64,
    pub data: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnkiV11Review {
    pub id: i64,
    pub card_id: i64,
    pub update_sequence_number: i64,
    pub ease: i64,
    pub interval: i64,
    pub last_interval: i64,
    pub factor: i64,
    pub time: i64,
    pub kind: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnkiV11Grave {
    pub update_sequence_number: i64,
    pub object_id: i64,
    pub kind: i64,
}

impl fmt::Display for ApkgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ApkgError {}

pub fn inspect_apkg(data: &[u8]) -> Result<AnkiPackageManifest, ApkgError> {
    let reader = ZipReader::new(data).map_err(apkg_error)?;
    let entries = archive_entries(&reader);
    let collection = collection_member(&entries)?;
    let media = media_manifest(&reader)?;

    Ok(AnkiPackageManifest {
        collection,
        media,
        entries,
    })
}

pub fn read_collection_bytes(data: &[u8]) -> Result<Vec<u8>, ApkgError> {
    let reader = ZipReader::new(data).map_err(apkg_error)?;
    let entries = archive_entries(&reader);
    let collection = collection_member(&entries)?;
    reader
        .read_by_name(&collection.name)
        .map_err(|err| apkg_error(format!("failed to read collection: {err}")))
}

pub fn read_v11_collection_bytes(data: &[u8]) -> Result<Vec<u8>, ApkgError> {
    let reader = ZipReader::new(data).map_err(apkg_error)?;
    let entries = archive_entries(&reader);
    let collection = collection_member(&entries)?;
    if !collection.format.is_v11_sqlite() {
        return Err(apkg_error(format!(
            "{} uses Anki's modern package format; the V11 collection reader supports collection.anki2 and collection.anki21 only",
            collection.name
        )));
    }

    reader
        .read_by_name(&collection.name)
        .map_err(|err| apkg_error(format!("failed to read collection: {err}")))
}

pub fn read_media_file(data: &[u8], archive_name: &str) -> Result<ResolvedMediaFile, ApkgError> {
    let reader = ZipReader::new(data).map_err(apkg_error)?;
    let entries = archive_entries(&reader);
    collection_member(&entries)?;
    let manifest = media_manifest(&reader)?;
    let media = manifest
        .media_files
        .into_iter()
        .find(|media| media.archive_name == archive_name)
        .ok_or_else(|| apkg_error(format!("media file '{archive_name}' not found")))?;
    let data = reader.read_by_name(&media.archive_name).map_err(|err| {
        apkg_error(format!(
            "failed to read media file '{}': {err}",
            media.archive_name
        ))
    })?;

    Ok(ResolvedMediaFile {
        archive_name: media.archive_name,
        filename: media.filename,
        data,
    })
}

pub fn read_media_files(data: &[u8]) -> Result<Vec<ResolvedMediaFile>, ApkgError> {
    let reader = ZipReader::new(data).map_err(apkg_error)?;
    let entries = archive_entries(&reader);
    collection_member(&entries)?;
    let manifest = media_manifest(&reader)?;

    manifest
        .media_files
        .into_iter()
        .map(|media| {
            let data = reader.read_by_name(&media.archive_name).map_err(|err| {
                apkg_error(format!(
                    "failed to read media file '{}': {err}",
                    media.archive_name
                ))
            })?;
            Ok(ResolvedMediaFile {
                archive_name: media.archive_name,
                filename: media.filename,
                data,
            })
        })
        .collect()
}

pub fn write_legacy_apkg(collection_anki2: &[u8], media_assets: &[MediaAsset<'_>]) -> Vec<u8> {
    let mut writer = ZipWriter::new();
    writer.add_file(LEGACY_COLLECTION, collection_anki2, false);

    let media_map: BTreeMap<String, String> = media_assets
        .iter()
        .enumerate()
        .map(|(index, asset)| (index.to_string(), asset.filename.to_string()))
        .collect();
    let media_json = serde_json::to_vec(&media_map).unwrap_or_else(|_| b"{}".to_vec());
    writer.add_file(MEDIA_MAP, &media_json, false);

    for (index, asset) in media_assets.iter().enumerate() {
        writer.add_file(&index.to_string(), asset.data, false);
    }

    writer.finish()
}

pub fn read_v11_collection(data: &[u8]) -> Result<AnkiV11Collection, ApkgError> {
    let collection = read_v11_collection_bytes(data)?;
    parse_v11_collection_bytes(&collection)
}

pub fn parse_v11_collection_bytes(bytes: &[u8]) -> Result<AnkiV11Collection, ApkgError> {
    let sqlite_file = tempfile::NamedTempFile::new()
        .map_err(|err| apkg_error(format!("failed to create temporary SQLite file: {err}")))?;
    std::fs::write(sqlite_file.path(), bytes)
        .map_err(|err| apkg_error(format!("failed to write temporary SQLite file: {err}")))?;
    let connection = Connection::open_with_flags(
        sqlite_file.path(),
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|err| apkg_error(format!("failed to open Anki V11 SQLite collection: {err}")))?;

    let raw_col = read_v11_col_row(&connection)?;
    let metadata = AnkiV11CollectionMetadata {
        id: raw_col.id,
        created_at_days: raw_col.created_at_days,
        modified_at: raw_col.modified_at,
        schema_modified_at: raw_col.schema_modified_at,
        version: raw_col.version,
        dirty: raw_col.dirty,
        update_sequence_number: raw_col.update_sequence_number,
        last_sync: raw_col.last_sync,
        config: parse_json_value("col.conf", &raw_col.config_json)?,
        deck_config: parse_json_value("col.dconf", &raw_col.deck_config_json)?,
        tags: parse_json_value("col.tags", &raw_col.tags_json)?,
    };

    Ok(AnkiV11Collection {
        decks: parse_v11_decks(&raw_col.decks_json)?,
        note_types: parse_v11_note_types(&raw_col.models_json)?,
        notes: read_v11_notes(&connection)?,
        cards: read_v11_cards(&connection)?,
        reviews: read_v11_reviews(&connection)?,
        graves: read_v11_graves(&connection)?,
        metadata,
    })
}

#[derive(Debug)]
struct RawV11ColRow {
    id: i64,
    created_at_days: i64,
    modified_at: i64,
    schema_modified_at: i64,
    version: i64,
    dirty: i64,
    update_sequence_number: i64,
    last_sync: i64,
    config_json: String,
    models_json: String,
    decks_json: String,
    deck_config_json: String,
    tags_json: String,
}

fn read_v11_col_row(connection: &Connection) -> Result<RawV11ColRow, ApkgError> {
    connection
        .query_row(
            "SELECT id, crt, mod, scm, ver, dty, usn, ls, conf, models, decks, dconf, tags FROM col LIMIT 1",
            [],
            |row| {
                Ok(RawV11ColRow {
                    id: row.get(0)?,
                    created_at_days: row.get(1)?,
                    modified_at: row.get(2)?,
                    schema_modified_at: row.get(3)?,
                    version: row.get(4)?,
                    dirty: row.get(5)?,
                    update_sequence_number: row.get(6)?,
                    last_sync: row.get(7)?,
                    config_json: row.get(8)?,
                    models_json: row.get(9)?,
                    decks_json: row.get(10)?,
                    deck_config_json: row.get(11)?,
                    tags_json: row.get(12)?,
                })
            },
        )
        .map_err(|err| apkg_error(format!("failed to read Anki V11 col table: {err}")))
}

fn parse_v11_decks(json: &str) -> Result<Vec<AnkiV11Deck>, ApkgError> {
    let value = parse_json_value("col.decks", json)?;
    let object = value
        .as_object()
        .ok_or_else(|| apkg_error("col.decks must be a JSON object"))?;
    let mut decks = Vec::with_capacity(object.len());
    for (key, raw) in object {
        decks.push(AnkiV11Deck {
            id: json_i64(raw, "id")
                .or_else(|| key.parse().ok())
                .unwrap_or(0),
            name: json_string(raw, "name").unwrap_or_default(),
            description: json_string(raw, "desc").unwrap_or_default(),
            raw: raw.clone(),
        });
    }
    decks.sort_by_key(|deck| deck.id);
    Ok(decks)
}

fn parse_v11_note_types(json: &str) -> Result<Vec<AnkiV11NoteType>, ApkgError> {
    let value = parse_json_value("col.models", json)?;
    let object = value
        .as_object()
        .ok_or_else(|| apkg_error("col.models must be a JSON object"))?;
    let mut note_types = Vec::with_capacity(object.len());
    for (key, raw) in object {
        note_types.push(AnkiV11NoteType {
            id: json_i64(raw, "id")
                .or_else(|| key.parse().ok())
                .unwrap_or(0),
            name: json_string(raw, "name").unwrap_or_default(),
            kind: json_i64(raw, "type").unwrap_or(0),
            css: json_string(raw, "css").unwrap_or_default(),
            fields: parse_v11_fields(raw),
            templates: parse_v11_templates(raw),
            raw: raw.clone(),
        });
    }
    note_types.sort_by_key(|note_type| note_type.id);
    Ok(note_types)
}

fn parse_v11_fields(raw_model: &Value) -> Vec<AnkiV11Field> {
    let mut fields = raw_model
        .get("flds")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(index, raw)| AnkiV11Field {
            ordinal: json_i64(raw, "ord").unwrap_or(index as i64),
            name: json_string(raw, "name").unwrap_or_default(),
        })
        .collect::<Vec<_>>();
    fields.sort_by_key(|field| field.ordinal);
    fields
}

fn parse_v11_templates(raw_model: &Value) -> Vec<AnkiV11Template> {
    let mut templates = raw_model
        .get("tmpls")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(index, raw)| AnkiV11Template {
            ordinal: json_i64(raw, "ord").unwrap_or(index as i64),
            name: json_string(raw, "name").unwrap_or_default(),
            question_format: json_string(raw, "qfmt").unwrap_or_default(),
            answer_format: json_string(raw, "afmt").unwrap_or_default(),
            deck_id: json_i64(raw, "did"),
        })
        .collect::<Vec<_>>();
    templates.sort_by_key(|template| template.ordinal);
    templates
}

fn read_v11_notes(connection: &Connection) -> Result<Vec<AnkiV11Note>, ApkgError> {
    let mut statement = connection
        .prepare(
            "SELECT id, guid, mid, mod, usn, tags, flds, sfld, csum, flags, data FROM notes ORDER BY id",
        )
        .map_err(|err| apkg_error(format!("failed to prepare Anki V11 notes query: {err}")))?;
    let rows = statement
        .query_map([], |row| {
            let tags: String = row.get(5)?;
            let fields: String = row.get(6)?;
            Ok(AnkiV11Note {
                id: row.get(0)?,
                guid: row.get(1)?,
                note_type_id: row.get(2)?,
                modified_at: row.get(3)?,
                update_sequence_number: row.get(4)?,
                tags: split_anki_tags(&tags),
                field_values: split_anki_fields(&fields),
                sort_field: row.get(7)?,
                checksum: row.get(8)?,
                flags: row.get(9)?,
                data: row.get(10)?,
            })
        })
        .map_err(|err| apkg_error(format!("failed to read Anki V11 notes: {err}")))?;
    collect_sqlite_rows("Anki V11 notes", rows)
}

fn read_v11_cards(connection: &Connection) -> Result<Vec<AnkiV11Card>, ApkgError> {
    let mut statement = connection
        .prepare(
            "SELECT id, nid, did, ord, mod, usn, type, queue, due, ivl, factor, reps, lapses, left, odue, odid, flags, data FROM cards ORDER BY id",
        )
        .map_err(|err| apkg_error(format!("failed to prepare Anki V11 cards query: {err}")))?;
    let rows = statement
        .query_map([], |row| {
            Ok(AnkiV11Card {
                id: row.get(0)?,
                note_id: row.get(1)?,
                deck_id: row.get(2)?,
                ordinal: row.get(3)?,
                modified_at: row.get(4)?,
                update_sequence_number: row.get(5)?,
                kind: row.get(6)?,
                queue: row.get(7)?,
                due: row.get(8)?,
                interval: row.get(9)?,
                factor: row.get(10)?,
                repetitions: row.get(11)?,
                lapses: row.get(12)?,
                left: row.get(13)?,
                original_due: row.get(14)?,
                original_deck_id: row.get(15)?,
                flags: row.get(16)?,
                data: row.get(17)?,
            })
        })
        .map_err(|err| apkg_error(format!("failed to read Anki V11 cards: {err}")))?;
    collect_sqlite_rows("Anki V11 cards", rows)
}

fn read_v11_reviews(connection: &Connection) -> Result<Vec<AnkiV11Review>, ApkgError> {
    let mut statement = connection
        .prepare(
            "SELECT id, cid, usn, ease, ivl, lastIvl, factor, time, type FROM revlog ORDER BY id",
        )
        .map_err(|err| apkg_error(format!("failed to prepare Anki V11 revlog query: {err}")))?;
    let rows = statement
        .query_map([], |row| {
            Ok(AnkiV11Review {
                id: row.get(0)?,
                card_id: row.get(1)?,
                update_sequence_number: row.get(2)?,
                ease: row.get(3)?,
                interval: row.get(4)?,
                last_interval: row.get(5)?,
                factor: row.get(6)?,
                time: row.get(7)?,
                kind: row.get(8)?,
            })
        })
        .map_err(|err| apkg_error(format!("failed to read Anki V11 revlog: {err}")))?;
    collect_sqlite_rows("Anki V11 revlog", rows)
}

fn read_v11_graves(connection: &Connection) -> Result<Vec<AnkiV11Grave>, ApkgError> {
    let mut statement = connection
        .prepare("SELECT usn, oid, type FROM graves ORDER BY oid")
        .map_err(|err| apkg_error(format!("failed to prepare Anki V11 graves query: {err}")))?;
    let rows = statement
        .query_map([], |row| {
            Ok(AnkiV11Grave {
                update_sequence_number: row.get(0)?,
                object_id: row.get(1)?,
                kind: row.get(2)?,
            })
        })
        .map_err(|err| apkg_error(format!("failed to read Anki V11 graves: {err}")))?;
    collect_sqlite_rows("Anki V11 graves", rows)
}

fn collect_sqlite_rows<T>(
    context: &str,
    rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>>,
) -> Result<Vec<T>, ApkgError> {
    let mut values = Vec::new();
    for row in rows {
        values.push(row.map_err(|err| apkg_error(format!("failed to map {context}: {err}")))?);
    }
    Ok(values)
}

fn parse_json_value(field_name: &str, json: &str) -> Result<Value, ApkgError> {
    serde_json::from_str(json)
        .map_err(|err| apkg_error(format!("invalid Anki V11 JSON in {field_name}: {err}")))
}

fn json_i64(value: &Value, key: &str) -> Option<i64> {
    value.get(key).and_then(Value::as_i64)
}

fn json_string(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_string)
}

fn split_anki_fields(fields: &str) -> Vec<String> {
    fields.split('\u{1f}').map(str::to_string).collect()
}

fn split_anki_tags(tags: &str) -> Vec<String> {
    tags.split_whitespace().map(str::to_string).collect()
}

fn archive_entries(reader: &ZipReader<'_>) -> Vec<ArchiveEntry> {
    reader
        .entries()
        .iter()
        .map(|entry| ArchiveEntry {
            name: entry.name.clone(),
            size: entry.size,
            compressed_size: entry.compressed_size,
            compression_method: entry.method,
            directory: entry.is_directory,
        })
        .collect()
}

fn collection_member(entries: &[ArchiveEntry]) -> Result<CollectionMember, ApkgError> {
    let candidates = [
        (LEGACY_COLLECTION, CollectionFormat::LegacySqlite),
        (SQLITE_21_COLLECTION, CollectionFormat::Sqlite21),
        (SQLITE_21B_COLLECTION, CollectionFormat::Sqlite21b),
    ];

    for (name, format) in candidates {
        if let Some(entry) = entries.iter().find(|entry| entry.name == name) {
            return Ok(CollectionMember {
                name: entry.name.clone(),
                format,
                size: entry.size,
                compressed_size: entry.compressed_size,
                compression_method: entry.compression_method,
            });
        }
    }

    Err(apkg_error(
        "Anki package is missing collection.anki2, collection.anki21, or collection.anki21b",
    ))
}

fn media_manifest(reader: &ZipReader<'_>) -> Result<MediaManifest, ApkgError> {
    let mut manifest = MediaManifest::default();
    if let Ok(bytes) = reader.read_by_name(MEDIA_MAP) {
        manifest.map_present = true;
        manifest.mapping = serde_json::from_slice(&bytes)
            .map_err(|err| apkg_error(format!("invalid Anki media map JSON: {err}")))?;
    }

    let mut file_names = BTreeSet::new();
    for entry in reader.entries() {
        if entry.is_directory || is_reserved_entry(&entry.name) {
            continue;
        }
        file_names.insert(entry.name.clone());
        manifest.media_files.push(MediaFile {
            archive_name: entry.name.clone(),
            filename: manifest.mapping.get(&entry.name).cloned(),
            size: entry.size,
            compressed_size: entry.compressed_size,
        });
    }

    for archive_name in manifest.mapping.keys() {
        if !file_names.contains(archive_name) {
            manifest.missing_files.push(archive_name.clone());
        }
    }

    for archive_name in file_names {
        if manifest.map_present && !manifest.mapping.contains_key(&archive_name) {
            manifest.unmapped_files.push(archive_name);
        }
    }

    Ok(manifest)
}

fn is_reserved_entry(name: &str) -> bool {
    matches!(
        name,
        LEGACY_COLLECTION | SQLITE_21_COLLECTION | SQLITE_21B_COLLECTION | MEDIA_MAP
    )
}

fn apkg_error(message: impl Into<String>) -> ApkgError {
    ApkgError {
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn package(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut writer = ZipWriter::new();
        for (name, data) in entries {
            writer.add_file(name, data, false);
        }
        writer.finish()
    }

    fn v11_sqlite_collection_bytes() -> Vec<u8> {
        let sqlite = tempfile::NamedTempFile::new().unwrap();
        {
            let connection = Connection::open(sqlite.path()).unwrap();
            connection
                .execute_batch(
                    r#"
CREATE TABLE col (
  id integer primary key,
  crt integer not null,
  mod integer not null,
  scm integer not null,
  ver integer not null,
  dty integer not null,
  usn integer not null,
  ls integer not null,
  conf text not null,
  models text not null,
  decks text not null,
  dconf text not null,
  tags text not null
);
CREATE TABLE notes (
  id integer primary key,
  guid text not null,
  mid integer not null,
  mod integer not null,
  usn integer not null,
  tags text not null,
  flds text not null,
  sfld text not null,
  csum integer not null,
  flags integer not null,
  data text not null
);
CREATE TABLE cards (
  id integer primary key,
  nid integer not null,
  did integer not null,
  ord integer not null,
  mod integer not null,
  usn integer not null,
  type integer not null,
  queue integer not null,
  due integer not null,
  ivl integer not null,
  factor integer not null,
  reps integer not null,
  lapses integer not null,
  left integer not null,
  odue integer not null,
  odid integer not null,
  flags integer not null,
  data text not null
);
CREATE TABLE revlog (
  id integer primary key,
  cid integer not null,
  usn integer not null,
  ease integer not null,
  ivl integer not null,
  lastIvl integer not null,
  factor integer not null,
  time integer not null,
  type integer not null
);
CREATE TABLE graves (
  usn integer not null,
  oid integer not null,
  type integer not null
);
"#,
                )
                .unwrap();

            let decks = r#"{
  "1": {"id": 1, "name": "Default", "desc": "Root deck"},
  "2": {"id": 2, "name": "Spanish::Latin", "desc": "Story deck"}
}"#;
            let models = r#"{
  "100": {
    "id": 100,
    "name": "Basic",
    "type": 0,
    "css": ".card { color: black; }",
    "flds": [
      {"name": "Front", "ord": 0},
      {"name": "Back", "ord": 1}
    ],
    "tmpls": [
      {"name": "Card 1", "ord": 0, "qfmt": "{{Front}}", "afmt": "{{Back}}", "did": 2}
    ]
  }
}"#;
            connection
                .execute(
                    "INSERT INTO col VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                    rusqlite::params![
                        1_i64,
                        19_000_i64,
                        1_700_000_000_i64,
                        1_700_000_001_i64,
                        11_i64,
                        0_i64,
                        -1_i64,
                        1_700_000_002_i64,
                        r#"{"nextPos": 1}"#,
                        models,
                        decks,
                        r#"{"1": {"name": "Default"}}"#,
                        r#"{"spanish": 1}"#
                    ],
                )
                .unwrap();
            connection
                .execute(
                    "INSERT INTO notes VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    rusqlite::params![
                        1000_i64,
                        "guid-1000",
                        100_i64,
                        1_700_000_010_i64,
                        -1_i64,
                        " spanish core ",
                        "hola\u{1f}hello",
                        "hola",
                        123_i64,
                        0_i64,
                        ""
                    ],
                )
                .unwrap();
            connection
                .execute(
                    "INSERT INTO cards VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
                    rusqlite::params![
                        2000_i64,
                        1000_i64,
                        2_i64,
                        0_i64,
                        1_700_000_020_i64,
                        -1_i64,
                        2_i64,
                        2_i64,
                        42_i64,
                        7_i64,
                        2500_i64,
                        3_i64,
                        1_i64,
                        0_i64,
                        0_i64,
                        0_i64,
                        0_i64,
                        ""
                    ],
                )
                .unwrap();
            connection
                .execute(
                    "INSERT INTO revlog VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    rusqlite::params![
                        3000_i64, 2000_i64, -1_i64, 3_i64, 7_i64, 3_i64, 2500_i64, 12_000_i64,
                        1_i64
                    ],
                )
                .unwrap();
            connection
                .execute(
                    "INSERT INTO graves VALUES (?1, ?2, ?3)",
                    rusqlite::params![-1_i64, 999_i64, 1_i64],
                )
                .unwrap();
        }
        std::fs::read(sqlite.path()).unwrap()
    }

    #[test]
    fn inspects_legacy_apkg_collection_and_media_map() {
        let media = br#"{"0":"audio/hola.mp3","1":"images/card.png","3":"missing.wav"}"#;
        let apkg = package(&[
            (LEGACY_COLLECTION, b"sqlite bytes"),
            (MEDIA_MAP, media),
            ("0", b"mp3"),
            ("1", b"png"),
            ("2", b"extra"),
        ]);

        let manifest = inspect_apkg(&apkg).unwrap();

        assert_eq!(manifest.collection.name, LEGACY_COLLECTION);
        assert_eq!(manifest.collection.format, CollectionFormat::LegacySqlite);
        assert_eq!(manifest.media.map_present, true);
        assert_eq!(
            manifest.media.mapping.get("0").map(String::as_str),
            Some("audio/hola.mp3")
        );
        assert_eq!(manifest.media.media_files.len(), 3);
        assert_eq!(manifest.media.media_files[0].archive_name, "0");
        assert_eq!(
            manifest.media.media_files[0].filename.as_deref(),
            Some("audio/hola.mp3")
        );
        assert_eq!(manifest.media.missing_files, vec!["3"]);
        assert_eq!(manifest.media.unmapped_files, vec!["2"]);
    }

    #[test]
    fn recognizes_modern_collection_members() {
        let apkg = package(&[(SQLITE_21B_COLLECTION, b"modern collection")]);

        let manifest = inspect_apkg(&apkg).unwrap();
        let collection = read_collection_bytes(&apkg).unwrap();

        assert_eq!(manifest.collection.name, SQLITE_21B_COLLECTION);
        assert_eq!(manifest.collection.format, CollectionFormat::Sqlite21b);
        assert_eq!(collection, b"modern collection");
    }

    #[test]
    fn reads_v11_collection_members_and_rejects_modern_packages() {
        let legacy = package(&[(LEGACY_COLLECTION, b"legacy collection")]);
        let sqlite21 = package(&[(SQLITE_21_COLLECTION, b"v11 collection")]);
        let modern = package(&[(SQLITE_21B_COLLECTION, b"modern collection")]);

        assert_eq!(
            read_v11_collection_bytes(&legacy).unwrap(),
            b"legacy collection"
        );
        assert_eq!(
            read_v11_collection_bytes(&sqlite21).unwrap(),
            b"v11 collection"
        );

        let error = read_v11_collection_bytes(&modern).unwrap_err();
        assert!(error.message.contains("modern package format"));
        assert!(error
            .message
            .contains("collection.anki2 and collection.anki21"));
    }

    #[test]
    fn parses_v11_sqlite_collection_tables() {
        let collection = parse_v11_collection_bytes(&v11_sqlite_collection_bytes()).unwrap();

        assert_eq!(collection.metadata.version, 11);
        assert_eq!(collection.metadata.config["nextPos"], 1);
        assert_eq!(collection.decks.len(), 2);
        assert_eq!(collection.decks[1].name, "Spanish::Latin");
        assert_eq!(collection.decks[1].description, "Story deck");

        assert_eq!(collection.note_types.len(), 1);
        let note_type = &collection.note_types[0];
        assert_eq!(note_type.id, 100);
        assert_eq!(note_type.name, "Basic");
        assert_eq!(note_type.fields[0].name, "Front");
        assert_eq!(note_type.fields[1].name, "Back");
        assert_eq!(note_type.templates[0].question_format, "{{Front}}");
        assert_eq!(note_type.templates[0].deck_id, Some(2));

        assert_eq!(collection.notes.len(), 1);
        let note = &collection.notes[0];
        assert_eq!(note.id, 1000);
        assert_eq!(note.tags, vec!["spanish", "core"]);
        assert_eq!(note.field_values, vec!["hola", "hello"]);

        assert_eq!(collection.cards.len(), 1);
        let card = &collection.cards[0];
        assert_eq!(card.id, 2000);
        assert_eq!(card.note_id, 1000);
        assert_eq!(card.deck_id, 2);
        assert_eq!(card.kind, 2);
        assert_eq!(card.queue, 2);
        assert_eq!(card.interval, 7);
        assert_eq!(card.factor, 2500);

        assert_eq!(collection.reviews.len(), 1);
        assert_eq!(collection.reviews[0].card_id, 2000);
        assert_eq!(collection.reviews[0].ease, 3);
        assert_eq!(collection.reviews[0].last_interval, 3);

        assert_eq!(collection.graves.len(), 1);
        assert_eq!(collection.graves[0].object_id, 999);
    }

    #[test]
    fn reads_v11_sqlite_collection_from_apkg_envelope() {
        let sqlite = v11_sqlite_collection_bytes();
        let apkg = write_legacy_apkg(&sqlite, &[]);

        let collection = read_v11_collection(&apkg).unwrap();

        assert_eq!(collection.note_types[0].name, "Basic");
        assert_eq!(collection.notes[0].field_values, vec!["hola", "hello"]);
    }

    #[test]
    fn v11_collection_reader_rejects_modern_apkg_envelope() {
        let sqlite = v11_sqlite_collection_bytes();
        let modern = package(&[(SQLITE_21B_COLLECTION, sqlite.as_slice())]);

        let error = read_v11_collection(&modern).unwrap_err();

        assert!(error.message.contains("modern package format"));
    }

    #[test]
    fn reads_resolved_media_payloads() {
        let apkg = write_legacy_apkg(
            b"sqlite collection",
            &[
                MediaAsset {
                    filename: "audio/hola.mp3",
                    data: b"mp3",
                },
                MediaAsset {
                    filename: "images/card.png",
                    data: b"png",
                },
            ],
        );

        let media_files = read_media_files(&apkg).unwrap();
        assert_eq!(
            media_files,
            vec![
                ResolvedMediaFile {
                    archive_name: "0".to_string(),
                    filename: Some("audio/hola.mp3".to_string()),
                    data: b"mp3".to_vec(),
                },
                ResolvedMediaFile {
                    archive_name: "1".to_string(),
                    filename: Some("images/card.png".to_string()),
                    data: b"png".to_vec(),
                },
            ]
        );

        let audio = read_media_file(&apkg, "0").unwrap();
        assert_eq!(audio.filename.as_deref(), Some("audio/hola.mp3"));
        assert_eq!(audio.data, b"mp3");
    }

    #[test]
    fn rejects_unknown_media_payloads() {
        let apkg = write_legacy_apkg(b"sqlite collection", &[]);
        let error = read_media_file(&apkg, "0").unwrap_err();

        assert!(error.message.contains("media file '0' not found"));
    }

    #[test]
    fn writes_legacy_apkg_package_envelope() {
        let apkg = write_legacy_apkg(
            b"sqlite collection",
            &[
                MediaAsset {
                    filename: "audio/hola.mp3",
                    data: b"mp3",
                },
                MediaAsset {
                    filename: "images/card.png",
                    data: b"png",
                },
            ],
        );

        let manifest = inspect_apkg(&apkg).unwrap();
        let collection = read_collection_bytes(&apkg).unwrap();

        assert_eq!(manifest.collection.name, LEGACY_COLLECTION);
        assert_eq!(collection, b"sqlite collection");
        assert_eq!(manifest.media.map_present, true);
        assert_eq!(manifest.media.mapping["0"], "audio/hola.mp3");
        assert_eq!(manifest.media.mapping["1"], "images/card.png");
        assert_eq!(manifest.media.media_files.len(), 2);
        assert!(manifest.media.missing_files.is_empty());
        assert!(manifest.media.unmapped_files.is_empty());
    }

    #[test]
    fn reports_missing_collection_and_invalid_media_json() {
        let missing = package(&[(MEDIA_MAP, br#"{}"#)]);
        let error = inspect_apkg(&missing).unwrap_err();
        assert!(error.message.contains("missing collection"));

        let invalid_media = package(&[(LEGACY_COLLECTION, b"sqlite"), (MEDIA_MAP, b"not json")]);
        let error = inspect_apkg(&invalid_media).unwrap_err();
        assert!(error.message.contains("invalid Anki media map JSON"));
    }
}
