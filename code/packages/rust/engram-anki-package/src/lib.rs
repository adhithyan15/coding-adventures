//! APKG archive inspection for Engram.
//!
//! This crate owns the APKG archive boundary plus the supported SQLite
//! collection import/export path. It identifies Anki collection members,
//! honors modern package metadata, decodes zstd-compressed modern payloads, and
//! resolves legacy JSON or modern protobuf media maps.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;
use std::io::Cursor;

use coding_adventures_sha1::sum1;
pub use engram_core::EngramMediaReferenceAnalysis;

use engram_core::{
    analyze_media_references, render_cloze_template, render_cloze_template_with_front_side,
    render_template, render_template_with_front_side, template_references_cloze, AppState, Card,
    CardFlag, CardLineage, CardProgress, CardState, CardTemplate, ClozeRenderSide, Deck,
    DeckOptions, DeckOptionsPreset, ExternalSourceRecord, ExternalSourceTarget, FieldDef,
    LeechAction, MediaAssetRecord, Note, NoteFieldValue, NoteType, Rating, Review, Session,
    SessionStatus, TemplateRequirementMode, INITIAL_EASE_FACTOR, ONE_DAY_MS,
};
use rusqlite::{Connection, DatabaseName};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlite_file::SqlValue;
use zip::{ZipReader, ZipWriter};

const LEGACY_COLLECTION: &str = "collection.anki2";
const SQLITE_21_COLLECTION: &str = "collection.anki21";
const SQLITE_21B_COLLECTION: &str = "collection.anki21b";
const MEDIA_MAP: &str = "media";
const META: &str = "meta";
const ANKI_V11_SOURCE: &str = "anki-v11";
const ANKI_MARKED_TAG: &str = "marked";

#[derive(Clone, PartialEq)]
struct PackageMetadataProto {
    version: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
enum PackageVersionProto {
    Unknown = 0,
    Legacy1 = 1,
    Legacy2 = 2,
    Latest = 3,
}

#[derive(Clone, PartialEq)]
struct MediaEntriesProto {
    entries: Vec<MediaEntryProto>,
}

#[derive(Clone, PartialEq)]
struct MediaEntryProto {
    name: String,
    size: u32,
    sha1: Vec<u8>,
    legacy_zip_filename: Option<u32>,
}

// --- Zero-dep protobuf codecs for the Anki meta/media messages --------------
//
// Hand-coded encode/decode against the repo `protobuf` wire crate. These
// replaced the third-party `prost` derive (removed in this crate) after a
// cross-compat gate proved they produce byte-for-byte identical output to
// `prost` and round-trip its bytes, guaranteeing real-Anki `.anki21b` interop.
// They follow proto3 semantics: implicit-presence scalar fields are OMITTED
// when equal to their default
// (empty string / 0 / empty bytes), and the explicit-`optional`
// `legacy_zip_filename` is emitted only when `Some` (even if 0). Decoders start
// from defaults and overwrite per field, ignoring unknown field numbers.

impl PackageVersionProto {
    fn parse_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(PackageVersionProto::Unknown),
            1 => Some(PackageVersionProto::Legacy1),
            2 => Some(PackageVersionProto::Legacy2),
            3 => Some(PackageVersionProto::Latest),
            _ => None,
        }
    }
}

impl PackageMetadataProto {
    fn encode_pb(&self) -> Vec<u8> {
        let mut w = protobuf::Writer::new();
        if self.version != 0 {
            w.varint(1, self.version as u64);
        }
        w.into_bytes()
    }

    fn decode_pb(bytes: &[u8]) -> Result<Self, protobuf::Error> {
        let mut version = 0i32;
        let mut r = protobuf::Reader::new(bytes);
        while let Some(field) = r.next_field()? {
            if field.number == 1 {
                if let Some(v) = field.value.as_varint() {
                    version = v as i32;
                }
            }
        }
        Ok(PackageMetadataProto { version })
    }
}

impl MediaEntryProto {
    fn encode_pb(&self) -> Vec<u8> {
        let mut w = protobuf::Writer::new();
        if !self.name.is_empty() {
            w.string(1, &self.name);
        }
        if self.size != 0 {
            w.varint(2, self.size as u64);
        }
        if !self.sha1.is_empty() {
            w.bytes(3, &self.sha1);
        }
        if let Some(legacy) = self.legacy_zip_filename {
            w.varint(255, legacy as u64);
        }
        w.into_bytes()
    }

    fn decode_pb(bytes: &[u8]) -> Result<Self, protobuf::Error> {
        let mut entry = MediaEntryProto {
            name: String::new(),
            size: 0,
            sha1: Vec::new(),
            legacy_zip_filename: None,
        };
        let mut r = protobuf::Reader::new(bytes);
        while let Some(field) = r.next_field()? {
            match field.number {
                1 => {
                    if let Some(b) = field.value.as_bytes() {
                        entry.name = String::from_utf8_lossy(b).into_owned();
                    }
                }
                2 => {
                    if let Some(v) = field.value.as_varint() {
                        entry.size = v as u32;
                    }
                }
                3 => {
                    if let Some(b) = field.value.as_bytes() {
                        entry.sha1 = b.to_vec();
                    }
                }
                255 => {
                    if let Some(v) = field.value.as_varint() {
                        entry.legacy_zip_filename = Some(v as u32);
                    }
                }
                _ => {}
            }
        }
        Ok(entry)
    }
}

impl MediaEntriesProto {
    fn encode_pb(&self) -> Vec<u8> {
        let mut w = protobuf::Writer::new();
        for entry in &self.entries {
            w.message(1, &entry.encode_pb());
        }
        w.into_bytes()
    }

    fn decode_pb(bytes: &[u8]) -> Result<Self, protobuf::Error> {
        let mut entries = Vec::new();
        let mut r = protobuf::Reader::new(bytes);
        while let Some(field) = r.next_field()? {
            if field.number == 1 {
                if let Some(b) = field.value.as_bytes() {
                    entries.push(MediaEntryProto::decode_pb(b)?);
                }
            }
        }
        Ok(MediaEntriesProto { entries })
    }
}

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

    fn is_modern(self) -> bool {
        matches!(self, Self::Sqlite21b)
    }

    fn collection_name(self) -> &'static str {
        match self {
            Self::LegacySqlite => LEGACY_COLLECTION,
            Self::Sqlite21 => SQLITE_21_COLLECTION,
            Self::Sqlite21b => SQLITE_21B_COLLECTION,
        }
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legacy_zip_filename: Option<u32>,
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
    let collection = collection_member(&reader, &entries)?;
    let media = media_manifest(&reader, collection.format)?;

    Ok(AnkiPackageManifest {
        collection,
        media,
        entries,
    })
}

pub fn read_collection_bytes(data: &[u8]) -> Result<Vec<u8>, ApkgError> {
    let reader = ZipReader::new(data).map_err(apkg_error)?;
    let entries = archive_entries(&reader);
    let collection = collection_member(&reader, &entries)?;
    let bytes = reader
        .read_by_name(&collection.name)
        .map_err(|err| apkg_error(format!("failed to read collection: {err}")))?;
    decode_package_payload(collection.format, "collection", &bytes)
}

pub fn read_v11_collection_bytes(data: &[u8]) -> Result<Vec<u8>, ApkgError> {
    let reader = ZipReader::new(data).map_err(apkg_error)?;
    let entries = archive_entries(&reader);
    let collection = collection_member(&reader, &entries)?;
    let bytes = reader
        .read_by_name(&collection.name)
        .map_err(|err| apkg_error(format!("failed to read collection: {err}")))?;
    decode_package_payload(collection.format, "collection", &bytes)
}

pub fn read_media_file(data: &[u8], archive_name: &str) -> Result<ResolvedMediaFile, ApkgError> {
    let reader = ZipReader::new(data).map_err(apkg_error)?;
    let entries = archive_entries(&reader);
    let collection = collection_member(&reader, &entries)?;
    let manifest = media_manifest(&reader, collection.format)?;
    let media = manifest
        .media_files
        .into_iter()
        .find(|media| media_matches_archive_name(media, archive_name))
        .ok_or_else(|| apkg_error(format!("media file '{archive_name}' not found")))?;
    let payload_archive_name = media_payload_archive_name(&media);
    let data = reader.read_by_name(&payload_archive_name).map_err(|err| {
        apkg_error(format!(
            "failed to read media file '{}' from archive member '{}': {err}",
            media.archive_name, payload_archive_name
        ))
    })?;
    let data = decode_package_payload(collection.format, "media file", &data)?;

    Ok(ResolvedMediaFile {
        archive_name: media.archive_name,
        filename: media.filename,
        data,
    })
}

pub fn read_media_files(data: &[u8]) -> Result<Vec<ResolvedMediaFile>, ApkgError> {
    let reader = ZipReader::new(data).map_err(apkg_error)?;
    let entries = archive_entries(&reader);
    let collection = collection_member(&reader, &entries)?;
    let manifest = media_manifest(&reader, collection.format)?;

    manifest
        .media_files
        .into_iter()
        .map(|media| {
            let payload_archive_name = media_payload_archive_name(&media);
            let data = reader.read_by_name(&payload_archive_name).map_err(|err| {
                apkg_error(format!(
                    "failed to read media file '{}' from archive member '{}': {err}",
                    media.archive_name, payload_archive_name
                ))
            })?;
            let data = decode_package_payload(collection.format, "media file", &data)?;
            Ok(ResolvedMediaFile {
                archive_name: media.archive_name,
                filename: media.filename,
                data,
            })
        })
        .collect()
}

fn media_matches_archive_name(media: &MediaFile, archive_name: &str) -> bool {
    media.archive_name == archive_name
        || media
            .legacy_zip_filename
            .is_some_and(|legacy| archive_name == legacy.to_string())
}

fn media_payload_archive_name(media: &MediaFile) -> String {
    media
        .legacy_zip_filename
        .map(|legacy| legacy.to_string())
        .unwrap_or_else(|| media.archive_name.clone())
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

pub fn write_modern_apkg(
    collection_anki21b: &[u8],
    media_assets: &[MediaAsset<'_>],
) -> Result<Vec<u8>, ApkgError> {
    let mut writer = ZipWriter::new();

    let metadata = PackageMetadataProto {
        version: PackageVersionProto::Latest as i32,
    }
    .encode_pb();
    writer.add_file(META, &metadata, false);

    let collection = encode_package_payload("collection", collection_anki21b)?;
    writer.add_file(SQLITE_21B_COLLECTION, &collection, false);

    let media_entries = MediaEntriesProto {
        entries: media_assets
            .iter()
            .map(|asset| MediaEntryProto {
                name: asset.filename.to_string(),
                size: asset.data.len() as u32,
                sha1: sum1(asset.data).to_vec(),
                legacy_zip_filename: None,
            })
            .collect(),
    };
    let media_map = media_entries.encode_pb();
    let media_map = encode_package_payload("media map", &media_map)?;
    writer.add_file(MEDIA_MAP, &media_map, false);

    for (index, asset) in media_assets.iter().enumerate() {
        let media = encode_package_payload(&format!("media file {index}"), asset.data)?;
        writer.add_file(&index.to_string(), &media, false);
    }

    Ok(writer.finish())
}

pub fn write_v11_collection_bytes_from_engram_state(
    state: &AppState,
) -> Result<Vec<u8>, ApkgError> {
    let export = ExportModel::from_state(state)?;
    let connection = Connection::open_in_memory().map_err(|err| {
        apkg_error(format!(
            "failed to open in-memory Anki V11 SQLite collection: {err}"
        ))
    })?;
    create_v11_export_schema(&connection)?;
    write_v11_export_rows(&connection, &export)?;
    connection
        .serialize(DatabaseName::Main)
        .map(|data| data.to_vec())
        .map_err(|err| apkg_error(format!("failed to serialize Anki V11 collection: {err}")))
}

pub fn write_legacy_apkg_from_engram_state(
    state: &AppState,
    media_assets: &[MediaAsset<'_>],
) -> Result<Vec<u8>, ApkgError> {
    let collection = write_v11_collection_bytes_from_engram_state(state)?;
    let mut state_media = state
        .media_assets
        .iter()
        .map(|asset| MediaAsset {
            filename: asset.filename.as_deref().unwrap_or(&asset.archive_name),
            data: asset.data.as_slice(),
        })
        .collect::<Vec<_>>();
    state_media.extend_from_slice(media_assets);
    Ok(write_legacy_apkg(&collection, &state_media))
}

pub fn write_modern_apkg_from_engram_state(
    state: &AppState,
    media_assets: &[MediaAsset<'_>],
) -> Result<Vec<u8>, ApkgError> {
    let collection = write_v11_collection_bytes_from_engram_state(state)?;
    let mut state_media = state
        .media_assets
        .iter()
        .map(|asset| MediaAsset {
            filename: asset.filename.as_deref().unwrap_or(&asset.archive_name),
            data: asset.data.as_slice(),
        })
        .collect::<Vec<_>>();
    state_media.extend_from_slice(media_assets);
    write_modern_apkg(&collection, &state_media)
}

pub fn read_v11_collection(data: &[u8]) -> Result<AnkiV11Collection, ApkgError> {
    let collection = read_v11_collection_bytes(data)?;
    parse_v11_collection_bytes(&collection)
}

pub fn parse_v11_collection_bytes(bytes: &[u8]) -> Result<AnkiV11Collection, ApkgError> {
    let raw_col = read_v11_col_row(bytes)?;
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
        notes: read_v11_notes(bytes)?,
        cards: read_v11_cards(bytes)?,
        reviews: read_v11_reviews(bytes)?,
        graves: read_v11_graves(bytes)?,
        metadata,
    })
}

pub fn read_v11_collection_as_engram_state(data: &[u8]) -> Result<AppState, ApkgError> {
    let collection = read_v11_collection(data)?;
    let mut state = v11_collection_to_engram_state(&collection)?;
    let media_assets: Vec<MediaAssetRecord> = read_media_files(data)?
        .into_iter()
        .map(media_asset_record_from_resolved)
        .collect();
    state
        .external_sources
        .extend(v11_media_external_sources(&media_assets));
    state.media_assets = media_assets;
    Ok(state)
}

pub fn v11_collection_to_engram_state(
    collection: &AnkiV11Collection,
) -> Result<AppState, ApkgError> {
    let default_deck_id = collection
        .decks
        .first()
        .map(|deck| deck.id.to_string())
        .unwrap_or_else(|| "anki-default".to_string());
    let decks = if collection.decks.is_empty() {
        vec![Deck {
            id: default_deck_id.clone(),
            name: "Imported Anki Deck".to_string(),
            description: String::new(),
            created_at: anki_days_to_millis(collection.metadata.created_at_days),
        }]
    } else {
        collection
            .decks
            .iter()
            .map(|deck| Deck {
                id: deck.id.to_string(),
                name: deck.name.clone(),
                description: deck.description.clone(),
                created_at: anki_days_to_millis(collection.metadata.created_at_days),
            })
            .collect()
    };

    let note_types = collection
        .note_types
        .iter()
        .map(map_v11_note_type)
        .collect::<Vec<_>>();
    let anki_note_types_by_id: HashMap<i64, &AnkiV11NoteType> = collection
        .note_types
        .iter()
        .map(|note_type| (note_type.id, note_type))
        .collect();
    let note_types_by_id: HashMap<String, NoteType> = note_types
        .iter()
        .cloned()
        .map(|note_type| (note_type.id.clone(), note_type))
        .collect();

    let mut deck_by_note_id = BTreeMap::new();
    for card in &collection.cards {
        deck_by_note_id
            .entry(card.note_id)
            .or_insert_with(|| card.deck_id.to_string());
    }

    let mut notes = Vec::with_capacity(collection.notes.len());
    for note in &collection.notes {
        let anki_note_type = anki_note_types_by_id
            .get(&note.note_type_id)
            .ok_or_else(|| {
                apkg_error(format!(
                    "Anki note {} references missing note type {}",
                    note.id, note.note_type_id
                ))
            })?;
        notes.push(map_v11_note(
            note,
            anki_note_type,
            deck_by_note_id
                .get(&note.id)
                .map(String::as_str)
                .unwrap_or(&default_deck_id),
        ));
    }
    let notes_by_id: HashMap<String, Note> = notes
        .iter()
        .cloned()
        .map(|note| (note.id.clone(), note))
        .collect();
    let deck_names_by_id: HashMap<i64, String> = collection
        .decks
        .iter()
        .map(|deck| (deck.id, deck.name.clone()))
        .collect();

    let mut cards = Vec::with_capacity(collection.cards.len());
    for card in &collection.cards {
        cards.push(map_v11_card(
            card,
            &notes_by_id,
            &note_types_by_id,
            &anki_note_types_by_id,
            &deck_names_by_id,
        )?);
    }

    let marked_at_by_note_id = collection
        .notes
        .iter()
        .filter_map(|note| anki_marked_at_for_note(note).map(|marked_at| (note.id, marked_at)))
        .collect::<BTreeMap<_, _>>();
    let last_reviewed_at_by_card = last_reviewed_at_by_card(&collection.reviews);
    let deck_options = v11_deck_options(collection);
    let deck_options_by_deck_id: HashMap<i64, &DeckOptions> = deck_options
        .iter()
        .filter_map(|preset| {
            preset
                .deck_id
                .parse::<i64>()
                .ok()
                .map(|deck_id| (deck_id, &preset.options))
        })
        .collect();
    let card_progress = collection
        .cards
        .iter()
        .filter_map(|card| {
            map_v11_card_progress(
                card,
                collection.metadata.created_at_days,
                &marked_at_by_note_id,
                &last_reviewed_at_by_card,
                deck_options_by_deck_id.get(&card.deck_id).copied(),
            )
        })
        .collect::<Vec<_>>();

    let deck_by_card_id: HashMap<i64, String> = collection
        .cards
        .iter()
        .map(|card| (card.id, card.deck_id.to_string()))
        .collect();
    let reviews = collection
        .reviews
        .iter()
        .map(|review| {
            map_v11_review(
                review,
                deck_by_card_id
                    .get(&review.card_id)
                    .map(String::as_str)
                    .unwrap_or(&default_deck_id),
            )
        })
        .collect::<Vec<_>>();
    let sessions = synthetic_import_sessions(&reviews, &deck_by_card_id, &default_deck_id);
    let external_sources = v11_external_sources(collection)?;

    Ok(AppState {
        decks,
        note_types,
        notes,
        cards,
        card_progress,
        sessions,
        reviews,
        deck_options,
        external_sources,
        media_assets: Vec::new(),
        active_session: None,
    })
}

fn media_asset_record_from_resolved(media: ResolvedMediaFile) -> MediaAssetRecord {
    MediaAssetRecord {
        id: format!("anki-media:{}", media.archive_name),
        archive_name: media.archive_name,
        filename: media.filename,
        data: media.data,
    }
}

fn v11_media_external_sources(media_assets: &[MediaAssetRecord]) -> Vec<ExternalSourceRecord> {
    media_assets
        .iter()
        .map(|asset| {
            let mut data = BTreeMap::new();
            insert_string(&mut data, "archiveName", &asset.archive_name);
            if let Some(filename) = asset.filename.as_deref() {
                insert_string(&mut data, "filename", filename);
            }
            source_record(
                ExternalSourceTarget::Media,
                asset.id.clone(),
                Some(asset.archive_name.clone()),
                data,
            )
        })
        .collect()
}

pub fn analyze_engram_media_references(state: &AppState) -> EngramMediaReferenceAnalysis {
    analyze_media_references(state)
}

fn v11_deck_options(collection: &AnkiV11Collection) -> Vec<DeckOptionsPreset> {
    collection
        .decks
        .iter()
        .map(|deck| DeckOptionsPreset {
            deck_id: deck.id.to_string(),
            options: v11_options_for_deck(deck, &collection.metadata.deck_config),
        })
        .collect()
}

fn v11_options_for_deck(deck: &AnkiV11Deck, deck_config: &Value) -> DeckOptions {
    let config_id = json_i64(&deck.raw, "conf").unwrap_or(1);
    let config = deck_config
        .get(config_id.to_string())
        .or_else(|| deck_config.get("1"));
    let mut options = DeckOptions::default();

    if let Some(config) = config {
        options.new_cards_per_day = json_path_u32(config, &["new", "perDay"])
            .or_else(|| json_path_u32(config, &["new", "perday"]))
            .unwrap_or(options.new_cards_per_day);
        options.reviews_per_day = json_path_u32(config, &["rev", "perDay"])
            .or_else(|| json_path_u32(config, &["rev", "perday"]))
            .unwrap_or(options.reviews_per_day);
        options.bury_new_siblings =
            json_path_bool(config, &["new", "bury"]).unwrap_or(options.bury_new_siblings);
        options.bury_review_siblings =
            json_path_bool(config, &["rev", "bury"]).unwrap_or(options.bury_review_siblings);
        options.bury_interday_learning_siblings = json_path_bool(config, &["buryInterdayLearning"])
            .or_else(|| json_path_bool(config, &["new", "buryInterdayLearning"]))
            .unwrap_or(options.bury_interday_learning_siblings);
        options.learning_steps_minutes =
            json_path_minutes(config, &["new", "delays"]).unwrap_or(options.learning_steps_minutes);
        options.relearning_steps_minutes = json_path_minutes(config, &["lapse", "delays"])
            .unwrap_or(options.relearning_steps_minutes);
        if let Some(intervals) = json_path_u32_array(config, &["new", "ints"]) {
            if let Some(graduating) = intervals.first() {
                options.graduating_interval_days = (*graduating).max(1);
            }
            if let Some(easy) = intervals.get(1) {
                options.easy_interval_days = (*easy).max(options.graduating_interval_days);
            }
        }
        if let Some(initial_factor) = json_path_f64(config, &["new", "initialFactor"]) {
            options.initial_ease_factor =
                normalized_anki_ease_factor(initial_factor, options.initial_ease_factor);
        }
        if let Some(multiplier) = json_path_f64(config, &["lapse", "mult"]) {
            options.lapse_interval_multiplier =
                normalized_anki_multiplier(multiplier, options.lapse_interval_multiplier);
        }
        options.leech_threshold =
            json_path_u32(config, &["lapse", "leechFails"]).unwrap_or(options.leech_threshold);
        if let Some(action) = json_path_i64(config, &["lapse", "leechAction"]) {
            options.leech_action = match action {
                0 => LeechAction::Suspend,
                1 => LeechAction::TagOnly,
                _ => options.leech_action,
            };
        }
        if let Some(max_interval) = json_path_u32(config, &["rev", "maxIvl"]) {
            options.maximum_interval_days = max_interval.max(1);
        }
        if let Some(modifier) = json_path_f64(config, &["rev", "ivlFct"]) {
            options.review_interval_modifier =
                normalized_anki_multiplier(modifier, options.review_interval_modifier);
        }
        if let Some(multiplier) = json_path_f64(config, &["rev", "hardFactor"]) {
            options.hard_interval_multiplier =
                normalized_anki_multiplier(multiplier, options.hard_interval_multiplier);
        }
        if let Some(multiplier) = json_path_f64(config, &["rev", "ease4"]) {
            options.easy_bonus_multiplier =
                normalized_anki_multiplier(multiplier, options.easy_bonus_multiplier);
        }
        if let Some(desired_retention) = json_path_f64(config, &["desiredRetention"]) {
            options.desired_retention =
                normalized_retention(desired_retention, options.desired_retention);
        }
        options.fsrs_parameters = json_path_f64_array(config, &["fsrsParams6"])
            .filter(|parameters| !parameters.is_empty())
            .or_else(|| {
                json_path_f64_array(config, &["fsrsParams5"])
                    .filter(|parameters| !parameters.is_empty())
            })
            .or_else(|| {
                json_path_f64_array(config, &["fsrsWeights"])
                    .filter(|parameters| !parameters.is_empty())
            })
            .unwrap_or(options.fsrs_parameters);
        options.fsrs_parameter_search =
            json_path_string(config, &["weightSearch"]).unwrap_or(options.fsrs_parameter_search);
        options.ignore_review_history_before =
            json_path_string(config, &["ignoreRevlogsBeforeDate"])
                .unwrap_or(options.ignore_review_history_before);
        if let Some(historical_retention) = json_path_f64(config, &["sm2Retention"]) {
            options.historical_retention =
                normalized_retention(historical_retention, options.historical_retention);
        }
        options.easy_days_percentages = json_path_f64_array(config, &["easyDaysPercentages"])
            .filter(|values| !values.is_empty())
            .unwrap_or(options.easy_days_percentages);
    }

    options
}

fn normalized_retention(value: f64, fallback: f64) -> f64 {
    if value.is_finite() && value > 0.0 && value <= 1.0 {
        value
    } else {
        fallback
    }
}

fn normalized_anki_multiplier(value: f64, fallback: f64) -> f64 {
    if !value.is_finite() || value <= 0.0 {
        return fallback;
    }
    if value > 10.0 {
        value / 100.0
    } else {
        value
    }
}

fn normalized_anki_ease_factor(value: f64, fallback: f64) -> f64 {
    if !value.is_finite() || value <= 0.0 {
        return fallback;
    }
    if value > 100.0 {
        value / 1000.0
    } else {
        value
    }
}

fn v11_external_sources(
    collection: &AnkiV11Collection,
) -> Result<Vec<ExternalSourceRecord>, ApkgError> {
    let mut sources = Vec::new();

    let mut collection_data = BTreeMap::new();
    insert_i64(&mut collection_data, "id", collection.metadata.id);
    insert_i64(
        &mut collection_data,
        "createdAtDays",
        collection.metadata.created_at_days,
    );
    insert_i64(
        &mut collection_data,
        "modifiedAt",
        collection.metadata.modified_at,
    );
    insert_i64(
        &mut collection_data,
        "schemaModifiedAt",
        collection.metadata.schema_modified_at,
    );
    insert_i64(&mut collection_data, "version", collection.metadata.version);
    insert_i64(&mut collection_data, "dirty", collection.metadata.dirty);
    insert_i64(
        &mut collection_data,
        "updateSequenceNumber",
        collection.metadata.update_sequence_number,
    );
    insert_i64(
        &mut collection_data,
        "lastSync",
        collection.metadata.last_sync,
    );
    insert_json(
        &mut collection_data,
        "configJson",
        &collection.metadata.config,
        "col.conf",
    )?;
    insert_json(
        &mut collection_data,
        "deckConfigJson",
        &collection.metadata.deck_config,
        "col.dconf",
    )?;
    insert_json(
        &mut collection_data,
        "tagsJson",
        &collection.metadata.tags,
        "col.tags",
    )?;
    insert_serialized_json(
        &mut collection_data,
        "gravesJson",
        &collection.graves,
        "Anki graves",
    )?;
    sources.push(source_record(
        ExternalSourceTarget::Collection,
        "collection",
        Some(collection.metadata.id.to_string()),
        collection_data,
    ));

    for deck in &collection.decks {
        let mut data = BTreeMap::new();
        let config_id = json_i64(&deck.raw, "conf").unwrap_or(1);
        insert_i64(&mut data, "configId", config_id);
        if let Some(name) = collection
            .metadata
            .deck_config
            .get(config_id.to_string())
            .or_else(|| collection.metadata.deck_config.get("1"))
            .and_then(|config| config.get("name"))
            .and_then(Value::as_str)
        {
            insert_string(&mut data, "configName", name);
        }
        insert_json(&mut data, "rawJson", &deck.raw, "Anki deck JSON")?;
        insert_i64(
            &mut data,
            "dyn",
            deck.raw.get("dyn").and_then(Value::as_i64).unwrap_or(0),
        );
        if let Some(resched) = deck.raw.get("resched").and_then(Value::as_bool) {
            insert_string(&mut data, "resched", if resched { "true" } else { "false" });
        }
        sources.push(source_record(
            ExternalSourceTarget::Deck,
            deck.id.to_string(),
            Some(deck.id.to_string()),
            data,
        ));
    }

    for note_type in &collection.note_types {
        let mut data = BTreeMap::new();
        insert_json(&mut data, "rawJson", &note_type.raw, "Anki model JSON")?;
        sources.push(source_record(
            ExternalSourceTarget::NoteType,
            note_type.id.to_string(),
            Some(note_type.id.to_string()),
            data,
        ));
    }

    for note in &collection.notes {
        let mut data = BTreeMap::new();
        insert_string(&mut data, "guid", &note.guid);
        insert_i64(&mut data, "modifiedAt", note.modified_at);
        insert_i64(
            &mut data,
            "updateSequenceNumber",
            note.update_sequence_number,
        );
        insert_string(&mut data, "sortField", &note.sort_field);
        insert_i64(&mut data, "checksum", note.checksum);
        insert_i64(&mut data, "flags", note.flags);
        insert_string(&mut data, "data", &note.data);
        sources.push(source_record(
            ExternalSourceTarget::Note,
            note.id.to_string(),
            Some(note.id.to_string()),
            data,
        ));
    }

    for card in &collection.cards {
        let mut data = BTreeMap::new();
        insert_i64(&mut data, "noteId", card.note_id);
        insert_i64(&mut data, "deckId", card.deck_id);
        insert_i64(&mut data, "ordinal", card.ordinal);
        insert_i64(&mut data, "modifiedAt", card.modified_at);
        insert_i64(
            &mut data,
            "updateSequenceNumber",
            card.update_sequence_number,
        );
        insert_i64(&mut data, "kind", card.kind);
        insert_i64(&mut data, "queue", card.queue);
        insert_i64(&mut data, "due", card.due);
        insert_i64(&mut data, "interval", card.interval);
        insert_i64(&mut data, "factor", card.factor);
        insert_i64(&mut data, "repetitions", card.repetitions);
        insert_i64(&mut data, "lapses", card.lapses);
        insert_i64(&mut data, "left", card.left);
        insert_i64(&mut data, "originalDue", card.original_due);
        insert_i64(&mut data, "originalDeckId", card.original_deck_id);
        insert_i64(&mut data, "flags", card.flags);
        insert_string(&mut data, "data", &card.data);
        sources.push(source_record(
            ExternalSourceTarget::Card,
            card.id.to_string(),
            Some(card.id.to_string()),
            data,
        ));
    }

    for review in &collection.reviews {
        let mut data = BTreeMap::new();
        insert_i64(&mut data, "cardId", review.card_id);
        insert_i64(
            &mut data,
            "updateSequenceNumber",
            review.update_sequence_number,
        );
        insert_i64(&mut data, "ease", review.ease);
        insert_i64(&mut data, "interval", review.interval);
        insert_i64(&mut data, "lastInterval", review.last_interval);
        insert_i64(&mut data, "factor", review.factor);
        insert_i64(&mut data, "time", review.time);
        insert_i64(&mut data, "kind", review.kind);
        sources.push(source_record(
            ExternalSourceTarget::Review,
            review.id.to_string(),
            Some(review.id.to_string()),
            data,
        ));
    }

    Ok(sources)
}

fn source_record(
    target: ExternalSourceTarget,
    target_id: impl Into<String>,
    original_id: Option<String>,
    data: BTreeMap<String, String>,
) -> ExternalSourceRecord {
    ExternalSourceRecord {
        target,
        target_id: target_id.into(),
        source: ANKI_V11_SOURCE.to_string(),
        original_id,
        data,
    }
}

fn insert_i64(data: &mut BTreeMap<String, String>, key: &str, value: i64) {
    data.insert(key.to_string(), value.to_string());
}

fn insert_string(data: &mut BTreeMap<String, String>, key: &str, value: &str) {
    data.insert(key.to_string(), value.to_string());
}

fn insert_json(
    data: &mut BTreeMap<String, String>,
    key: &str,
    value: &Value,
    label: &str,
) -> Result<(), ApkgError> {
    insert_serialized_json(data, key, value, label)
}

fn insert_serialized_json<T: Serialize>(
    data: &mut BTreeMap<String, String>,
    key: &str,
    value: &T,
    label: &str,
) -> Result<(), ApkgError> {
    let json = serde_json::to_string(value)
        .map_err(|err| apkg_error(format!("failed to serialize {label}: {err}")))?;
    data.insert(key.to_string(), json);
    Ok(())
}

#[derive(Clone, Debug)]
struct ExportDeck {
    key: String,
    name: String,
    description: String,
    created_at: u64,
}

#[derive(Clone, Debug)]
struct ExportNoteType {
    key: String,
    name: String,
    kind: i64,
    fields: Vec<FieldDef>,
    templates: Vec<CardTemplate>,
    stylesheet: Option<String>,
    created_at: u64,
    updated_at: u64,
}

#[derive(Clone, Debug)]
struct ExportNote {
    key: String,
    note_type_key: String,
    fields: Vec<NoteFieldValue>,
    tags: Vec<String>,
    created_at: u64,
    updated_at: u64,
}

#[derive(Clone, Debug)]
struct ExportCard {
    key: String,
    note_key: String,
    deck_key: String,
    template_ordinal: u32,
    created_at: u64,
}

#[derive(Clone, Debug)]
struct ExportModel {
    created_at_days: i64,
    modified_at_seconds: i64,
    decks: Vec<ExportDeck>,
    note_types: Vec<ExportNoteType>,
    notes: Vec<ExportNote>,
    cards: Vec<ExportCard>,
    reviews: Vec<Review>,
    session_deck_by_id: HashMap<String, String>,
    progress_by_card: HashMap<String, CardProgress>,
    deck_options: Vec<DeckOptionsPreset>,
    deck_ids: BTreeMap<String, i64>,
    note_type_ids: BTreeMap<String, i64>,
    note_ids: BTreeMap<String, i64>,
    card_ids: BTreeMap<String, i64>,
    external_sources: Vec<ExternalSourceRecord>,
}

impl ExportModel {
    fn from_state(state: &AppState) -> Result<Self, ApkgError> {
        let default_deck_key = "1".to_string();
        let notes_by_id: HashMap<&str, &Note> = state
            .notes
            .iter()
            .map(|note| (note.id.as_str(), note))
            .collect();
        let note_types_by_id: HashMap<&str, &NoteType> = state
            .note_types
            .iter()
            .map(|note_type| (note_type.id.as_str(), note_type))
            .collect();
        let progress_by_card: HashMap<String, CardProgress> = state
            .card_progress
            .iter()
            .cloned()
            .map(|progress| (progress.card_id.clone(), progress))
            .collect();

        let mut decks = state
            .decks
            .iter()
            .map(|deck| ExportDeck {
                key: deck.id.clone(),
                name: deck.name.clone(),
                description: deck.description.clone(),
                created_at: deck.created_at,
            })
            .collect::<Vec<_>>();
        let mut known_decks = decks
            .iter()
            .map(|deck| deck.key.clone())
            .collect::<BTreeSet<_>>();
        for deck_key in state
            .cards
            .iter()
            .map(|card| fallback_deck_key(&card.deck_id, &default_deck_key))
            .chain(
                state
                    .notes
                    .iter()
                    .map(|note| fallback_deck_key(&note.deck_id, &default_deck_key)),
            )
        {
            if known_decks.insert(deck_key.clone()) {
                decks.push(ExportDeck {
                    key: deck_key.clone(),
                    name: deck_key,
                    description: String::new(),
                    created_at: 0,
                });
            }
        }
        if decks.is_empty() {
            decks.push(ExportDeck {
                key: default_deck_key.clone(),
                name: "Default".to_string(),
                description: String::new(),
                created_at: 0,
            });
        }

        let needs_synthetic_basic = state
            .cards
            .iter()
            .any(|card| !has_exportable_lineage(card, &notes_by_id, &note_types_by_id));
        let mut note_types = state
            .note_types
            .iter()
            .map(|note_type| ExportNoteType {
                key: note_type.id.clone(),
                name: note_type.name.clone(),
                kind: note_type_kind(note_type),
                fields: note_type.fields.clone(),
                templates: note_type.templates.clone(),
                stylesheet: note_type.stylesheet.clone(),
                created_at: note_type.created_at,
                updated_at: note_type.updated_at,
            })
            .collect::<Vec<_>>();
        if needs_synthetic_basic
            && !note_types
                .iter()
                .any(|note_type| note_type.key == SYNTHETIC_BASIC_NOTE_TYPE)
        {
            note_types.push(synthetic_basic_note_type());
        }

        let mut notes = Vec::with_capacity(state.notes.len() + state.cards.len());
        for note in &state.notes {
            if !note_types_by_id.contains_key(note.note_type_id.as_str()) {
                return Err(apkg_error(format!(
                    "Engram note {} references missing note type {}",
                    note.id, note.note_type_id
                )));
            }
            notes.push(ExportNote {
                key: note.id.clone(),
                note_type_key: note.note_type_id.clone(),
                fields: note.fields.clone(),
                tags: note.tags.clone(),
                created_at: note.created_at,
                updated_at: note.updated_at,
            });
        }

        let mut cards = Vec::with_capacity(state.cards.len());
        let mut marked_note_keys = BTreeSet::new();
        for card in &state.cards {
            let card_marked = progress_by_card
                .get(&card.id)
                .is_some_and(|progress| progress.marked_at.is_some());
            if let Some(lineage) = card
                .lineage
                .as_ref()
                .filter(|lineage| lineage_is_exportable(lineage, &notes_by_id, &note_types_by_id))
            {
                if card_marked {
                    marked_note_keys.insert(lineage.note_id.clone());
                }
                cards.push(ExportCard {
                    key: card.id.clone(),
                    note_key: lineage.note_id.clone(),
                    deck_key: fallback_deck_key(&card.deck_id, &default_deck_key),
                    template_ordinal: lineage.ordinal,
                    created_at: card.created_at,
                });
            } else {
                let note_key = synthetic_basic_note_key(&card.id);
                if card_marked {
                    marked_note_keys.insert(note_key.clone());
                }
                notes.push(ExportNote {
                    key: note_key.clone(),
                    note_type_key: SYNTHETIC_BASIC_NOTE_TYPE.to_string(),
                    fields: vec![
                        NoteFieldValue {
                            field_id: SYNTHETIC_BASIC_FRONT_FIELD.to_string(),
                            value: card.front.clone(),
                        },
                        NoteFieldValue {
                            field_id: SYNTHETIC_BASIC_BACK_FIELD.to_string(),
                            value: card.back.clone(),
                        },
                    ],
                    tags: Vec::new(),
                    created_at: card.created_at,
                    updated_at: card.created_at,
                });
                cards.push(ExportCard {
                    key: card.id.clone(),
                    note_key,
                    deck_key: fallback_deck_key(&card.deck_id, &default_deck_key),
                    template_ordinal: 0,
                    created_at: card.created_at,
                });
            }
        }
        for note in &mut notes {
            if marked_note_keys.contains(&note.key) {
                note.tags = tags_with_anki_marked(&note.tags);
            }
        }

        let computed_created_at_days = export_created_at_days(state, &decks, &notes, &cards);
        let mut session_deck_by_id: HashMap<String, String> = state
            .sessions
            .iter()
            .map(|session| {
                (
                    session.id.clone(),
                    fallback_deck_key(&session.deck_id, &default_deck_key),
                )
            })
            .collect();
        if let Some(active_session) = &state.active_session {
            session_deck_by_id.insert(
                active_session.session_id.clone(),
                fallback_deck_key(&active_session.deck_id, &default_deck_key),
            );
        }
        let created_at_days = state
            .external_sources
            .iter()
            .find(|source| {
                source.source == ANKI_V11_SOURCE
                    && source.target == ExternalSourceTarget::Collection
                    && source.target_id == "collection"
            })
            .and_then(|source| source_i64(Some(source), "createdAtDays"))
            .unwrap_or(computed_created_at_days);
        let modified_at_seconds = export_modified_at_seconds(state, &notes, &cards);
        let deck_ids = assign_anki_ids(decks.iter().map(|deck| deck.key.as_str()), 1_000_000);
        let note_type_ids = assign_anki_ids(
            note_types.iter().map(|note_type| note_type.key.as_str()),
            2_000_000,
        );
        let note_ids = assign_anki_ids(notes.iter().map(|note| note.key.as_str()), 3_000_000);
        let card_ids = assign_anki_ids(cards.iter().map(|card| card.key.as_str()), 4_000_000);

        Ok(Self {
            created_at_days,
            modified_at_seconds,
            decks,
            note_types,
            notes,
            cards,
            reviews: state.reviews.clone(),
            session_deck_by_id,
            progress_by_card,
            deck_options: state.deck_options.clone(),
            deck_ids,
            note_type_ids,
            note_ids,
            card_ids,
            external_sources: state.external_sources.clone(),
        })
    }
}

const SYNTHETIC_BASIC_NOTE_TYPE: &str = "engram-basic";
const SYNTHETIC_BASIC_FRONT_FIELD: &str = "engram-basic:field:0";
const SYNTHETIC_BASIC_BACK_FIELD: &str = "engram-basic:field:1";
const SYNTHETIC_BASIC_TEMPLATE: &str = "engram-basic:template:0";

fn create_v11_export_schema(connection: &Connection) -> Result<(), ApkgError> {
    connection
        .execute_batch(
            r#"
PRAGMA user_version = 11;
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
        .map_err(|err| apkg_error(format!("failed to create Anki V11 export schema: {err}")))
}

fn write_v11_export_rows(connection: &Connection, export: &ExportModel) -> Result<(), ApkgError> {
    let decks_json = serde_json::to_string(&export_decks_json(export))
        .map_err(|err| apkg_error(format!("failed to serialize Anki deck map: {err}")))?;
    let models_json = serde_json::to_string(&export_note_types_json(export))
        .map_err(|err| apkg_error(format!("failed to serialize Anki model map: {err}")))?;
    let config_json = serde_json::to_string(&export_collection_config_json(export))
        .map_err(|err| apkg_error(format!("failed to serialize Anki config map: {err}")))?;
    let deck_config_json = serde_json::to_string(&export_collection_deck_config_json(export))
        .map_err(|err| apkg_error(format!("failed to serialize Anki deck config map: {err}")))?;
    let tags_json = serde_json::to_string(&export_tags_json(export))
        .map_err(|err| apkg_error(format!("failed to serialize Anki tag map: {err}")))?;

    connection
        .execute(
            "INSERT INTO col VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            rusqlite::params![
                export_collection_i64(export, "id").unwrap_or(1_i64),
                export.created_at_days,
                export_collection_i64(export, "modifiedAt").unwrap_or(export.modified_at_seconds),
                export_collection_i64(export, "schemaModifiedAt")
                    .unwrap_or(export.modified_at_seconds),
                export_collection_i64(export, "version").unwrap_or(11_i64),
                export_collection_i64(export, "dirty").unwrap_or(0_i64),
                export_collection_i64(export, "updateSequenceNumber").unwrap_or(-1_i64),
                export_collection_i64(export, "lastSync").unwrap_or(0_i64),
                config_json,
                models_json,
                decks_json,
                deck_config_json,
                tags_json,
            ],
        )
        .map_err(|err| apkg_error(format!("failed to write Anki col row: {err}")))?;

    for note in &export.notes {
        let note_type = export
            .note_types
            .iter()
            .find(|note_type| note_type.key == note.note_type_key)
            .ok_or_else(|| {
                apkg_error(format!(
                    "Engram note {} references missing export note type {}",
                    note.key, note.note_type_key
                ))
            })?;
        let fields = export_note_field_values(note, note_type);
        let field_join = fields.join("\u{1f}");
        let source = anki_source(export, ExternalSourceTarget::Note, &note.key);
        let note_type_source =
            anki_source(export, ExternalSourceTarget::NoteType, &note.note_type_key);
        let sort_field = export_note_sort_field(&fields, note_type_source);
        connection
            .execute(
                "INSERT INTO notes VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                rusqlite::params![
                    export.note_ids[&note.key],
                    source_string(source, "guid")
                        .unwrap_or_else(|| export_note_guid(&note.key, export.note_ids[&note.key])),
                    export.note_type_ids[&note.note_type_key],
                    export_note_modified_at(note, source),
                    source_i64(source, "updateSequenceNumber").unwrap_or(-1_i64),
                    join_anki_tags(&note.tags),
                    field_join,
                    sort_field,
                    export_note_checksum(source, &sort_field),
                    source_i64(source, "flags").unwrap_or_default(),
                    source_string(source, "data").unwrap_or_default(),
                ],
            )
            .map_err(|err| apkg_error(format!("failed to write Anki note {}: {err}", note.key)))?;
    }

    for (index, card) in export.cards.iter().enumerate() {
        let progress = export.progress_by_card.get(&card.key);
        let source = anki_source(export, ExternalSourceTarget::Card, &card.key);
        let deck_options = export
            .deck_options
            .iter()
            .find(|preset| preset.deck_id == card.deck_key)
            .map(|preset| &preset.options);
        let scheduling = export_card_scheduling(
            progress,
            export.created_at_days,
            index,
            source,
            deck_options,
        );
        connection
            .execute(
                "INSERT INTO cards VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
                rusqlite::params![
                    export.card_ids[&card.key],
                    export.note_ids[&card.note_key],
                    export.deck_ids[&card.deck_key],
                    i64::from(card.template_ordinal),
                    export_card_modified_at(card, progress, source),
                    source_i64(source, "updateSequenceNumber").unwrap_or(-1_i64),
                    scheduling.kind,
                    scheduling.queue,
                    scheduling.due,
                    scheduling.interval,
                    scheduling.factor,
                    scheduling.repetitions,
                    scheduling.lapses,
                    scheduling.left,
                    source_i64(source, "originalDue").unwrap_or_default(),
                    export_original_deck_id(export, source),
                    scheduling.flags,
                    export_card_data(progress, source),
                ],
            )
            .map_err(|err| apkg_error(format!("failed to write Anki card {}: {err}", card.key)))?;
    }

    let mut used_review_ids = BTreeSet::new();
    for review in &export.reviews {
        let source = anki_source(export, ExternalSourceTarget::Review, &review.id);
        let card_id = export
            .card_ids
            .get(&review.card_id)
            .copied()
            .or_else(|| source_i64(source, "cardId"))
            .ok_or_else(|| {
                apkg_error(format!(
                    "Engram review {} references missing card {}",
                    review.id, review.card_id
                ))
            })?;
        let review_id = unique_review_id(review, &mut used_review_ids);
        connection
            .execute(
                "INSERT INTO revlog VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    review_id,
                    card_id,
                    source_i64(source, "updateSequenceNumber").unwrap_or(-1_i64),
                    source_i64(source, "ease").unwrap_or_else(|| rating_to_v11_ease(review.rating)),
                    source_i64(source, "interval").unwrap_or_else(|| {
                        review
                            .resulting_progress
                            .as_ref()
                            .map(|progress| i64::from(progress.interval))
                            .unwrap_or_default()
                    }),
                    source_i64(source, "lastInterval").unwrap_or_else(|| {
                        review
                            .previous_progress
                            .as_ref()
                            .map(|progress| i64::from(progress.interval))
                            .unwrap_or_default()
                    }),
                    source_i64(source, "factor").unwrap_or_else(|| {
                        review
                            .resulting_progress
                            .as_ref()
                            .map(progress_factor_to_anki)
                            .unwrap_or((INITIAL_EASE_FACTOR * 1000.0).round() as i64)
                    }),
                    source_i64(source, "time")
                        .or_else(|| review.answer_time_ms.map(i64::from))
                        .unwrap_or_default(),
                    source_i64(source, "kind").unwrap_or_else(|| review_kind(
                        export,
                        review,
                        &review.card_id
                    )),
                ],
            )
            .map_err(|err| {
                apkg_error(format!("failed to write Anki review {}: {err}", review.id))
            })?;
    }

    for grave in export_collection_graves(export) {
        connection
            .execute(
                "INSERT INTO graves VALUES (?1, ?2, ?3)",
                rusqlite::params![grave.update_sequence_number, grave.object_id, grave.kind],
            )
            .map_err(|err| {
                apkg_error(format!(
                    "failed to write Anki grave {}:{}: {err}",
                    grave.kind, grave.object_id
                ))
            })?;
    }

    Ok(())
}

fn export_decks_json(export: &ExportModel) -> Value {
    let mut object = serde_json::Map::new();
    for deck in &export.decks {
        let id = export.deck_ids[&deck.key];
        let source = anki_source(export, ExternalSourceTarget::Deck, &deck.key);
        let mut deck_json = source_json(source, "rawJson").unwrap_or_else(|| serde_json::json!({}));
        let deck_object = ensure_json_object(&mut deck_json);
        deck_object.insert("id".to_string(), Value::Number(id.into()));
        deck_object.insert("name".to_string(), Value::String(deck.name.clone()));
        deck_object.insert("desc".to_string(), Value::String(deck.description.clone()));
        deck_object
            .entry("mod".to_string())
            .or_insert_with(|| Value::Number(millis_to_anki_seconds(deck.created_at).into()));
        deck_object
            .entry("usn".to_string())
            .or_insert_with(|| Value::Number((-1_i64).into()));
        deck_object
            .entry("conf".to_string())
            .or_insert_with(|| Value::Number(1_i64.into()));
        if let Some(config_id) = export_deck_option_config_id(export, &deck.key) {
            deck_object.insert("conf".to_string(), Value::Number(config_id.into()));
        }
        deck_object
            .entry("dyn".to_string())
            .or_insert_with(|| Value::Number(0_i64.into()));
        deck_object
            .entry("extendNew".to_string())
            .or_insert_with(|| Value::Number(10_i64.into()));
        deck_object
            .entry("extendRev".to_string())
            .or_insert_with(|| Value::Number(50_i64.into()));
        merge_dynamic_deck_source_json(deck_object, source);
        object.insert(id.to_string(), deck_json);
    }
    Value::Object(object)
}

fn merge_dynamic_deck_source_json(
    deck_object: &mut serde_json::Map<String, Value>,
    source: Option<&ExternalSourceRecord>,
) {
    if let Some(dyn_value) = source_i64(source, "dyn") {
        deck_object.insert("dyn".to_string(), Value::Number(dyn_value.into()));
    }
    if let Some(reschedule) = source_bool(source, "resched") {
        deck_object.insert("resched".to_string(), Value::Bool(reschedule));
    }
    let Some(search) = source_string(source, "search") else {
        return;
    };
    let limit = source_i64(source, "limit").unwrap_or_default();
    let order = source_i64(source, "order").unwrap_or_default();
    deck_object.insert(
        "terms".to_string(),
        Value::Array(vec![Value::Array(vec![
            Value::String(search),
            Value::Number(limit.into()),
            Value::Number(order.into()),
        ])]),
    );
}

fn export_note_types_json(export: &ExportModel) -> Value {
    let mut object = serde_json::Map::new();
    for note_type in &export.note_types {
        let id = export.note_type_ids[&note_type.key];
        let mut model_json = anki_source_json(
            export,
            ExternalSourceTarget::NoteType,
            &note_type.key,
            "rawJson",
        )
        .unwrap_or_else(|| serde_json::json!({}));
        let raw_fields = model_json.get("flds").cloned().unwrap_or(Value::Null);
        let raw_templates = model_json.get("tmpls").cloned().unwrap_or(Value::Null);
        let fields = export_note_type_fields_json(note_type, &raw_fields);
        let templates =
            export_note_type_templates_json(note_type, &raw_templates, &export.deck_ids);
        let requirements = export_note_type_requirements_json(note_type);
        let model_object = ensure_json_object(&mut model_json);
        model_object.insert("id".to_string(), Value::Number(id.into()));
        model_object.insert("name".to_string(), Value::String(note_type.name.clone()));
        model_object.insert("type".to_string(), Value::Number(note_type.kind.into()));
        model_object.entry("mod".to_string()).or_insert_with(|| {
            Value::Number(
                millis_to_anki_seconds(note_type.updated_at.max(note_type.created_at)).into(),
            )
        });
        model_object
            .entry("usn".to_string())
            .or_insert_with(|| Value::Number((-1_i64).into()));
        model_object
            .entry("sortf".to_string())
            .or_insert_with(|| Value::Number(0_i64.into()));
        model_object.entry("did".to_string()).or_insert(Value::Null);
        if let Some(stylesheet) = &note_type.stylesheet {
            model_object.insert("css".to_string(), Value::String(stylesheet.clone()));
        } else {
            model_object.entry("css".to_string()).or_insert_with(|| {
                Value::String(
                    ".card { font-family: arial; font-size: 20px; text-align: center; color: black; background-color: white; }"
                        .to_string(),
                )
            });
        }
        model_object
            .entry("latexPre".to_string())
            .or_insert_with(|| Value::String("\\documentclass[12pt]{article}".to_string()));
        model_object
            .entry("latexPost".to_string())
            .or_insert_with(|| Value::String("\\end{document}".to_string()));
        model_object.insert("flds".to_string(), Value::Array(fields));
        model_object.insert("tmpls".to_string(), Value::Array(templates));
        model_object.insert("req".to_string(), Value::Array(requirements));
        object.insert(id.to_string(), model_json);
    }
    Value::Object(object)
}

fn export_tags_json(export: &ExportModel) -> Value {
    let mut object = export_collection_json(export, "tagsJson")
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();
    for tag in export.notes.iter().flat_map(|note| note.tags.iter()) {
        object.insert(tag.clone(), Value::Number(1.into()));
    }
    Value::Object(object)
}

fn export_collection_config_json(export: &ExportModel) -> Value {
    let mut config =
        export_collection_json(export, "configJson").unwrap_or_else(|| serde_json::json!({}));
    ensure_json_object(&mut config).insert(
        "nextPos".to_string(),
        Value::Number(i64::try_from(export.cards.len()).unwrap_or(i64::MAX).into()),
    );
    config
}

fn export_collection_deck_config_json(export: &ExportModel) -> Value {
    let mut deck_config = export_collection_json(export, "deckConfigJson")
        .unwrap_or_else(|| serde_json::json!({ "1": { "id": 1, "name": "Default" } }));
    let object = ensure_json_object(&mut deck_config);
    object.entry("1".to_string()).or_insert_with(|| {
        serde_json::json!({
            "id": 1,
            "name": "Default",
        })
    });

    for preset in &export.deck_options {
        let Some(config_id) = export_deck_option_config_id(export, &preset.deck_id) else {
            continue;
        };
        let mut config = object
            .get(&config_id.to_string())
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));
        let deck_name = export
            .decks
            .iter()
            .find(|deck| deck.key == preset.deck_id)
            .map(|deck| deck.name.as_str())
            .unwrap_or("Engram");
        merge_deck_options_json(&mut config, config_id, deck_name, &preset.options);
        object.insert(config_id.to_string(), config);
    }

    deck_config
}

fn export_collection_graves(export: &ExportModel) -> Vec<AnkiV11Grave> {
    let mut graves: Vec<AnkiV11Grave> = export_collection_json(export, "gravesJson")
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default();
    let mut seen = graves
        .iter()
        .map(|grave| (grave.kind, grave.object_id))
        .collect::<BTreeSet<_>>();

    for source in &export.external_sources {
        if let Some(grave) = deleted_source_grave(source) {
            if seen.insert((grave.kind, grave.object_id)) {
                graves.push(grave);
            }
        }
    }

    graves
}

fn deleted_source_grave(source: &ExternalSourceRecord) -> Option<AnkiV11Grave> {
    if source.source != ANKI_V11_SOURCE || source.target != ExternalSourceTarget::Deleted {
        return None;
    }
    let kind = match source.data.get("deletedTarget")?.as_str() {
        "card" => 0,
        "note" => 1,
        "deck" => 2,
        _ => return None,
    };
    Some(AnkiV11Grave {
        update_sequence_number: source_i64(Some(source), "updateSequenceNumber").unwrap_or(-1),
        object_id: source.original_id.as_deref()?.parse::<i64>().ok()?,
        kind,
    })
}

fn export_collection_i64(export: &ExportModel, key: &str) -> Option<i64> {
    anki_source(export, ExternalSourceTarget::Collection, "collection")
        .and_then(|source| source_i64(Some(source), key))
}

fn export_collection_json(export: &ExportModel, key: &str) -> Option<Value> {
    anki_source(export, ExternalSourceTarget::Collection, "collection")
        .and_then(|source| source_json(Some(source), key))
}

fn export_deck_option_config_id(export: &ExportModel, deck_key: &str) -> Option<i64> {
    if !export
        .deck_options
        .iter()
        .any(|preset| preset.deck_id == deck_key)
    {
        return None;
    }

    anki_source_json(export, ExternalSourceTarget::Deck, deck_key, "rawJson")
        .as_ref()
        .and_then(|raw| json_i64(raw, "conf"))
        .filter(|config_id| *config_id > 0)
        .or_else(|| export.deck_ids.get(deck_key).copied())
        .or(Some(1))
}

fn merge_deck_options_json(
    config: &mut Value,
    config_id: i64,
    deck_name: &str,
    options: &DeckOptions,
) {
    let object = ensure_json_object(config);
    object.insert("id".to_string(), Value::Number(config_id.into()));
    object
        .entry("name".to_string())
        .or_insert_with(|| Value::String(format!("Engram {deck_name}")));

    let mut new_section = object
        .get("new")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let new_object = ensure_json_object(&mut new_section);
    new_object.insert(
        "perDay".to_string(),
        Value::Number(i64::from(options.new_cards_per_day).into()),
    );
    new_object.insert("bury".to_string(), Value::Bool(options.bury_new_siblings));
    new_object.insert(
        "delays".to_string(),
        Value::Array(
            options
                .learning_steps_minutes
                .iter()
                .map(|minutes| Value::Number(i64::from(*minutes).into()))
                .collect(),
        ),
    );
    new_object.insert(
        "ints".to_string(),
        Value::Array(vec![
            Value::Number(i64::from(options.graduating_interval_days).into()),
            Value::Number(i64::from(options.easy_interval_days).into()),
        ]),
    );
    new_object.insert(
        "initialFactor".to_string(),
        Value::Number(
            ((finite_positive(options.initial_ease_factor, INITIAL_EASE_FACTOR) * 1000.0)
                .round()
                .clamp(0.0, i64::MAX as f64) as i64)
                .into(),
        ),
    );
    object.insert("new".to_string(), new_section);

    let mut review_section = object
        .get("rev")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let review_object = ensure_json_object(&mut review_section);
    review_object.insert(
        "perDay".to_string(),
        Value::Number(i64::from(options.reviews_per_day).into()),
    );
    review_object.insert(
        "bury".to_string(),
        Value::Bool(options.bury_review_siblings),
    );
    review_object.insert(
        "maxIvl".to_string(),
        Value::Number(i64::from(options.maximum_interval_days.max(1)).into()),
    );
    review_object.insert(
        "ivlFct".to_string(),
        json_f64_or(options.review_interval_modifier, 1.0),
    );
    review_object.insert(
        "hardFactor".to_string(),
        json_f64_or(options.hard_interval_multiplier, 1.2),
    );
    review_object.insert(
        "ease4".to_string(),
        json_f64_or(options.easy_bonus_multiplier, 1.3),
    );
    object.insert("rev".to_string(), review_section);

    let mut lapse_section = object
        .get("lapse")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let lapse_object = ensure_json_object(&mut lapse_section);
    lapse_object.insert(
        "delays".to_string(),
        Value::Array(
            options
                .relearning_steps_minutes
                .iter()
                .map(|minutes| Value::Number(i64::from(*minutes).into()))
                .collect(),
        ),
    );
    lapse_object.insert(
        "mult".to_string(),
        json_f64_or(options.lapse_interval_multiplier, 0.0),
    );
    lapse_object.insert(
        "leechFails".to_string(),
        Value::Number(i64::from(options.leech_threshold).into()),
    );
    lapse_object.insert(
        "leechAction".to_string(),
        Value::Number(leech_action_to_anki(options.leech_action).into()),
    );
    object.insert("lapse".to_string(), lapse_section);
    object.insert(
        "buryInterdayLearning".to_string(),
        Value::Bool(options.bury_interday_learning_siblings),
    );
    object.insert(
        "desiredRetention".to_string(),
        json_f64_or(options.desired_retention, 0.9),
    );
    if !options.fsrs_parameters.is_empty() {
        object.insert(
            "fsrsParams6".to_string(),
            json_f64_array_or(&options.fsrs_parameters),
        );
    }
    object.insert(
        "weightSearch".to_string(),
        Value::String(options.fsrs_parameter_search.clone()),
    );
    object.insert(
        "ignoreRevlogsBeforeDate".to_string(),
        Value::String(options.ignore_review_history_before.clone()),
    );
    object.insert(
        "sm2Retention".to_string(),
        json_f64_or(options.historical_retention, 0.9),
    );
    object.insert(
        "easyDaysPercentages".to_string(),
        json_f64_array_or(&options.easy_days_percentages),
    );
}

fn json_f64_or(value: f64, fallback: f64) -> Value {
    let normalized = if value.is_finite() { value } else { fallback };
    serde_json::Number::from_f64(normalized)
        .map(Value::Number)
        .unwrap_or_else(|| Value::Number(0_i64.into()))
}

fn json_f64_array_or(values: &[f64]) -> Value {
    Value::Array(
        values
            .iter()
            .filter(|value| value.is_finite())
            .map(|value| json_f64_or(*value, 0.0))
            .collect(),
    )
}

fn finite_positive(value: f64, fallback: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
    }
}

fn leech_action_to_anki(action: LeechAction) -> i64 {
    match action {
        LeechAction::Suspend => 0,
        LeechAction::TagOnly => 1,
    }
}

fn export_note_type_fields_json(note_type: &ExportNoteType, raw_fields: &Value) -> Vec<Value> {
    let raw_by_ordinal = raw_values_by_ordinal(raw_fields);
    let mut fields = note_type.fields.clone();
    fields.sort_by_key(|field| field.ordinal);
    fields
        .into_iter()
        .map(|field| {
            let mut field_json = raw_by_ordinal
                .get(&(i64::from(field.ordinal)))
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));
            let object = ensure_json_object(&mut field_json);
            object.insert("name".to_string(), Value::String(field.name));
            object.insert(
                "ord".to_string(),
                Value::Number(i64::from(field.ordinal).into()),
            );
            object
                .entry("sticky".to_string())
                .or_insert(Value::Bool(false));
            object
                .entry("rtl".to_string())
                .or_insert(Value::Bool(false));
            object
                .entry("font".to_string())
                .or_insert_with(|| Value::String("Arial".to_string()));
            object
                .entry("size".to_string())
                .or_insert_with(|| Value::Number(20_i64.into()));
            field_json
        })
        .collect()
}

fn export_note_type_templates_json(
    note_type: &ExportNoteType,
    raw_templates: &Value,
    deck_ids: &BTreeMap<String, i64>,
) -> Vec<Value> {
    let raw_by_ordinal = raw_values_by_ordinal(raw_templates);
    let mut templates = note_type.templates.clone();
    templates.sort_by_key(|template| template.ordinal);
    templates
        .into_iter()
        .map(|template| {
            let mut template_json = raw_by_ordinal
                .get(&(i64::from(template.ordinal)))
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));
            let object = ensure_json_object(&mut template_json);
            object.insert("name".to_string(), Value::String(template.name));
            object.insert(
                "ord".to_string(),
                Value::Number(i64::from(template.ordinal).into()),
            );
            object.insert("qfmt".to_string(), Value::String(template.front_template));
            object.insert("afmt".to_string(), Value::String(template.back_template));
            object.insert(
                "did".to_string(),
                export_template_deck_id(template.deck_id.as_deref(), deck_ids)
                    .map(|deck_id| Value::Number(deck_id.into()))
                    .unwrap_or(Value::Null),
            );
            object
                .entry("bqfmt".to_string())
                .or_insert_with(|| Value::String(String::new()));
            object
                .entry("bafmt".to_string())
                .or_insert_with(|| Value::String(String::new()));
            template_json
        })
        .collect()
}

fn export_template_deck_id(deck_id: Option<&str>, deck_ids: &BTreeMap<String, i64>) -> Option<i64> {
    let deck_id = deck_id?;
    deck_ids
        .get(deck_id)
        .copied()
        .or_else(|| deck_id.parse::<i64>().ok())
        .filter(|deck_id| *deck_id > 0)
}

fn export_note_type_requirements_json(note_type: &ExportNoteType) -> Vec<Value> {
    let field_ordinals_by_name = note_type
        .fields
        .iter()
        .map(|field| (field.name.as_str(), i64::from(field.ordinal)))
        .collect::<HashMap<_, _>>();
    let mut templates = note_type.templates.clone();
    templates.sort_by_key(|template| template.ordinal);
    templates
        .into_iter()
        .map(|template| {
            let field_ordinals = template
                .required_field_names
                .iter()
                .filter_map(|field_name| field_ordinals_by_name.get(field_name.as_str()).copied())
                .map(|ordinal| Value::Number(ordinal.into()))
                .collect::<Vec<_>>();
            let mode = match template.requirement_mode {
                TemplateRequirementMode::Any => "any",
                TemplateRequirementMode::All => "all",
            };
            Value::Array(vec![
                Value::Number(i64::from(template.ordinal).into()),
                Value::String(mode.to_string()),
                Value::Array(field_ordinals),
            ])
        })
        .collect()
}

fn raw_values_by_ordinal(value: &Value) -> BTreeMap<i64, Value> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(index, raw)| (json_i64(raw, "ord").unwrap_or(index as i64), raw.clone()))
        .collect()
}

fn ensure_json_object(value: &mut Value) -> &mut serde_json::Map<String, Value> {
    if !value.is_object() {
        *value = Value::Object(serde_json::Map::new());
    }
    value
        .as_object_mut()
        .expect("value was just made an object")
}

fn anki_source<'a>(
    export: &'a ExportModel,
    target: ExternalSourceTarget,
    target_id: &str,
) -> Option<&'a ExternalSourceRecord> {
    export.external_sources.iter().find(|source| {
        source.source == ANKI_V11_SOURCE && source.target == target && source.target_id == target_id
    })
}

fn anki_source_json(
    export: &ExportModel,
    target: ExternalSourceTarget,
    target_id: &str,
    key: &str,
) -> Option<Value> {
    anki_source(export, target, target_id).and_then(|source| source_json(Some(source), key))
}

fn source_string(source: Option<&ExternalSourceRecord>, key: &str) -> Option<String> {
    source.and_then(|source| source.data.get(key).cloned())
}

fn source_i64(source: Option<&ExternalSourceRecord>, key: &str) -> Option<i64> {
    source
        .and_then(|source| source.data.get(key))
        .and_then(|value| value.parse().ok())
}

fn source_bool(source: Option<&ExternalSourceRecord>, key: &str) -> Option<bool> {
    let value = source.and_then(|source| source.data.get(key))?.trim();
    if value.eq_ignore_ascii_case("true") || value == "1" {
        Some(true)
    } else if value.eq_ignore_ascii_case("false") || value == "0" {
        Some(false)
    } else {
        None
    }
}

fn source_json(source: Option<&ExternalSourceRecord>, key: &str) -> Option<Value> {
    source
        .and_then(|source| source.data.get(key))
        .and_then(|value| serde_json::from_str(value).ok())
}

fn export_card_data(
    progress: Option<&CardProgress>,
    source: Option<&ExternalSourceRecord>,
) -> String {
    let Some(progress) = progress else {
        return source_string(source, "data").unwrap_or_default();
    };
    let fsrs_stability = progress
        .fsrs_stability
        .filter(|value| value.is_finite() && *value > 0.0);
    let fsrs_difficulty = progress.fsrs_difficulty.filter(|value| value.is_finite());
    if fsrs_stability.is_none() && fsrs_difficulty.is_none() {
        return source_string(source, "data").unwrap_or_default();
    }

    let mut data = source_string(source, "data")
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|value| match value {
            Value::Object(data) => Some(data),
            _ => None,
        })
        .unwrap_or_default();
    if let Some(value) = fsrs_stability.and_then(serde_json::Number::from_f64) {
        data.insert("s".to_string(), Value::Number(value));
    }
    if let Some(value) = fsrs_difficulty.and_then(serde_json::Number::from_f64) {
        data.insert("d".to_string(), Value::Number(value));
    }
    Value::Object(data).to_string()
}

fn export_original_deck_id(export: &ExportModel, source: Option<&ExternalSourceRecord>) -> i64 {
    let Some(original_deck_id) = source.and_then(|source| source.data.get("originalDeckId")) else {
        return 0;
    };
    let original_deck_id = original_deck_id.trim();
    if original_deck_id.is_empty() || original_deck_id == "0" {
        return 0;
    }
    original_deck_id
        .parse::<i64>()
        .ok()
        .filter(|deck_id| *deck_id != 0)
        .or_else(|| export.deck_ids.get(original_deck_id).copied())
        .unwrap_or_default()
}

fn export_note_sort_field(
    fields: &[String],
    note_type_source: Option<&ExternalSourceRecord>,
) -> String {
    let sort_index = source_json(note_type_source, "rawJson")
        .and_then(|raw| json_i64(&raw, "sortf"))
        .and_then(|index| usize::try_from(index).ok())
        .filter(|index| *index < fields.len())
        .unwrap_or_default();
    fields.get(sort_index).cloned().unwrap_or_default()
}

fn export_note_checksum(source: Option<&ExternalSourceRecord>, sort_field: &str) -> i64 {
    let source_sort_field = source_string(source, "sortField");
    if source_sort_field.as_deref() == Some(sort_field) {
        source_i64(source, "checksum").unwrap_or_else(|| anki_field_checksum(sort_field))
    } else {
        anki_field_checksum(sort_field)
    }
}

fn export_note_field_values(note: &ExportNote, note_type: &ExportNoteType) -> Vec<String> {
    let fields_by_id: HashMap<&str, &str> = note
        .fields
        .iter()
        .map(|field| (field.field_id.as_str(), field.value.as_str()))
        .collect();
    let mut fields = note_type.fields.clone();
    fields.sort_by_key(|field| field.ordinal);
    fields
        .iter()
        .map(|field| {
            fields_by_id
                .get(field.id.as_str())
                .copied()
                .unwrap_or_default()
                .to_string()
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ExportCardScheduling {
    kind: i64,
    queue: i64,
    due: i64,
    interval: i64,
    factor: i64,
    repetitions: i64,
    lapses: i64,
    left: i64,
    flags: i64,
}

fn export_card_scheduling(
    progress: Option<&CardProgress>,
    collection_created_at_days: i64,
    index: usize,
    source: Option<&ExternalSourceRecord>,
    deck_options: Option<&DeckOptions>,
) -> ExportCardScheduling {
    let Some(progress) = progress else {
        return ExportCardScheduling {
            kind: source_i64(source, "kind").unwrap_or(0),
            queue: source_i64(source, "queue").unwrap_or(0),
            due: source_i64(source, "due").unwrap_or(index.saturating_add(1) as i64),
            interval: source_i64(source, "interval").unwrap_or(0),
            factor: source_i64(source, "factor").unwrap_or_else(|| {
                let initial_ease = deck_options
                    .map(|options| options.initial_ease_factor)
                    .unwrap_or(INITIAL_EASE_FACTOR);
                (finite_positive(initial_ease, INITIAL_EASE_FACTOR) * 1000.0).round() as i64
            }),
            repetitions: source_i64(source, "repetitions").unwrap_or(0),
            lapses: source_i64(source, "lapses").unwrap_or(0),
            left: source_i64(source, "left").unwrap_or(0),
            flags: source_i64(source, "flags").unwrap_or(0),
        };
    };
    if is_export_metadata_overlay(progress) {
        return ExportCardScheduling {
            kind: source_i64(source, "kind").unwrap_or(0),
            queue: source_i64(source, "queue").unwrap_or(0),
            due: source_i64(source, "due").unwrap_or(index.saturating_add(1) as i64),
            interval: 0,
            factor: progress_factor_to_anki(progress),
            repetitions: 0,
            lapses: 0,
            left: source_i64(source, "left").unwrap_or(0),
            flags: progress
                .flag
                .map(card_flag_to_anki)
                .or_else(|| source_i64(source, "flags"))
                .unwrap_or_default(),
        };
    }

    let (kind, queue, due) = match progress.state {
        CardState::Learning => {
            let (queue, due) = learning_queue_and_due(progress, collection_created_at_days, source);
            (1, queue, due)
        }
        CardState::Relearning => {
            let (queue, due) = learning_queue_and_due(progress, collection_created_at_days, source);
            (3, queue, due)
        }
        CardState::Suspended => (
            review_or_new_kind(progress),
            preserved_source_queue(source, &[-1], -1),
            millis_to_anki_due_day(collection_created_at_days, progress.next_due_at),
        ),
        CardState::Buried => (
            review_or_new_kind(progress),
            preserved_source_queue(source, &[-2, -3], -2),
            millis_to_anki_due_day(collection_created_at_days, progress.next_due_at),
        ),
        CardState::Review => (
            review_or_new_kind(progress),
            2,
            millis_to_anki_due_day(collection_created_at_days, progress.next_due_at),
        ),
    };

    ExportCardScheduling {
        kind,
        queue,
        due,
        interval: i64::from(progress.interval),
        factor: progress_factor_to_anki(progress),
        repetitions: i64::from(progress.times_seen),
        lapses: i64::from(progress.times_incorrect),
        left: learning_step_index_to_anki_left(progress, source, deck_options),
        flags: progress
            .flag
            .map(card_flag_to_anki)
            .or_else(|| source_i64(source, "flags"))
            .unwrap_or_default(),
    }
}

fn learning_queue_and_due(
    progress: &CardProgress,
    collection_created_at_days: i64,
    source: Option<&ExternalSourceRecord>,
) -> (i64, i64) {
    let queue = source_i64(source, "queue")
        .filter(|queue| matches!(*queue, 1 | 3))
        .unwrap_or_else(|| {
            if progress.next_due_at.saturating_sub(progress.last_seen_at) >= ONE_DAY_MS {
                3
            } else {
                1
            }
        });
    let due = if queue == 3 {
        millis_to_anki_due_day(collection_created_at_days, progress.next_due_at)
    } else {
        millis_to_anki_seconds(progress.next_due_at).max(1)
    };
    (queue, due)
}

fn learning_step_index_to_anki_left(
    progress: &CardProgress,
    source: Option<&ExternalSourceRecord>,
    deck_options: Option<&DeckOptions>,
) -> i64 {
    let Some(step_index) = progress.learning_step_index else {
        return source_i64(source, "left").unwrap_or_default();
    };
    let step_count = learning_step_count_for_state(progress.state, deck_options);
    if step_count == 0 {
        return 0;
    }

    let clamped_index = (step_index as usize).min(step_count.saturating_sub(1));
    let remaining = step_count.saturating_sub(clamped_index) as i64;
    let learning_today = source_i64(source, "left")
        .filter(|left| *left > 0)
        .map(|left| (left / 1000) * 1000)
        .unwrap_or_default();
    learning_today + remaining
}

fn learning_step_count_for_state(state: CardState, deck_options: Option<&DeckOptions>) -> usize {
    match state {
        CardState::Learning => deck_options
            .map(|options| options.learning_steps_minutes.len())
            .unwrap_or_else(|| DeckOptions::default().learning_steps_minutes.len()),
        CardState::Relearning => deck_options
            .map(|options| options.relearning_steps_minutes.len())
            .unwrap_or_else(|| DeckOptions::default().relearning_steps_minutes.len()),
        _ => 0,
    }
}

fn preserved_source_queue(
    source: Option<&ExternalSourceRecord>,
    allowed: &[i64],
    fallback: i64,
) -> i64 {
    source_i64(source, "queue")
        .filter(|queue| allowed.contains(queue))
        .unwrap_or(fallback)
}

fn review_or_new_kind(progress: &CardProgress) -> i64 {
    if progress.times_seen == 0 && progress.interval == 0 {
        0
    } else {
        2
    }
}

fn is_export_metadata_overlay(progress: &CardProgress) -> bool {
    progress.state == CardState::Review
        && progress.interval == 0
        && progress.learning_step_index.is_none()
        && progress.buried_until.is_none()
        && progress.suspended_at.is_none()
        && progress.times_seen == 0
        && progress.times_correct == 0
        && progress.times_incorrect == 0
}

fn progress_factor_to_anki(progress: &CardProgress) -> i64 {
    (progress.ease_factor * 1000.0)
        .round()
        .clamp(0.0, i64::MAX as f64) as i64
}

fn card_flag_to_anki(flag: CardFlag) -> i64 {
    match flag {
        CardFlag::Red => 1,
        CardFlag::Orange => 2,
        CardFlag::Green => 3,
        CardFlag::Blue => 4,
        CardFlag::Pink => 5,
        CardFlag::Turquoise => 6,
        CardFlag::Purple => 7,
    }
}

fn export_note_modified_at(note: &ExportNote, source: Option<&ExternalSourceRecord>) -> i64 {
    source_i64(source, "modifiedAt")
        .unwrap_or_else(|| millis_to_anki_seconds(note.updated_at.max(note.created_at)))
}

fn export_card_modified_at(
    card: &ExportCard,
    progress: Option<&CardProgress>,
    source: Option<&ExternalSourceRecord>,
) -> i64 {
    source_i64(source, "modifiedAt").unwrap_or_else(|| {
        let timestamp = progress
            .map(|progress| progress.last_seen_at.max(card.created_at))
            .unwrap_or(card.created_at);
        millis_to_anki_seconds(timestamp)
    })
}

fn rating_to_v11_ease(rating: Rating) -> i64 {
    match rating {
        Rating::Again => 1,
        Rating::Hard => 2,
        Rating::Good => 3,
        Rating::Easy => 4,
    }
}

fn review_kind(export: &ExportModel, review: &Review, card_key: &str) -> i64 {
    if review_is_from_dynamic_deck(export, review, card_key) {
        return 3;
    }

    if let Some(progress) = review.previous_progress.as_ref() {
        return match progress.state {
            CardState::Learning => 0,
            CardState::Relearning => 2,
            _ => 1,
        };
    }

    match review.resulting_progress.as_ref() {
        Some(progress) if progress.state == CardState::Learning => 0,
        Some(progress) if progress.state == CardState::Relearning => 2,
        Some(progress) if progress.state == CardState::Review && progress.times_seen <= 1 => 0,
        _ => 1,
    }
}

fn review_is_from_dynamic_deck(export: &ExportModel, review: &Review, card_key: &str) -> bool {
    let deck_key = export
        .session_deck_by_id
        .get(&review.session_id)
        .map(String::as_str)
        .or_else(|| {
            review
                .previous_active_session
                .as_ref()
                .map(|session| session.deck_id.as_str())
        })
        .or_else(|| {
            export
                .cards
                .iter()
                .find(|card| card.key == card_key)
                .map(|card| card.deck_key.as_str())
        });
    deck_key.is_some_and(|deck_key| is_dynamic_anki_deck(export, deck_key))
}

fn is_dynamic_anki_deck(export: &ExportModel, deck_key: &str) -> bool {
    let source = anki_source(export, ExternalSourceTarget::Deck, deck_key);
    source_i64(source, "dyn").is_some_and(|dyn_value| dyn_value != 0)
        || source_json(source, "rawJson")
            .and_then(|raw| json_i64(&raw, "dyn"))
            .is_some_and(|dyn_value| dyn_value != 0)
}

fn unique_review_id(review: &Review, used: &mut BTreeSet<i64>) -> i64 {
    let mut candidate = review
        .id
        .parse::<i64>()
        .ok()
        .filter(|id| *id > 0)
        .unwrap_or_else(|| i64::try_from(review.reviewed_at).unwrap_or(i64::MAX));
    if candidate <= 0 {
        candidate = 1;
    }
    while !used.insert(candidate) {
        candidate = candidate.saturating_add(1);
    }
    candidate
}

fn assign_anki_ids<'a>(
    keys: impl IntoIterator<Item = &'a str>,
    generated_base: i64,
) -> BTreeMap<String, i64> {
    let mut mapped = BTreeMap::new();
    let mut used = BTreeSet::new();
    let mut next_generated = generated_base.max(1);

    for key in keys {
        if mapped.contains_key(key) {
            continue;
        }
        let id = key
            .parse::<i64>()
            .ok()
            .filter(|id| *id > 0 && !used.contains(id))
            .unwrap_or_else(|| {
                while used.contains(&next_generated) {
                    next_generated = next_generated.saturating_add(1);
                }
                let id = next_generated;
                next_generated = next_generated.saturating_add(1);
                id
            });
        used.insert(id);
        mapped.insert(key.to_string(), id);
    }

    mapped
}

fn has_exportable_lineage(
    card: &Card,
    notes_by_id: &HashMap<&str, &Note>,
    note_types_by_id: &HashMap<&str, &NoteType>,
) -> bool {
    card.lineage
        .as_ref()
        .is_some_and(|lineage| lineage_is_exportable(lineage, notes_by_id, note_types_by_id))
}

fn lineage_is_exportable(
    lineage: &CardLineage,
    notes_by_id: &HashMap<&str, &Note>,
    note_types_by_id: &HashMap<&str, &NoteType>,
) -> bool {
    notes_by_id.contains_key(lineage.note_id.as_str())
        && note_types_by_id
            .get(lineage.note_type_id.as_str())
            .is_some_and(|note_type| {
                note_type
                    .templates
                    .iter()
                    .any(|template| template.id == lineage.template_id)
            })
}

fn note_type_kind(note_type: &NoteType) -> i64 {
    if note_type.templates.iter().any(|template| {
        template_references_cloze(&template.front_template, &template.back_template)
    }) {
        1
    } else {
        0
    }
}

fn synthetic_basic_note_type() -> ExportNoteType {
    ExportNoteType {
        key: SYNTHETIC_BASIC_NOTE_TYPE.to_string(),
        name: "Engram Basic".to_string(),
        kind: 0,
        fields: vec![
            FieldDef {
                id: SYNTHETIC_BASIC_FRONT_FIELD.to_string(),
                name: "Front".to_string(),
                required: true,
                ordinal: 0,
            },
            FieldDef {
                id: SYNTHETIC_BASIC_BACK_FIELD.to_string(),
                name: "Back".to_string(),
                required: true,
                ordinal: 1,
            },
        ],
        templates: vec![CardTemplate {
            id: SYNTHETIC_BASIC_TEMPLATE.to_string(),
            name: "Card 1".to_string(),
            front_template: "{{Front}}".to_string(),
            back_template: "{{Back}}".to_string(),
            deck_id: None,
            required_field_names: vec!["Front".to_string()],
            requirement_mode: TemplateRequirementMode::All,
            ordinal: 0,
        }],
        stylesheet: None,
        created_at: 0,
        updated_at: 0,
    }
}

fn synthetic_basic_note_key(card_id: &str) -> String {
    format!("engram-basic-note:{card_id}")
}

fn fallback_deck_key(deck_id: &str, default_deck_key: &str) -> String {
    if deck_id.is_empty() {
        default_deck_key.to_string()
    } else {
        deck_id.to_string()
    }
}

fn export_created_at_days(
    state: &AppState,
    decks: &[ExportDeck],
    notes: &[ExportNote],
    cards: &[ExportCard],
) -> i64 {
    let earliest = decks
        .iter()
        .map(|deck| deck.created_at)
        .chain(notes.iter().map(|note| note.created_at))
        .chain(cards.iter().map(|card| card.created_at))
        .chain(
            state
                .card_progress
                .iter()
                .map(|progress| progress.last_seen_at),
        )
        .chain(state.reviews.iter().map(|review| review.reviewed_at))
        .filter(|timestamp| *timestamp > 0)
        .min()
        .unwrap_or_default();
    (earliest / ONE_DAY_MS) as i64
}

fn export_modified_at_seconds(state: &AppState, notes: &[ExportNote], cards: &[ExportCard]) -> i64 {
    let latest = notes
        .iter()
        .map(|note| note.updated_at.max(note.created_at))
        .chain(cards.iter().map(|card| card.created_at))
        .chain(
            state
                .card_progress
                .iter()
                .map(|progress| progress.last_seen_at),
        )
        .chain(state.reviews.iter().map(|review| review.reviewed_at))
        .max()
        .unwrap_or_default();
    millis_to_anki_seconds(latest)
}

fn millis_to_anki_seconds(millis: u64) -> i64 {
    i64::try_from(millis / 1000).unwrap_or(i64::MAX)
}

fn millis_to_anki_due_day(collection_created_at_days: i64, millis: u64) -> i64 {
    let absolute_day = i64::try_from(millis / ONE_DAY_MS).unwrap_or(i64::MAX);
    absolute_day.saturating_sub(collection_created_at_days)
}

fn export_note_guid(note_key: &str, note_id: i64) -> String {
    if note_key
        .chars()
        .all(|value| value.is_ascii_alphanumeric() || value == '_')
        && note_key.len() <= 64
    {
        note_key.to_string()
    } else {
        format!("engram-{note_id}")
    }
}

fn join_anki_tags(tags: &[String]) -> String {
    let normalized = tags
        .iter()
        .filter_map(|tag| {
            let tag = tag.trim();
            (!tag.is_empty()).then_some(tag)
        })
        .collect::<Vec<_>>();
    if normalized.is_empty() {
        String::new()
    } else {
        format!(" {} ", normalized.join(" "))
    }
}

fn tags_with_anki_marked(tags: &[String]) -> Vec<String> {
    if tags.iter().any(|tag| tag.trim() == ANKI_MARKED_TAG) {
        return tags.to_vec();
    }

    let mut next = tags
        .iter()
        .filter(|tag| !tag.trim().eq_ignore_ascii_case(ANKI_MARKED_TAG))
        .cloned()
        .collect::<Vec<_>>();
    next.push(ANKI_MARKED_TAG.to_string());
    next
}

fn anki_field_checksum(sort_field: &str) -> i64 {
    let digest = sum1(sort_field.as_bytes());
    u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]]) as i64
}

fn map_v11_note_type(note_type: &AnkiV11NoteType) -> NoteType {
    let id = note_type.id.to_string();
    let fields = note_type
        .fields
        .iter()
        .map(|field| FieldDef {
            id: field_id(note_type.id, field.ordinal),
            name: field.name.clone(),
            required: false,
            ordinal: i64_to_u32(field.ordinal),
        })
        .collect::<Vec<_>>();
    let templates = note_type
        .templates
        .iter()
        .map(|template| {
            let requirement = requirement_for_anki_template(note_type, template);
            CardTemplate {
                id: template_id(note_type.id, template.ordinal),
                name: template.name.clone(),
                front_template: template.question_format.clone(),
                back_template: template.answer_format.clone(),
                deck_id: template
                    .deck_id
                    .filter(|deck_id| *deck_id > 0)
                    .map(|deck_id| deck_id.to_string()),
                required_field_names: requirement.field_names,
                requirement_mode: requirement.mode,
                ordinal: i64_to_u32(template.ordinal),
            }
        })
        .collect();

    NoteType {
        id,
        name: note_type.name.clone(),
        fields,
        templates,
        stylesheet: (!note_type.css.trim().is_empty()).then(|| note_type.css.clone()),
        created_at: 0,
        updated_at: 0,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TemplateRequirement {
    field_names: Vec<String>,
    mode: TemplateRequirementMode,
}

fn requirement_for_anki_template(
    note_type: &AnkiV11NoteType,
    template: &AnkiV11Template,
) -> TemplateRequirement {
    anki_req_requirement_for_template(note_type, template).unwrap_or_else(|| TemplateRequirement {
        field_names: inferred_required_field_names_for_anki_template(note_type, template),
        mode: TemplateRequirementMode::All,
    })
}

fn anki_req_requirement_for_template(
    note_type: &AnkiV11NoteType,
    template: &AnkiV11Template,
) -> Option<TemplateRequirement> {
    let field_names_by_ordinal = note_type
        .fields
        .iter()
        .map(|field| (field.ordinal, field.name.as_str()))
        .collect::<BTreeMap<_, _>>();
    let req_rows = note_type.raw.get("req")?.as_array()?;
    for row in req_rows {
        let row = row.as_array()?;
        if row.first().and_then(Value::as_i64) != Some(template.ordinal) {
            continue;
        }
        let mode = match row.get(1).and_then(Value::as_str) {
            Some("any") => TemplateRequirementMode::Any,
            Some("all") => TemplateRequirementMode::All,
            _ => return None,
        };
        let field_names = row
            .get(2)
            .and_then(Value::as_array)?
            .iter()
            .filter_map(Value::as_i64)
            .filter_map(|ordinal| field_names_by_ordinal.get(&ordinal).copied())
            .map(str::to_string)
            .collect::<Vec<_>>();
        return Some(TemplateRequirement { field_names, mode });
    }
    None
}

fn inferred_required_field_names_for_anki_template(
    note_type: &AnkiV11NoteType,
    template: &AnkiV11Template,
) -> Vec<String> {
    let field_names = note_type
        .fields
        .iter()
        .map(|field| field.name.as_str())
        .collect::<BTreeSet<_>>();
    let mut required = BTreeSet::new();
    collect_anki_template_field_references(&template.question_format, &field_names, &mut required);
    required.into_iter().map(str::to_string).collect()
}

fn collect_anki_template_field_references<'a>(
    template: &str,
    field_names: &BTreeSet<&'a str>,
    required: &mut BTreeSet<&'a str>,
) {
    let mut rest = template;

    while let Some(start) = rest.find("{{") {
        let (_, after_start) = rest.split_at(start);
        let after_start = &after_start[2..];
        match after_start.find("}}") {
            Some(end) => {
                let (tag, after_end) = after_start.split_at(end);
                if let Some(field_name) = normalize_anki_template_field_tag(tag.trim()) {
                    if let Some(field_name) = field_names.get(field_name) {
                        required.insert(*field_name);
                    }
                }
                rest = &after_end[2..];
            }
            None => break,
        }
    }
}

fn normalize_anki_template_field_tag(tag: &str) -> Option<&str> {
    if tag.starts_with('#') || tag.starts_with('^') || tag.starts_with('/') {
        return None;
    }

    let tag = tag.trim();
    if tag.is_empty() {
        return None;
    }

    let mut field_name = tag;
    loop {
        let Some(after_filter) = strip_anki_template_field_filter(field_name) else {
            break;
        };
        field_name = after_filter.trim();
    }

    let field_name = field_name.trim();
    if is_anki_special_template_field(field_name) {
        return None;
    }

    Some(field_name)
}

fn strip_anki_template_field_filter(tag: &str) -> Option<&str> {
    [
        "cloze:",
        "hint:",
        "type:",
        "nc:",
        "text:",
        "furigana:",
        "kana:",
        "kanji:",
    ]
    .iter()
    .find_map(|prefix| tag.strip_prefix(prefix))
}

fn is_anki_special_template_field(field_name: &str) -> bool {
    matches!(
        field_name,
        "FrontSide" | "Tags" | "Type" | "Deck" | "Subdeck" | "Card" | "CardFlag" | "CardID"
    )
}

fn map_v11_note(note: &AnkiV11Note, note_type: &AnkiV11NoteType, deck_id: &str) -> Note {
    let fields = note_type
        .fields
        .iter()
        .enumerate()
        .map(|(index, field)| NoteFieldValue {
            field_id: field_id(note_type.id, field.ordinal),
            value: note.field_values.get(index).cloned().unwrap_or_default(),
        })
        .collect();

    Note {
        id: note.id.to_string(),
        note_type_id: note.note_type_id.to_string(),
        deck_id: deck_id.to_string(),
        fields,
        tags: note.tags.clone(),
        created_at: anki_seconds_to_millis(note.modified_at),
        updated_at: anki_seconds_to_millis(note.modified_at),
    }
}

fn map_v11_card(
    card: &AnkiV11Card,
    notes_by_id: &HashMap<String, Note>,
    note_types_by_id: &HashMap<String, NoteType>,
    anki_note_types_by_id: &HashMap<i64, &AnkiV11NoteType>,
    deck_names_by_id: &HashMap<i64, String>,
) -> Result<Card, ApkgError> {
    let note = notes_by_id.get(&card.note_id.to_string()).ok_or_else(|| {
        apkg_error(format!(
            "Anki card {} references missing note {}",
            card.id, card.note_id
        ))
    })?;
    let note_type = note_types_by_id.get(&note.note_type_id).ok_or_else(|| {
        apkg_error(format!(
            "Anki card {} references missing note type {}",
            card.id, note.note_type_id
        ))
    })?;
    let anki_note_type = anki_note_types_by_id
        .get(&card_note_type_id(note))
        .ok_or_else(|| {
            apkg_error(format!(
                "Anki card {} references missing raw note type {}",
                card.id, note.note_type_id
            ))
        })?;
    let template = template_for_v11_card(card, note_type, anki_note_type)?;
    let mut field_values = field_value_map(note, anki_note_type);
    insert_anki_special_template_values(
        &mut field_values,
        note,
        anki_note_type,
        template,
        card,
        deck_names_by_id,
    );
    let cloze_ordinal = if anki_note_type.kind == 1 {
        Some(i64_to_u32(card.ordinal.saturating_add(1)))
    } else {
        None
    };
    let (front, back) = if let Some(cloze_ordinal) = cloze_ordinal {
        let front = render_cloze_template(
            &template.front_template,
            &field_values,
            cloze_ordinal,
            ClozeRenderSide::Question,
        );
        let back = render_cloze_template_with_front_side(
            &template.back_template,
            &field_values,
            cloze_ordinal,
            ClozeRenderSide::Answer,
            &front,
        );
        (front, back)
    } else {
        let front = render_template(&template.front_template, &field_values);
        let back = render_template_with_front_side(&template.back_template, &field_values, &front);
        (front, back)
    };

    Ok(Card {
        id: card.id.to_string(),
        deck_id: card.deck_id.to_string(),
        front,
        back,
        created_at: anki_seconds_to_millis(card.modified_at),
        lineage: Some(CardLineage {
            note_id: note.id.clone(),
            note_type_id: note.note_type_id.clone(),
            template_id: template.id.clone(),
            ordinal: i64_to_u32(card.ordinal),
            cloze_ordinal,
        }),
    })
}

fn template_for_v11_card<'a>(
    card: &AnkiV11Card,
    note_type: &'a NoteType,
    anki_note_type: &AnkiV11NoteType,
) -> Result<&'a CardTemplate, ApkgError> {
    if anki_note_type.kind == 1 {
        return note_type
            .templates
            .iter()
            .find(|template| template.ordinal == 0)
            .or_else(|| {
                note_type.templates.iter().find(|template| {
                    template_references_cloze(&template.front_template, &template.back_template)
                })
            })
            .or_else(|| note_type.templates.first())
            .ok_or_else(|| {
                apkg_error(format!(
                    "Anki cloze card {} references note type {} without templates",
                    card.id, note_type.id
                ))
            });
    }

    note_type
        .templates
        .iter()
        .find(|template| template.ordinal == i64_to_u32(card.ordinal))
        .ok_or_else(|| {
            apkg_error(format!(
                "Anki card {} references missing template ordinal {}",
                card.id, card.ordinal
            ))
        })
}

fn field_value_map(note: &Note, note_type: &AnkiV11NoteType) -> HashMap<String, String> {
    let values_by_field_id: HashMap<&str, &str> = note
        .fields
        .iter()
        .map(|field| (field.field_id.as_str(), field.value.as_str()))
        .collect();
    note_type
        .fields
        .iter()
        .map(|field| {
            let id = field_id(note_type.id, field.ordinal);
            (
                field.name.clone(),
                values_by_field_id
                    .get(id.as_str())
                    .copied()
                    .unwrap_or_default()
                    .to_string(),
            )
        })
        .collect()
}

fn insert_anki_special_template_values(
    field_values: &mut HashMap<String, String>,
    note: &Note,
    note_type: &AnkiV11NoteType,
    template: &CardTemplate,
    card: &AnkiV11Card,
    deck_names_by_id: &HashMap<i64, String>,
) {
    let render_deck_id = if card.original_deck_id != 0 {
        card.original_deck_id
    } else {
        card.deck_id
    };
    let deck_name = deck_names_by_id
        .get(&render_deck_id)
        .cloned()
        .unwrap_or_else(|| render_deck_id.to_string());
    field_values
        .entry("Tags".to_string())
        .or_insert_with(|| note.tags.join(" "));
    field_values
        .entry("Type".to_string())
        .or_insert_with(|| note_type.name.clone());
    field_values
        .entry("Deck".to_string())
        .or_insert_with(|| deck_name.clone());
    field_values
        .entry("Subdeck".to_string())
        .or_insert_with(|| {
            deck_name
                .rsplit_once("::")
                .map_or(deck_name.as_str(), |(_, subdeck)| subdeck)
                .to_string()
        });
    field_values
        .entry("Card".to_string())
        .or_insert_with(|| template.name.clone());
    field_values
        .entry("CardFlag".to_string())
        .or_insert_with(|| anki_card_flag_template_value(card.flags));
    field_values
        .entry("CardID".to_string())
        .or_insert_with(|| card.id.to_string());
}

fn anki_card_flag_template_value(flags: i64) -> String {
    format!("flag{}", flags & 0b111)
}

fn card_note_type_id(note: &Note) -> i64 {
    note.note_type_id.parse().unwrap_or_default()
}

fn map_v11_card_progress(
    card: &AnkiV11Card,
    collection_created_at_days: i64,
    marked_at_by_note_id: &BTreeMap<i64, u64>,
    last_reviewed_at_by_card: &BTreeMap<i64, u64>,
    deck_options: Option<&DeckOptions>,
) -> Option<CardProgress> {
    let flag = anki_card_flag(card.flags);
    let marked_at = marked_at_by_note_id.get(&card.note_id).copied();
    if card.queue == 0 {
        return (flag.is_some() || marked_at.is_some())
            .then(|| new_card_metadata_overlay(card, flag, marked_at));
    }

    let state = match card.queue {
        -3 | -2 => CardState::Buried,
        -1 => CardState::Suspended,
        _ => match card.kind {
            1 => CardState::Learning,
            3 => CardState::Relearning,
            _ => CardState::Review,
        },
    };
    let next_due_at = if card.queue == 1 {
        anki_seconds_to_millis(card.due)
    } else {
        anki_due_day_to_millis(collection_created_at_days, card.due)
    };
    let last_seen_at = last_reviewed_at_by_card
        .get(&card.id)
        .copied()
        .unwrap_or_else(|| anki_seconds_to_millis(card.modified_at));
    let card_data = serde_json::from_str::<Value>(&card.data).ok();
    let fsrs_stability = card_data
        .as_ref()
        .and_then(|data| json_path_f64(data, &["s"]))
        .filter(|value| value.is_finite() && *value > 0.0);
    let fsrs_difficulty = card_data
        .as_ref()
        .and_then(|data| json_path_f64(data, &["d"]))
        .filter(|value| value.is_finite());

    Some(CardProgress {
        card_id: card.id.to_string(),
        state,
        interval: i64_to_u32(card.interval),
        ease_factor: if card.factor > 0 {
            card.factor as f64 / 1000.0
        } else {
            INITIAL_EASE_FACTOR
        },
        next_due_at,
        learning_step_index: anki_left_to_learning_step_index(
            card.left,
            learning_step_count_for_state(state, deck_options),
        ),
        buried_until: (state == CardState::Buried).then_some(next_due_at),
        suspended_at: (state == CardState::Suspended)
            .then_some(anki_seconds_to_millis(card.modified_at)),
        times_seen: i64_to_u32(card.repetitions),
        times_correct: i64_to_u32(card.repetitions.saturating_sub(card.lapses)),
        times_incorrect: i64_to_u32(card.lapses),
        last_seen_at,
        fsrs_stability,
        fsrs_difficulty,
        flag,
        marked_at,
    })
}

fn anki_left_to_learning_step_index(left: i64, step_count: usize) -> Option<u32> {
    if step_count == 0 {
        return None;
    }
    let remaining = left.rem_euclid(1000) as usize;
    if remaining == 0 {
        return None;
    }
    let index = step_count
        .saturating_sub(remaining)
        .min(step_count.saturating_sub(1));
    Some(index as u32)
}

fn new_card_metadata_overlay(
    card: &AnkiV11Card,
    flag: Option<CardFlag>,
    marked_at: Option<u64>,
) -> CardProgress {
    let timestamp = anki_seconds_to_millis(card.modified_at);
    CardProgress {
        card_id: card.id.to_string(),
        state: CardState::Review,
        interval: 0,
        ease_factor: INITIAL_EASE_FACTOR,
        next_due_at: timestamp,
        learning_step_index: None,
        buried_until: None,
        suspended_at: None,
        times_seen: 0,
        times_correct: 0,
        times_incorrect: 0,
        last_seen_at: timestamp,
        fsrs_stability: None,
        fsrs_difficulty: None,
        flag,
        marked_at,
    }
}

fn anki_marked_at_for_note(note: &AnkiV11Note) -> Option<u64> {
    note.tags
        .iter()
        .any(|tag| tag.eq_ignore_ascii_case(ANKI_MARKED_TAG))
        .then_some(anki_seconds_to_millis(note.modified_at))
}

fn map_v11_review(review: &AnkiV11Review, deck_id: &str) -> Review {
    Review {
        id: review.id.to_string(),
        session_id: import_session_id(deck_id),
        card_id: review.card_id.to_string(),
        rating: rating_from_v11_ease(review.ease),
        reviewed_at: i64_to_u64(review.id),
        answer_time_ms: (review.time > 0).then(|| i64_to_u32(review.time)),
        leech_event: None,
        previous_progress: Some(v11_review_progress_snapshot(
            review,
            review.last_interval,
            false,
        )),
        resulting_progress: Some(v11_review_progress_snapshot(review, review.interval, true)),
        previous_active_session: None,
        sibling_progress_snapshots: Vec::new(),
    }
}

fn v11_review_progress_snapshot(
    review: &AnkiV11Review,
    interval: i64,
    after_review: bool,
) -> CardProgress {
    let state = match review.kind {
        0 => CardState::Learning,
        2 => CardState::Relearning,
        _ => CardState::Review,
    };
    let timestamp = i64_to_u64(review.id);
    let incorrect = u32::from(after_review && review.ease == 1);
    CardProgress {
        card_id: review.card_id.to_string(),
        state,
        interval: i64_to_u32(interval),
        ease_factor: if review.factor > 0 {
            review.factor as f64 / 1000.0
        } else {
            INITIAL_EASE_FACTOR
        },
        next_due_at: timestamp,
        learning_step_index: None,
        buried_until: None,
        suspended_at: None,
        times_seen: u32::from(after_review),
        times_correct: u32::from(after_review && review.ease != 1),
        times_incorrect: incorrect,
        last_seen_at: timestamp,
        fsrs_stability: None,
        fsrs_difficulty: None,
        flag: None,
        marked_at: None,
    }
}

#[derive(Default)]
struct SessionAccumulator {
    deck_id: String,
    started_at: u64,
    ended_at: u64,
    cards_reviewed: u32,
    cards_correct: u32,
}

fn synthetic_import_sessions(
    reviews: &[Review],
    deck_by_card_id: &HashMap<i64, String>,
    default_deck_id: &str,
) -> Vec<Session> {
    let mut accumulators: BTreeMap<String, SessionAccumulator> = BTreeMap::new();
    for review in reviews {
        let card_id = review.card_id.parse::<i64>().unwrap_or_default();
        let deck_id = deck_by_card_id
            .get(&card_id)
            .map(String::as_str)
            .unwrap_or(default_deck_id);
        let id = import_session_id(deck_id);
        let entry = accumulators
            .entry(id)
            .or_insert_with(|| SessionAccumulator {
                deck_id: deck_id.to_string(),
                started_at: review.reviewed_at,
                ended_at: review.reviewed_at,
                cards_reviewed: 0,
                cards_correct: 0,
            });
        entry.started_at = entry.started_at.min(review.reviewed_at);
        entry.ended_at = entry.ended_at.max(review.reviewed_at);
        entry.cards_reviewed = entry.cards_reviewed.saturating_add(1);
        if review.rating != Rating::Again {
            entry.cards_correct = entry.cards_correct.saturating_add(1);
        }
    }

    accumulators
        .into_iter()
        .map(|(id, session)| Session {
            id,
            deck_id: session.deck_id,
            status: SessionStatus::Completed,
            started_at: session.started_at,
            ended_at: Some(session.ended_at),
            cards_reviewed: session.cards_reviewed,
            cards_correct: session.cards_correct,
        })
        .collect()
}

fn last_reviewed_at_by_card(reviews: &[AnkiV11Review]) -> BTreeMap<i64, u64> {
    let mut last_reviewed: BTreeMap<i64, u64> = BTreeMap::new();
    for review in reviews {
        last_reviewed
            .entry(review.card_id)
            .and_modify(|reviewed_at| *reviewed_at = (*reviewed_at).max(i64_to_u64(review.id)))
            .or_insert_with(|| i64_to_u64(review.id));
    }
    last_reviewed
}

fn rating_from_v11_ease(ease: i64) -> Rating {
    match ease {
        1 => Rating::Again,
        2 => Rating::Hard,
        4 => Rating::Easy,
        _ => Rating::Good,
    }
}

fn anki_card_flag(flags: i64) -> Option<CardFlag> {
    match flags & 0b111 {
        1 => Some(CardFlag::Red),
        2 => Some(CardFlag::Orange),
        3 => Some(CardFlag::Green),
        4 => Some(CardFlag::Blue),
        5 => Some(CardFlag::Pink),
        6 => Some(CardFlag::Turquoise),
        7 => Some(CardFlag::Purple),
        _ => None,
    }
}

fn field_id(note_type_id: i64, ordinal: i64) -> String {
    format!("{note_type_id}:field:{ordinal}")
}

fn template_id(note_type_id: i64, ordinal: i64) -> String {
    format!("{note_type_id}:template:{ordinal}")
}

fn import_session_id(deck_id: &str) -> String {
    format!("anki-import:{deck_id}")
}

fn anki_seconds_to_millis(seconds: i64) -> u64 {
    i64_to_u64(seconds).saturating_mul(1000)
}

fn anki_days_to_millis(days: i64) -> u64 {
    i64_to_u64(days).saturating_mul(ONE_DAY_MS)
}

fn anki_due_day_to_millis(collection_created_at_days: i64, due_day: i64) -> u64 {
    i64_to_u64(collection_created_at_days.saturating_add(due_day)).saturating_mul(ONE_DAY_MS)
}

fn i64_to_u32(value: i64) -> u32 {
    value.clamp(0, u32::MAX as i64) as u32
}

fn i64_to_u64(value: i64) -> u64 {
    value.max(0) as u64
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

fn read_v11_col_row(bytes: &[u8]) -> Result<RawV11ColRow, ApkgError> {
    let mut rows = read_v11_table(bytes, "col", "Anki V11 col table")?;
    let (rowid, columns) = rows
        .drain(..)
        .next()
        .ok_or_else(|| apkg_error("Anki V11 col table is empty"))?;
    require_sqlite_column_count("Anki V11 col row", &columns, 13)?;
    Ok(RawV11ColRow {
        id: rowid,
        created_at_days: sqlite_i64(&columns[1], "col.crt")?,
        modified_at: sqlite_i64(&columns[2], "col.mod")?,
        schema_modified_at: sqlite_i64(&columns[3], "col.scm")?,
        version: sqlite_i64(&columns[4], "col.ver")?,
        dirty: sqlite_i64(&columns[5], "col.dty")?,
        update_sequence_number: sqlite_i64(&columns[6], "col.usn")?,
        last_sync: sqlite_i64(&columns[7], "col.ls")?,
        config_json: sqlite_text(&columns[8], "col.conf")?,
        models_json: sqlite_text(&columns[9], "col.models")?,
        decks_json: sqlite_text(&columns[10], "col.decks")?,
        deck_config_json: sqlite_text(&columns[11], "col.dconf")?,
        tags_json: sqlite_text(&columns[12], "col.tags")?,
    })
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

fn read_v11_notes(bytes: &[u8]) -> Result<Vec<AnkiV11Note>, ApkgError> {
    read_v11_table(bytes, "notes", "Anki V11 notes")?
        .into_iter()
        .map(|(rowid, columns)| {
            require_sqlite_column_count("Anki V11 notes row", &columns, 11)?;
            let tags = sqlite_text(&columns[5], "notes.tags")?;
            let fields = sqlite_text(&columns[6], "notes.flds")?;
            Ok(AnkiV11Note {
                id: rowid,
                guid: sqlite_text(&columns[1], "notes.guid")?,
                note_type_id: sqlite_i64(&columns[2], "notes.mid")?,
                modified_at: sqlite_i64(&columns[3], "notes.mod")?,
                update_sequence_number: sqlite_i64(&columns[4], "notes.usn")?,
                tags: split_anki_tags(&tags),
                field_values: split_anki_fields(&fields),
                sort_field: sqlite_value_to_string(&columns[7]),
                checksum: sqlite_i64(&columns[8], "notes.csum")?,
                flags: sqlite_i64(&columns[9], "notes.flags")?,
                data: sqlite_text(&columns[10], "notes.data")?,
            })
        })
        .collect()
}

fn read_v11_cards(bytes: &[u8]) -> Result<Vec<AnkiV11Card>, ApkgError> {
    read_v11_table(bytes, "cards", "Anki V11 cards")?
        .into_iter()
        .map(|(rowid, columns)| {
            require_sqlite_column_count("Anki V11 cards row", &columns, 18)?;
            Ok(AnkiV11Card {
                id: rowid,
                note_id: sqlite_i64(&columns[1], "cards.nid")?,
                deck_id: sqlite_i64(&columns[2], "cards.did")?,
                ordinal: sqlite_i64(&columns[3], "cards.ord")?,
                modified_at: sqlite_i64(&columns[4], "cards.mod")?,
                update_sequence_number: sqlite_i64(&columns[5], "cards.usn")?,
                kind: sqlite_i64(&columns[6], "cards.type")?,
                queue: sqlite_i64(&columns[7], "cards.queue")?,
                due: sqlite_i64(&columns[8], "cards.due")?,
                interval: sqlite_i64(&columns[9], "cards.ivl")?,
                factor: sqlite_i64(&columns[10], "cards.factor")?,
                repetitions: sqlite_i64(&columns[11], "cards.reps")?,
                lapses: sqlite_i64(&columns[12], "cards.lapses")?,
                left: sqlite_i64(&columns[13], "cards.left")?,
                original_due: sqlite_i64(&columns[14], "cards.odue")?,
                original_deck_id: sqlite_i64(&columns[15], "cards.odid")?,
                flags: sqlite_i64(&columns[16], "cards.flags")?,
                data: sqlite_text(&columns[17], "cards.data")?,
            })
        })
        .collect()
}

fn read_v11_reviews(bytes: &[u8]) -> Result<Vec<AnkiV11Review>, ApkgError> {
    read_v11_table(bytes, "revlog", "Anki V11 revlog")?
        .into_iter()
        .map(|(rowid, columns)| {
            require_sqlite_column_count("Anki V11 revlog row", &columns, 9)?;
            Ok(AnkiV11Review {
                id: rowid,
                card_id: sqlite_i64(&columns[1], "revlog.cid")?,
                update_sequence_number: sqlite_i64(&columns[2], "revlog.usn")?,
                ease: sqlite_i64(&columns[3], "revlog.ease")?,
                interval: sqlite_i64(&columns[4], "revlog.ivl")?,
                last_interval: sqlite_i64(&columns[5], "revlog.lastIvl")?,
                factor: sqlite_i64(&columns[6], "revlog.factor")?,
                time: sqlite_i64(&columns[7], "revlog.time")?,
                kind: sqlite_i64(&columns[8], "revlog.type")?,
            })
        })
        .collect()
}

fn read_v11_graves(bytes: &[u8]) -> Result<Vec<AnkiV11Grave>, ApkgError> {
    let mut graves = read_v11_table(bytes, "graves", "Anki V11 graves")?
        .into_iter()
        .map(|(_rowid, columns)| {
            require_sqlite_column_count("Anki V11 graves row", &columns, 3)?;
            Ok(AnkiV11Grave {
                update_sequence_number: sqlite_i64(&columns[0], "graves.usn")?,
                object_id: sqlite_i64(&columns[1], "graves.oid")?,
                kind: sqlite_i64(&columns[2], "graves.type")?,
            })
        })
        .collect::<Result<Vec<_>, ApkgError>>()?;
    graves.sort_by_key(|grave| grave.object_id);
    Ok(graves)
}

fn read_v11_table(
    bytes: &[u8],
    table: &str,
    context: &str,
) -> Result<Vec<(i64, Vec<SqlValue>)>, ApkgError> {
    sqlite_file::read_table(bytes, table)
        .map_err(|err| apkg_error(format!("failed to read {context}: {err}")))
}

fn require_sqlite_column_count(
    context: &str,
    columns: &[SqlValue],
    expected: usize,
) -> Result<(), ApkgError> {
    if columns.len() == expected {
        Ok(())
    } else {
        Err(apkg_error(format!(
            "{context} has {} columns; expected {expected}",
            columns.len()
        )))
    }
}

fn sqlite_i64(value: &SqlValue, field: &str) -> Result<i64, ApkgError> {
    match value {
        SqlValue::Int(value) => Ok(*value),
        other => Err(apkg_error(format!(
            "expected integer for Anki V11 {field}, got {other:?}"
        ))),
    }
}

fn sqlite_text(value: &SqlValue, field: &str) -> Result<String, ApkgError> {
    match value {
        SqlValue::Text(value) => Ok(value.clone()),
        other => Err(apkg_error(format!(
            "expected text for Anki V11 {field}, got {other:?}"
        ))),
    }
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

fn json_path<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current)
}

fn json_path_u32(value: &Value, path: &[&str]) -> Option<u32> {
    json_path(value, path).and_then(value_to_u32)
}

fn json_path_i64(value: &Value, path: &[&str]) -> Option<i64> {
    json_path(value, path).and_then(Value::as_i64)
}

fn json_path_f64(value: &Value, path: &[&str]) -> Option<f64> {
    json_path(value, path).and_then(Value::as_f64)
}

fn json_path_string(value: &Value, path: &[&str]) -> Option<String> {
    json_path(value, path)
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn json_path_bool(value: &Value, path: &[&str]) -> Option<bool> {
    json_path(value, path).and_then(value_to_bool)
}

fn json_path_f64_array(value: &Value, path: &[&str]) -> Option<Vec<f64>> {
    json_path(value, path).and_then(|value| {
        value.as_array().map(|values| {
            values
                .iter()
                .filter_map(Value::as_f64)
                .filter(|value| value.is_finite())
                .collect()
        })
    })
}

fn json_path_u32_array(value: &Value, path: &[&str]) -> Option<Vec<u32>> {
    json_path(value, path).and_then(|value| {
        value
            .as_array()
            .map(|values| values.iter().filter_map(value_to_u32).collect())
    })
}

fn json_path_minutes(value: &Value, path: &[&str]) -> Option<Vec<u32>> {
    json_path(value, path).and_then(|value| {
        value.as_array().map(|values| {
            values
                .iter()
                .filter_map(Value::as_f64)
                .map(|minutes| minutes.max(0.0).round().clamp(0.0, u32::MAX as f64) as u32)
                .collect()
        })
    })
}

fn value_to_u32(value: &Value) -> Option<u32> {
    value
        .as_u64()
        .and_then(|value| u32::try_from(value).ok())
        .or_else(|| {
            value
                .as_i64()
                .and_then(|value| u32::try_from(value.max(0)).ok())
        })
        .or_else(|| {
            value
                .as_f64()
                .map(|value| value.max(0.0).round().clamp(0.0, u32::MAX as f64) as u32)
        })
}

fn value_to_bool(value: &Value) -> Option<bool> {
    value
        .as_bool()
        .or_else(|| value.as_i64().map(|value| value != 0))
        .or_else(
            || match value.as_str()?.trim().to_ascii_lowercase().as_str() {
                "true" | "1" | "yes" => Some(true),
                "false" | "0" | "no" => Some(false),
                _ => None,
            },
        )
}

fn split_anki_fields(fields: &str) -> Vec<String> {
    fields.split('\u{1f}').map(str::to_string).collect()
}

fn split_anki_tags(tags: &str) -> Vec<String> {
    tags.split_whitespace().map(str::to_string).collect()
}

fn sqlite_value_to_string(value: &SqlValue) -> String {
    match value {
        SqlValue::Null => String::new(),
        SqlValue::Int(value) => value.to_string(),
        SqlValue::Real(value) => value.to_string(),
        SqlValue::Text(value) => value.clone(),
        SqlValue::Blob(value) => String::from_utf8_lossy(value).into_owned(),
    }
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

fn collection_member(
    reader: &ZipReader<'_>,
    entries: &[ArchiveEntry],
) -> Result<CollectionMember, ApkgError> {
    if let Some(format) = package_collection_format(reader)? {
        return collection_member_for_format(entries, format);
    }

    let candidates = [
        CollectionFormat::Sqlite21b,
        CollectionFormat::Sqlite21,
        CollectionFormat::LegacySqlite,
    ];

    for format in candidates {
        let name = format.collection_name();
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

fn package_collection_format(
    reader: &ZipReader<'_>,
) -> Result<Option<CollectionFormat>, ApkgError> {
    let Ok(bytes) = reader.read_by_name(META) else {
        return Ok(None);
    };
    let metadata = PackageMetadataProto::decode_pb(bytes.as_slice())
        .map_err(|err| apkg_error(format!("invalid Anki package metadata: {err}")))?;
    let version = PackageVersionProto::parse_i32(metadata.version).ok_or_else(|| {
        apkg_error(format!(
            "unsupported Anki package version {}",
            metadata.version
        ))
    })?;
    let format = match version {
        PackageVersionProto::Unknown => {
            return Err(apkg_error("unsupported Anki package version 0"));
        }
        PackageVersionProto::Legacy1 => CollectionFormat::LegacySqlite,
        PackageVersionProto::Legacy2 => CollectionFormat::Sqlite21,
        PackageVersionProto::Latest => CollectionFormat::Sqlite21b,
    };
    Ok(Some(format))
}

fn collection_member_for_format(
    entries: &[ArchiveEntry],
    format: CollectionFormat,
) -> Result<CollectionMember, ApkgError> {
    let name = format.collection_name();
    let entry = entries
        .iter()
        .find(|entry| entry.name == name)
        .ok_or_else(|| apkg_error(format!("Anki package is missing {name}")))?;
    Ok(CollectionMember {
        name: entry.name.clone(),
        format,
        size: entry.size,
        compressed_size: entry.compressed_size,
        compression_method: entry.compression_method,
    })
}

fn decode_package_payload(
    format: CollectionFormat,
    label: &str,
    bytes: &[u8],
) -> Result<Vec<u8>, ApkgError> {
    if format.is_modern() {
        zstd_crate::stream::decode_all(Cursor::new(bytes))
            .map_err(|err| apkg_error(format!("failed to decode zstd-compressed {label}: {err}")))
    } else {
        Ok(bytes.to_vec())
    }
}

fn encode_package_payload(label: &str, bytes: &[u8]) -> Result<Vec<u8>, ApkgError> {
    zstd_crate::stream::encode_all(Cursor::new(bytes), 0)
        .map_err(|err| apkg_error(format!("failed to encode zstd-compressed {label}: {err}")))
}

fn media_manifest(
    reader: &ZipReader<'_>,
    format: CollectionFormat,
) -> Result<MediaManifest, ApkgError> {
    if format.is_modern() {
        return modern_media_manifest(reader);
    }

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
            sha1: None,
            legacy_zip_filename: None,
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

fn modern_media_manifest(reader: &ZipReader<'_>) -> Result<MediaManifest, ApkgError> {
    let mut manifest = MediaManifest::default();
    let media_map = match reader.read_by_name(MEDIA_MAP) {
        Ok(bytes) => bytes,
        Err(_) => return Ok(manifest),
    };
    manifest.map_present = true;
    let media_map = decode_package_payload(CollectionFormat::Sqlite21b, "media map", &media_map)?;
    let entries = MediaEntriesProto::decode_pb(media_map.as_slice())
        .map_err(|err| apkg_error(format!("invalid Anki media entries protobuf: {err}")))?;

    let archive_entries: BTreeMap<&str, u32> = reader
        .entries()
        .iter()
        .filter(|entry| !entry.is_directory && !is_reserved_entry(&entry.name))
        .map(|entry| (entry.name.as_str(), entry.compressed_size))
        .collect();
    let mut mapped_payload_archive_names = BTreeSet::new();

    for (index, entry) in entries.entries.into_iter().enumerate() {
        let archive_name = index.to_string();
        let payload_archive_name = entry
            .legacy_zip_filename
            .map(|legacy| legacy.to_string())
            .unwrap_or_else(|| archive_name.clone());
        manifest
            .mapping
            .insert(archive_name.clone(), entry.name.clone());
        mapped_payload_archive_names.insert(payload_archive_name.clone());
        if let Some(archive_entry) = archive_entries.get(payload_archive_name.as_str()) {
            manifest.media_files.push(MediaFile {
                archive_name,
                filename: Some(entry.name),
                size: entry.size,
                compressed_size: *archive_entry,
                sha1: (!entry.sha1.is_empty()).then(|| bytes_to_lower_hex(&entry.sha1)),
                legacy_zip_filename: entry.legacy_zip_filename,
            });
        } else {
            manifest.missing_files.push(archive_name);
        }
    }

    for archive_name in archive_entries.keys() {
        if !mapped_payload_archive_names.contains(*archive_name) {
            manifest.unmapped_files.push((*archive_name).to_string());
        }
    }

    Ok(manifest)
}

fn is_reserved_entry(name: &str) -> bool {
    matches!(
        name,
        LEGACY_COLLECTION | SQLITE_21_COLLECTION | SQLITE_21B_COLLECTION | MEDIA_MAP | META
    )
}

fn bytes_to_lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
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

    fn zstd_encode(data: &[u8]) -> Vec<u8> {
        zstd_crate::stream::encode_all(Cursor::new(data), 0).unwrap()
    }

    fn modern_package(collection: &[u8], media_assets: &[MediaAsset<'_>]) -> Vec<u8> {
        let mut writer = ZipWriter::new();

        let meta = PackageMetadataProto {
            version: PackageVersionProto::Latest as i32,
        }
        .encode_pb();
        writer.add_file(META, &meta, false);
        writer.add_file(SQLITE_21B_COLLECTION, &zstd_encode(collection), false);
        writer.add_file(LEGACY_COLLECTION, b"dummy legacy collection", false);

        let media_entries = MediaEntriesProto {
            entries: media_assets
                .iter()
                .enumerate()
                .map(|(index, asset)| MediaEntryProto {
                    name: asset.filename.to_string(),
                    size: asset.data.len() as u32,
                    sha1: sum1(asset.data).to_vec(),
                    legacy_zip_filename: Some(index as u32),
                })
                .collect(),
        };
        let media_map = media_entries.encode_pb();
        writer.add_file(MEDIA_MAP, &zstd_encode(&media_map), false);

        for (index, asset) in media_assets.iter().enumerate() {
            writer.add_file(&index.to_string(), &zstd_encode(asset.data), false);
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
  sfld integer not null,
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
                        r#"{
                            "1": {
                                "id": 1,
                                "name": "Default",
                                "new": {"perDay": 12, "bury": false, "delays": [3, 12], "ints": [2, 5], "initialFactor": 2800},
                                "rev": {"perDay": 80, "bury": false, "maxIvl": 90, "ivlFct": 0.75, "hardFactor": 1.4, "ease4": 1.6},
                                "lapse": {"delays": [20], "mult": 0.5, "leechFails": 6, "leechAction": 0},
                                "buryInterdayLearning": false
                            }
                        }"#,
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
                        4_i64,
                        r#"{"s":6.25,"d":7.3}"#
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

    fn golden_v11_apkg_fixture_bytes() -> Vec<u8> {
        let sqlite = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(sqlite.path(), v11_sqlite_collection_bytes()).unwrap();
        {
            let connection = Connection::open(sqlite.path()).unwrap();
            let decks = r#"{
  "1": {"id": 1, "name": "Default", "desc": "Root deck"},
  "2": {"id": 2, "name": "Spanish::Latin", "desc": "Story deck", "conf": 1},
  "3": {"id": 3, "name": "Filtered::Today", "desc": "Filtered cram deck", "dyn": 1, "conf": 1, "terms": [["deck:Spanish", 10, 0]], "resched": true}
}"#;
            connection
                .execute("UPDATE col SET decks = ?1", rusqlite::params![decks])
                .unwrap();
            connection
                .execute(
                    "UPDATE notes SET flds = ?1, tags = ?2, data = ?3 WHERE id = 1000",
                    rusqlite::params![
                        "hola [sound:audio/hola.mp3]\u{1f}hello <img src=\"images/card.png\">",
                        " spanish media filtered ",
                        "golden-note"
                    ],
                )
                .unwrap();
            connection
                .execute(
                    "UPDATE cards SET did = ?1, odue = ?2, odid = ?3, data = ?4 WHERE id = 2000",
                    rusqlite::params![3_i64, 42_i64, 2_i64, "filtered-card"],
                )
                .unwrap();
        }

        let collection = std::fs::read(sqlite.path()).unwrap();
        write_legacy_apkg(
            &collection,
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
        )
    }

    fn checked_in_golden_v11_apkg_fixture_bytes() -> &'static [u8] {
        include_bytes!("../tests/fixtures/golden-v11-filtered-media.apkg")
    }

    #[test]
    #[ignore]
    fn regenerate_checked_in_golden_v11_apkg_fixture() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("golden-v11-filtered-media.apkg");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, golden_v11_apkg_fixture_bytes()).unwrap();
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
        assert_eq!(manifest.media.media_files[0].sha1, None);
        assert_eq!(manifest.media.media_files[0].legacy_zip_filename, None);
        assert_eq!(manifest.media.missing_files, vec!["3"]);
        assert_eq!(manifest.media.unmapped_files, vec!["2"]);
    }

    #[test]
    fn recognizes_modern_collection_members() {
        let apkg = modern_package(b"modern collection", &[]);

        let manifest = inspect_apkg(&apkg).unwrap();
        let collection = read_collection_bytes(&apkg).unwrap();

        assert_eq!(manifest.collection.name, SQLITE_21B_COLLECTION);
        assert_eq!(manifest.collection.format, CollectionFormat::Sqlite21b);
        assert_eq!(collection, b"modern collection");
    }

    #[test]
    fn reads_collection_members_across_legacy_and_modern_packages() {
        let legacy = package(&[(LEGACY_COLLECTION, b"legacy collection")]);
        let sqlite21 = package(&[(SQLITE_21_COLLECTION, b"v11 collection")]);
        let modern = modern_package(b"modern collection", &[]);

        assert_eq!(
            read_v11_collection_bytes(&legacy).unwrap(),
            b"legacy collection"
        );
        assert_eq!(
            read_v11_collection_bytes(&sqlite21).unwrap(),
            b"v11 collection"
        );
        assert_eq!(
            read_v11_collection_bytes(&modern).unwrap(),
            b"modern collection"
        );
    }

    #[test]
    fn parses_v11_sqlite_collection_tables() {
        let collection = parse_v11_collection_bytes(&v11_sqlite_collection_bytes()).unwrap();

        assert_eq!(collection.metadata.version, 11);
        assert_eq!(collection.metadata.config["nextPos"], 1);
        assert_eq!(collection.metadata.deck_config["1"]["new"]["perDay"], 12);
        assert_eq!(collection.metadata.deck_config["1"]["rev"]["perDay"], 80);
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
        assert_eq!(collection.reviews[0].time, 12_000);

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
    fn maps_v11_collection_into_engram_app_state() {
        let collection = parse_v11_collection_bytes(&v11_sqlite_collection_bytes()).unwrap();
        let state = v11_collection_to_engram_state(&collection).unwrap();

        assert_eq!(state.decks.len(), 2);
        assert_eq!(state.decks[1].id, "2");
        assert_eq!(state.decks[1].name, "Spanish::Latin");
        assert_eq!(state.deck_options.len(), 2);
        let options = &state.deck_options[1].options;
        assert_eq!(options.new_cards_per_day, 12);
        assert_eq!(options.reviews_per_day, 80);
        assert_eq!(options.learning_steps_minutes, vec![3, 12]);
        assert_eq!(options.relearning_steps_minutes, vec![20]);
        assert_eq!(options.graduating_interval_days, 2);
        assert_eq!(options.easy_interval_days, 5);
        assert_eq!(options.initial_ease_factor, 2.8);
        assert_eq!(options.maximum_interval_days, 90);
        assert_eq!(options.review_interval_modifier, 0.75);
        assert_eq!(options.hard_interval_multiplier, 1.4);
        assert_eq!(options.easy_bonus_multiplier, 1.6);
        assert_eq!(options.lapse_interval_multiplier, 0.5);
        assert_eq!(options.leech_threshold, 6);
        assert_eq!(options.leech_action, LeechAction::Suspend);
        assert!(!options.bury_new_siblings);
        assert!(!options.bury_review_siblings);
        assert!(!options.bury_interday_learning_siblings);
        let deck_source = state
            .external_sources
            .iter()
            .find(|source| source.target == ExternalSourceTarget::Deck && source.target_id == "2")
            .unwrap();
        assert_eq!(
            deck_source.data.get("configId").map(String::as_str),
            Some("1")
        );
        assert_eq!(
            deck_source.data.get("configName").map(String::as_str),
            Some("Default")
        );

        assert_eq!(state.note_types.len(), 1);
        let note_type = &state.note_types[0];
        assert_eq!(note_type.id, "100");
        assert_eq!(note_type.fields[0].id, "100:field:0");
        assert_eq!(note_type.fields[0].name, "Front");
        assert_eq!(note_type.templates[0].id, "100:template:0");
        assert_eq!(note_type.templates[0].deck_id.as_deref(), Some("2"));

        assert_eq!(state.notes.len(), 1);
        let note = &state.notes[0];
        assert_eq!(note.id, "1000");
        assert_eq!(note.deck_id, "2");
        assert_eq!(note.fields[0].value, "hola");
        assert_eq!(note.fields[1].value, "hello");
        assert_eq!(note.tags, vec!["spanish", "core"]);

        assert_eq!(state.cards.len(), 1);
        let card = &state.cards[0];
        assert_eq!(card.id, "2000");
        assert_eq!(card.deck_id, "2");
        assert_eq!(card.front, "hola");
        assert_eq!(card.back, "hello");
        let lineage = card.lineage.as_ref().unwrap();
        assert_eq!(lineage.note_id, "1000");
        assert_eq!(lineage.note_type_id, "100");
        assert_eq!(lineage.template_id, "100:template:0");

        assert_eq!(state.card_progress.len(), 1);
        let progress = &state.card_progress[0];
        assert_eq!(progress.card_id, "2000");
        assert_eq!(progress.state, CardState::Review);
        assert_eq!(progress.interval, 7);
        assert_eq!(progress.ease_factor, 2.5);
        assert_eq!(progress.next_due_at, (19_000 + 42) * ONE_DAY_MS);
        assert_eq!(progress.times_seen, 3);
        assert_eq!(progress.times_correct, 2);
        assert_eq!(progress.times_incorrect, 1);
        assert_eq!(progress.last_seen_at, 3000);
        assert_eq!(progress.fsrs_stability, Some(6.25));
        assert_eq!(progress.fsrs_difficulty, Some(7.3));
        assert_eq!(progress.flag, Some(CardFlag::Blue));

        assert_eq!(state.reviews.len(), 1);
        assert_eq!(state.reviews[0].id, "3000");
        assert_eq!(state.reviews[0].session_id, "anki-import:2");
        assert_eq!(state.reviews[0].rating, Rating::Good);
        assert_eq!(state.reviews[0].reviewed_at, 3000);
        assert_eq!(state.reviews[0].answer_time_ms, Some(12_000));
        let previous = state.reviews[0].previous_progress.as_ref().unwrap();
        assert_eq!(previous.card_id, "2000");
        assert_eq!(previous.interval, 3);
        assert_eq!(previous.ease_factor, 2.5);
        let resulting = state.reviews[0].resulting_progress.as_ref().unwrap();
        assert_eq!(resulting.card_id, "2000");
        assert_eq!(resulting.interval, 7);
        assert_eq!(resulting.ease_factor, 2.5);

        let exported = parse_v11_collection_bytes(
            &write_v11_collection_bytes_from_engram_state(&state).unwrap(),
        )
        .unwrap();
        assert_eq!(exported.reviews[0].interval, 7);
        assert_eq!(exported.reviews[0].last_interval, 3);
        assert_eq!(exported.reviews[0].factor, 2500);
        assert_eq!(exported.decks[1].raw["conf"], 2);
        assert_eq!(exported.metadata.deck_config["2"]["new"]["perDay"], 12);
        assert_eq!(
            exported.metadata.deck_config["2"]["new"]["delays"],
            serde_json::json!([3, 12])
        );
        assert_eq!(
            exported.metadata.deck_config["2"]["new"]["initialFactor"],
            2800
        );
        assert_eq!(exported.metadata.deck_config["2"]["rev"]["maxIvl"], 90);
        assert_eq!(exported.metadata.deck_config["2"]["rev"]["ivlFct"], 0.75);
        assert_eq!(exported.metadata.deck_config["2"]["rev"]["hardFactor"], 1.4);
        assert_eq!(exported.metadata.deck_config["2"]["rev"]["ease4"], 1.6);
        assert_eq!(exported.metadata.deck_config["2"]["lapse"]["mult"], 0.5);
        assert_eq!(exported.metadata.deck_config["2"]["lapse"]["leechFails"], 6);
        assert_eq!(
            exported.metadata.deck_config["2"]["lapse"]["leechAction"],
            0
        );
        assert_eq!(exported.metadata.deck_config["2"]["new"]["bury"], false);
        assert_eq!(exported.metadata.deck_config["2"]["rev"]["bury"], false);
        assert_eq!(
            exported.metadata.deck_config["2"]["buryInterdayLearning"],
            false
        );

        assert_eq!(state.sessions.len(), 1);
        assert_eq!(state.sessions[0].id, "anki-import:2");
        assert_eq!(state.sessions[0].deck_id, "2");
        assert_eq!(state.sessions[0].status, SessionStatus::Completed);
        assert_eq!(state.sessions[0].cards_reviewed, 1);
        assert_eq!(state.sessions[0].cards_correct, 1);
    }

    #[test]
    fn imports_and_exports_v11_fsrs_deck_options() {
        let mut collection = parse_v11_collection_bytes(&v11_sqlite_collection_bytes()).unwrap();
        collection.metadata.deck_config = serde_json::json!({
            "1": {
                "id": 1,
                "name": "FSRS preset",
                "new": {"perDay": 12, "delays": [3, 12], "ints": [2, 5], "initialFactor": 2800},
                "rev": {"perDay": 80, "maxIvl": 90, "ivlFct": 0.75, "hardFactor": 1.4, "ease4": 1.6},
                "lapse": {"delays": [20], "mult": 0.5, "leechFails": 6, "leechAction": 0},
                "desiredRetention": 0.92,
                "fsrsParams6": [0.1, 1.2, 2.3],
                "weightSearch": "preset:\"FSRS preset\" -is:suspended",
                "ignoreRevlogsBeforeDate": "2024-01-02",
                "sm2Retention": 0.86,
                "easyDaysPercentages": [1.0, 0.9, 0.8, 1.1, 1.2, 1.0, 0.95]
            }
        });

        let state = v11_collection_to_engram_state(&collection).unwrap();
        let options = state
            .deck_options
            .iter()
            .find(|preset| preset.deck_id == "2")
            .map(|preset| &preset.options)
            .expect("Spanish deck should have imported deck options");

        assert_eq!(options.desired_retention, 0.92);
        assert_eq!(options.fsrs_parameters, vec![0.1, 1.2, 2.3]);
        assert_eq!(
            options.fsrs_parameter_search,
            "preset:\"FSRS preset\" -is:suspended"
        );
        assert_eq!(options.ignore_review_history_before, "2024-01-02");
        assert_eq!(options.historical_retention, 0.86);
        assert_eq!(
            options.easy_days_percentages,
            vec![1.0, 0.9, 0.8, 1.1, 1.2, 1.0, 0.95]
        );

        let exported = parse_v11_collection_bytes(
            &write_v11_collection_bytes_from_engram_state(&state).unwrap(),
        )
        .unwrap();
        let deck = exported.decks.iter().find(|deck| deck.id == 2).unwrap();
        let config_id = deck.raw["conf"].as_i64().unwrap();
        let config = &exported.metadata.deck_config[config_id.to_string()];

        assert_eq!(config["desiredRetention"], 0.92);
        assert_eq!(config["fsrsParams6"], serde_json::json!([0.1, 1.2, 2.3]));
        assert_eq!(
            config["weightSearch"],
            "preset:\"FSRS preset\" -is:suspended"
        );
        assert_eq!(config["ignoreRevlogsBeforeDate"], "2024-01-02");
        assert_eq!(config["sm2Retention"], 0.86);
        assert_eq!(
            config["easyDaysPercentages"],
            serde_json::json!([1.0, 0.9, 0.8, 1.1, 1.2, 1.0, 0.95])
        );
    }

    #[test]
    fn maps_v11_due_values_with_collection_day_offset() {
        let collection = parse_v11_collection_bytes(&v11_sqlite_collection_bytes()).unwrap();

        let review_state = v11_collection_to_engram_state(&collection).unwrap();
        assert_eq!(
            review_state.card_progress[0].next_due_at,
            (19_000 + 42) * ONE_DAY_MS
        );

        let mut intraday_learning = collection.clone();
        intraday_learning.cards[0].kind = 1;
        intraday_learning.cards[0].queue = 1;
        intraday_learning.cards[0].due = 1_700_000_300;
        let intraday_state = v11_collection_to_engram_state(&intraday_learning).unwrap();
        assert_eq!(intraday_state.card_progress[0].state, CardState::Learning);
        assert_eq!(
            intraday_state.card_progress[0].next_due_at,
            1_700_000_300_000
        );

        let mut day_learning = collection.clone();
        day_learning.cards[0].kind = 1;
        day_learning.cards[0].queue = 3;
        day_learning.cards[0].due = 43;
        let day_learning_state = v11_collection_to_engram_state(&day_learning).unwrap();
        assert_eq!(
            day_learning_state.card_progress[0].state,
            CardState::Learning
        );
        assert_eq!(
            day_learning_state.card_progress[0].next_due_at,
            (19_000 + 43) * ONE_DAY_MS
        );
        let day_learning_export = parse_v11_collection_bytes(
            &write_v11_collection_bytes_from_engram_state(&day_learning_state).unwrap(),
        )
        .unwrap();
        assert_eq!(day_learning_export.cards[0].kind, 1);
        assert_eq!(day_learning_export.cards[0].queue, 3);
        assert_eq!(day_learning_export.cards[0].due, 43);

        let mut day_relearning = collection.clone();
        day_relearning.cards[0].kind = 3;
        day_relearning.cards[0].queue = 3;
        day_relearning.cards[0].due = 44;
        let day_relearning_state = v11_collection_to_engram_state(&day_relearning).unwrap();
        assert_eq!(
            day_relearning_state.card_progress[0].state,
            CardState::Relearning
        );
        assert_eq!(
            day_relearning_state.card_progress[0].next_due_at,
            (19_000 + 44) * ONE_DAY_MS
        );
        let day_relearning_export = parse_v11_collection_bytes(
            &write_v11_collection_bytes_from_engram_state(&day_relearning_state).unwrap(),
        )
        .unwrap();
        assert_eq!(day_relearning_export.cards[0].kind, 3);
        assert_eq!(day_relearning_export.cards[0].queue, 3);
        assert_eq!(day_relearning_export.cards[0].due, 44);
    }

    #[test]
    fn exports_native_learning_due_after_one_day_as_interday_queue() {
        let collection = parse_v11_collection_bytes(&v11_sqlite_collection_bytes()).unwrap();
        let mut state = v11_collection_to_engram_state(&collection).unwrap();
        state.external_sources.retain(|source| {
            !(source.target == ExternalSourceTarget::Card && source.target_id == "2000")
        });

        let reviewed_at = (19_000 + 42) * ONE_DAY_MS;
        let progress = &mut state.card_progress[0];
        progress.state = CardState::Learning;
        progress.interval = 0;
        progress.learning_step_index = Some(1);
        progress.last_seen_at = reviewed_at;
        progress.next_due_at = reviewed_at + 10 * 60 * 1000;

        let intraday_export = parse_v11_collection_bytes(
            &write_v11_collection_bytes_from_engram_state(&state).unwrap(),
        )
        .unwrap();
        assert_eq!(intraday_export.cards[0].kind, 1);
        assert_eq!(intraday_export.cards[0].queue, 1);
        assert_eq!(
            intraday_export.cards[0].due,
            i64::try_from((reviewed_at + 10 * 60 * 1000) / 1000).unwrap()
        );

        state.card_progress[0].next_due_at = reviewed_at + ONE_DAY_MS;
        let interday_export = parse_v11_collection_bytes(
            &write_v11_collection_bytes_from_engram_state(&state).unwrap(),
        )
        .unwrap();
        assert_eq!(interday_export.cards[0].kind, 1);
        assert_eq!(interday_export.cards[0].queue, 3);
        assert_eq!(interday_export.cards[0].due, 43);
    }

    #[test]
    fn maps_v11_learning_left_as_remaining_steps_and_preserves_packed_today_count() {
        let collection = parse_v11_collection_bytes(&v11_sqlite_collection_bytes()).unwrap();

        let mut first_step = collection.clone();
        first_step.cards[0].kind = 1;
        first_step.cards[0].queue = 1;
        first_step.cards[0].due = 1_700_000_300;
        first_step.cards[0].left = 2;
        let first_step_state = v11_collection_to_engram_state(&first_step).unwrap();
        assert_eq!(
            first_step_state.card_progress[0].learning_step_index,
            Some(0)
        );
        let first_step_export = parse_v11_collection_bytes(
            &write_v11_collection_bytes_from_engram_state(&first_step_state).unwrap(),
        )
        .unwrap();
        assert_eq!(first_step_export.cards[0].left, 2);

        let mut second_step = collection.clone();
        second_step.cards[0].kind = 1;
        second_step.cards[0].queue = 1;
        second_step.cards[0].due = 1_700_000_300;
        second_step.cards[0].left = 1;
        let second_step_state = v11_collection_to_engram_state(&second_step).unwrap();
        assert_eq!(
            second_step_state.card_progress[0].learning_step_index,
            Some(1)
        );
        let second_step_export = parse_v11_collection_bytes(
            &write_v11_collection_bytes_from_engram_state(&second_step_state).unwrap(),
        )
        .unwrap();
        assert_eq!(second_step_export.cards[0].left, 1);

        let mut packed_today = first_step;
        packed_today.cards[0].left = 1002;
        let packed_today_state = v11_collection_to_engram_state(&packed_today).unwrap();
        assert_eq!(
            packed_today_state.card_progress[0].learning_step_index,
            Some(0)
        );
        let packed_today_export = parse_v11_collection_bytes(
            &write_v11_collection_bytes_from_engram_state(&packed_today_state).unwrap(),
        )
        .unwrap();
        assert_eq!(packed_today_export.cards[0].left, 1002);
    }

    #[test]
    fn preserves_v11_negative_queue_kind_for_suspended_and_scheduler_buried_cards() {
        let collection = parse_v11_collection_bytes(&v11_sqlite_collection_bytes()).unwrap();

        let mut scheduler_buried = collection.clone();
        scheduler_buried.cards[0].kind = 2;
        scheduler_buried.cards[0].queue = -3;
        scheduler_buried.cards[0].due = 45;
        scheduler_buried.cards[0].modified_at = 1_700_000_400;
        let scheduler_buried_state = v11_collection_to_engram_state(&scheduler_buried).unwrap();
        let scheduler_buried_progress = &scheduler_buried_state.card_progress[0];
        assert_eq!(scheduler_buried_progress.state, CardState::Buried);
        assert_eq!(
            scheduler_buried_progress.buried_until,
            Some((19_000 + 45) * ONE_DAY_MS)
        );

        let scheduler_buried_export = parse_v11_collection_bytes(
            &write_v11_collection_bytes_from_engram_state(&scheduler_buried_state).unwrap(),
        )
        .unwrap();
        assert_eq!(scheduler_buried_export.cards[0].queue, -3);
        assert_eq!(scheduler_buried_export.cards[0].due, 45);
        let scheduler_buried_reimport =
            v11_collection_to_engram_state(&scheduler_buried_export).unwrap();
        assert_eq!(
            scheduler_buried_reimport.card_progress[0].buried_until,
            scheduler_buried_progress.buried_until
        );

        let mut suspended = collection.clone();
        suspended.cards[0].kind = 2;
        suspended.cards[0].queue = -1;
        suspended.cards[0].due = 46;
        suspended.cards[0].modified_at = 1_700_000_500;
        let suspended_state = v11_collection_to_engram_state(&suspended).unwrap();
        let suspended_progress = &suspended_state.card_progress[0];
        assert_eq!(suspended_progress.state, CardState::Suspended);
        assert_eq!(suspended_progress.suspended_at, Some(1_700_000_500_000));

        let suspended_export = parse_v11_collection_bytes(
            &write_v11_collection_bytes_from_engram_state(&suspended_state).unwrap(),
        )
        .unwrap();
        assert_eq!(suspended_export.cards[0].queue, -1);
        assert_eq!(suspended_export.cards[0].due, 46);
        let suspended_reimport = v11_collection_to_engram_state(&suspended_export).unwrap();
        assert_eq!(
            suspended_reimport.card_progress[0].next_due_at,
            suspended_progress.next_due_at
        );
    }

    #[test]
    fn maps_v11_front_side_and_optional_template_sections() {
        let mut collection = parse_v11_collection_bytes(&v11_sqlite_collection_bytes()).unwrap();
        collection.note_types[0].fields.push(AnkiV11Field {
            ordinal: 2,
            name: "Extra".to_string(),
        });
        collection.note_types[0].templates[0].question_format =
            "{{Front}}{{#Extra}} has-extra{{/Extra}}".to_string();
        collection.note_types[0].templates[0].answer_format =
            "{{FrontSide}}<hr>{{hint:Back}}".to_string();
        collection.notes[0].field_values.push(String::new());

        let state = v11_collection_to_engram_state(&collection).unwrap();

        assert_eq!(
            state.note_types[0].templates[0].required_field_names,
            vec!["Front"]
        );
        assert_eq!(state.cards[0].front, "hola");
        assert_eq!(state.cards[0].back, "hola<hr>[show hint: Back]");
        assert!(!state.cards[0].back.contains(">hello"));
    }

    #[test]
    fn maps_v11_anki_special_template_fields() {
        let mut collection = parse_v11_collection_bytes(&v11_sqlite_collection_bytes()).unwrap();
        collection.note_types[0].templates[0].question_format =
            "{{Tags}}|{{Type}}|{{Deck}}|{{Subdeck}}|{{Card}}|{{CardFlag}}|{{CardID}}".to_string();
        collection.notes[0].tags = vec!["script".to_string(), "root".to_string()];
        collection.cards[0].original_deck_id = 2;
        collection.cards[0].flags = 3;

        let state = v11_collection_to_engram_state(&collection).unwrap();

        assert_eq!(
            state.note_types[0].templates[0].required_field_names,
            Vec::<String>::new()
        );
        assert_eq!(
            state.cards[0].front,
            "script root|Basic|Spanish::Latin|Latin|Card 1|flag3|2000"
        );
    }

    #[test]
    fn maps_v11_template_deck_override_into_regenerated_cards() {
        let mut collection = parse_v11_collection_bytes(&v11_sqlite_collection_bytes()).unwrap();
        collection.decks.push(AnkiV11Deck {
            id: 3,
            name: "Spanish::Reverse".to_string(),
            description: "Template override deck".to_string(),
            raw: serde_json::json!({
                "id": 3,
                "name": "Spanish::Reverse",
                "desc": "Template override deck"
            }),
        });
        collection.note_types[0].templates[0].deck_id = Some(3);
        collection.note_types[0].raw["tmpls"][0]["did"] = serde_json::json!(3);

        let state = v11_collection_to_engram_state(&collection).unwrap();
        let template = &state.note_types[0].templates[0];
        let generated = engram_core::generate_cards_for_note(&state.note_types[0], &state.notes[0]);

        assert_eq!(state.notes[0].deck_id, "2");
        assert_eq!(state.cards[0].deck_id, "2");
        assert_eq!(template.deck_id.as_deref(), Some("3"));
        assert_eq!(generated[0].deck_id, "3");

        let exported = parse_v11_collection_bytes(
            &write_v11_collection_bytes_from_engram_state(&state).unwrap(),
        )
        .unwrap();
        assert_eq!(exported.note_types[0].templates[0].deck_id, Some(3));
        assert_eq!(exported.note_types[0].raw["tmpls"][0]["did"], 3);
    }

    #[test]
    fn maps_v11_model_req_any_into_generated_card_rules() {
        let mut collection = parse_v11_collection_bytes(&v11_sqlite_collection_bytes()).unwrap();
        collection.note_types[0].raw["req"] = serde_json::json!([[0, "any", [0, 1]]]);
        collection.note_types[0].templates[0].question_format = "{{Front}}{{Back}}".to_string();
        collection.notes[0].field_values = vec![String::new(), "hello".to_string()];

        let state = v11_collection_to_engram_state(&collection).unwrap();
        let template = &state.note_types[0].templates[0];

        assert_eq!(template.requirement_mode, TemplateRequirementMode::Any);
        assert_eq!(
            template.required_field_names,
            vec!["Front".to_string(), "Back".to_string()]
        );
        let generated = engram_core::generate_cards_for_note(&state.note_types[0], &state.notes[0]);
        assert_eq!(generated.len(), 1);
        assert_eq!(generated[0].front, "hello");
    }

    #[test]
    fn maps_v11_new_card_flags_as_metadata_overlays() {
        let mut collection = parse_v11_collection_bytes(&v11_sqlite_collection_bytes()).unwrap();
        collection.cards[0].kind = 0;
        collection.cards[0].queue = 0;
        collection.cards[0].due = 1;
        collection.cards[0].interval = 0;
        collection.cards[0].repetitions = 0;
        collection.cards[0].lapses = 0;
        collection.cards[0].flags = 1;

        let state = v11_collection_to_engram_state(&collection).unwrap();

        assert_eq!(state.card_progress.len(), 1);
        let progress = &state.card_progress[0];
        assert_eq!(progress.card_id, "2000");
        assert_eq!(progress.state, CardState::Review);
        assert_eq!(progress.interval, 0);
        assert_eq!(progress.times_seen, 0);
        assert_eq!(progress.times_correct, 0);
        assert_eq!(progress.times_incorrect, 0);
        assert_eq!(progress.flag, Some(CardFlag::Red));

        let queue = engram_core::build_session_queue(&state.cards, &state.card_progress, "2", 0);
        assert_eq!(
            queue
                .iter()
                .map(|card| card.id.as_str())
                .collect::<Vec<_>>(),
            vec!["2000"]
        );
    }

    #[test]
    fn maps_v11_marked_note_tag_to_card_progress_overlay() {
        let mut collection = parse_v11_collection_bytes(&v11_sqlite_collection_bytes()).unwrap();
        collection.notes[0].tags.push("marked".to_string());
        collection.notes[0].modified_at = 1_234;
        collection.cards[0].kind = 0;
        collection.cards[0].queue = 0;
        collection.cards[0].due = 1;
        collection.cards[0].interval = 0;
        collection.cards[0].repetitions = 0;
        collection.cards[0].lapses = 0;
        collection.cards[0].flags = 0;

        let state = v11_collection_to_engram_state(&collection).unwrap();

        assert_eq!(state.card_progress.len(), 1);
        let progress = &state.card_progress[0];
        assert_eq!(progress.card_id, "2000");
        assert_eq!(progress.state, CardState::Review);
        assert_eq!(progress.interval, 0);
        assert_eq!(progress.times_seen, 0);
        assert_eq!(progress.flag, None);
        assert_eq!(progress.marked_at, Some(1_234_000));

        let queue = engram_core::build_session_queue(&state.cards, &state.card_progress, "2", 0);
        assert_eq!(
            queue
                .iter()
                .map(|card| card.id.as_str())
                .collect::<Vec<_>>(),
            vec!["2000"]
        );
    }

    #[test]
    fn exports_marked_card_progress_as_anki_marked_note_tag() {
        let collection = parse_v11_collection_bytes(&v11_sqlite_collection_bytes()).unwrap();
        let mut state = v11_collection_to_engram_state(&collection).unwrap();
        state.notes[0].tags = vec!["spanish".to_string(), "Marked".to_string()];
        state.card_progress[0].marked_at = Some(1_700_000_040_000);

        let exported = parse_v11_collection_bytes(
            &write_v11_collection_bytes_from_engram_state(&state).unwrap(),
        )
        .unwrap();

        assert_eq!(exported.notes[0].tags, vec!["spanish", "marked"]);
        assert_eq!(exported.metadata.tags["marked"], serde_json::json!(1));
    }

    #[test]
    fn imported_v11_source_metadata_round_trips_on_export() {
        let mut collection = parse_v11_collection_bytes(&v11_sqlite_collection_bytes()).unwrap();
        collection.metadata.modified_at = 1_700_001_111;
        collection.metadata.schema_modified_at = 1_700_002_222;
        collection.metadata.version = 11;
        collection.metadata.dirty = 7;
        collection.metadata.update_sequence_number = 33;
        collection.metadata.last_sync = 1_700_003_333;
        collection.metadata.config = serde_json::json!({
            "nextPos": 99,
            "customStudy": true,
        });
        collection.metadata.deck_config = serde_json::json!({
            "2": { "id": 2, "name": "Story defaults" },
        });
        collection.metadata.tags = serde_json::json!({
            "spanish": 1,
            "imported": 2,
        });
        collection.decks[1].raw = serde_json::json!({
            "id": 2,
            "name": "Spanish::Latin",
            "desc": "Story deck",
            "conf": 7,
            "mod": 1_700_004_444_i64,
            "dyn": 1,
            "extendNew": 25,
            "extendRev": 75,
            "collapsed": true,
        });
        collection.note_types[0].raw = serde_json::json!({
            "id": 100,
            "name": "Basic",
            "type": 0,
            "mod": 1_700_005_555_i64,
            "css": ".card { color: teal; }",
            "sortf": 1,
            "latexPre": "custom pre",
            "latexPost": "custom post",
            "flds": [
                { "name": "Front", "ord": 0, "font": "Noto Sans", "sticky": true },
                { "name": "Back", "ord": 1, "rtl": true, "size": 24 }
            ],
            "tmpls": [
                {
                    "name": "Card 1",
                    "ord": 0,
                    "qfmt": "{{Front}}",
                    "afmt": "{{Back}}",
                    "did": 2,
                    "bqfmt": "browser question"
                }
            ]
        });
        collection.note_types[0].css = ".card { color: teal; }".to_string();
        collection.notes[0].guid = "stable-guid".to_string();
        collection.notes[0].modified_at = 1_700_006_666;
        collection.notes[0].update_sequence_number = 17;
        collection.notes[0].sort_field = "hello".to_string();
        collection.notes[0].checksum = 4242;
        collection.notes[0].flags = 5;
        collection.notes[0].data = "note-data".to_string();
        collection.cards[0].modified_at = 1_700_007_777;
        collection.cards[0].update_sequence_number = 23;
        collection.cards[0].original_due = 777;
        collection.cards[0].original_deck_id = 1;
        collection.cards[0].data = "card-data".to_string();
        collection.reviews[0].update_sequence_number = 29;
        collection.reviews[0].ease = 4;
        collection.reviews[0].interval = 12;
        collection.reviews[0].last_interval = 6;
        collection.reviews[0].factor = 2650;
        collection.reviews[0].time = 34_567;
        collection.reviews[0].kind = 3;
        collection.graves = vec![AnkiV11Grave {
            update_sequence_number: 31,
            object_id: 9001,
            kind: 2,
        }];

        let state = v11_collection_to_engram_state(&collection).unwrap();
        assert_eq!(
            state.note_types[0].stylesheet.as_deref(),
            Some(".card { color: teal; }")
        );
        assert!(state.external_sources.iter().any(|source| {
            source.source == ANKI_V11_SOURCE
                && source.target == ExternalSourceTarget::NoteType
                && source.target_id == "100"
        }));

        let exported = parse_v11_collection_bytes(
            &write_v11_collection_bytes_from_engram_state(&state).unwrap(),
        )
        .unwrap();

        assert_eq!(exported.metadata.config["customStudy"], true);
        assert_eq!(exported.metadata.config["nextPos"], 1);
        assert_eq!(exported.metadata.modified_at, 1_700_001_111);
        assert_eq!(exported.metadata.schema_modified_at, 1_700_002_222);
        assert_eq!(exported.metadata.version, 11);
        assert_eq!(exported.metadata.dirty, 7);
        assert_eq!(exported.metadata.update_sequence_number, 33);
        assert_eq!(exported.metadata.last_sync, 1_700_003_333);
        assert_eq!(exported.metadata.deck_config["2"]["name"], "Story defaults");
        assert_eq!(exported.metadata.tags["imported"], 2);
        assert_eq!(exported.graves[0].object_id, 9001);
        assert_eq!(exported.graves[0].kind, 2);

        let deck = exported.decks.iter().find(|deck| deck.id == 2).unwrap();
        assert_eq!(deck.raw["conf"], 7);
        assert_eq!(deck.raw["mod"], 1_700_004_444_i64);
        assert_eq!(deck.raw["dyn"], 1);
        assert_eq!(deck.raw["collapsed"], true);
        assert_eq!(deck.name, "Spanish::Latin");

        let note_type = &exported.note_types[0];
        assert_eq!(note_type.css, ".card { color: teal; }");
        assert_eq!(note_type.raw["mod"], 1_700_005_555_i64);
        assert_eq!(note_type.raw["sortf"], 1);
        assert_eq!(note_type.raw["latexPre"], "custom pre");
        assert_eq!(note_type.raw["flds"][0]["font"], "Noto Sans");
        assert_eq!(note_type.raw["flds"][0]["sticky"], true);
        assert_eq!(note_type.raw["flds"][1]["rtl"], true);
        assert_eq!(note_type.raw["flds"][1]["size"], 24);
        assert_eq!(note_type.templates[0].deck_id, Some(2));
        assert_eq!(note_type.raw["tmpls"][0]["bqfmt"], "browser question");

        let note = &exported.notes[0];
        assert_eq!(note.guid, "stable-guid");
        assert_eq!(note.modified_at, 1_700_006_666);
        assert_eq!(note.update_sequence_number, 17);
        assert_eq!(note.sort_field, "hello");
        assert_eq!(note.checksum, 4242);
        assert_eq!(note.flags, 5);
        assert_eq!(note.data, "note-data");

        let card = &exported.cards[0];
        assert_eq!(card.modified_at, 1_700_007_777);
        assert_eq!(card.update_sequence_number, 23);
        assert_eq!(card.original_due, 777);
        assert_eq!(card.original_deck_id, 1);
        assert_eq!(card.data, "card-data");

        let review = &exported.reviews[0];
        assert_eq!(review.update_sequence_number, 29);
        assert_eq!(review.ease, 4);
        assert_eq!(review.interval, 12);
        assert_eq!(review.last_interval, 6);
        assert_eq!(review.factor, 2650);
        assert_eq!(review.time, 34_567);
        assert_eq!(review.kind, 3);
    }

    #[test]
    fn exports_current_fsrs_memory_into_v11_card_data() {
        let mut collection = parse_v11_collection_bytes(&v11_sqlite_collection_bytes()).unwrap();
        collection.cards[0].data =
            serde_json::json!({ "s": 1.25, "d": 2.5, "cd": { "source": "kept" } }).to_string();
        let mut state = v11_collection_to_engram_state(&collection).unwrap();
        state.card_progress[0].fsrs_stability = Some(9.5);
        state.card_progress[0].fsrs_difficulty = Some(4.25);

        let exported = parse_v11_collection_bytes(
            &write_v11_collection_bytes_from_engram_state(&state).unwrap(),
        )
        .unwrap();
        let card_data = serde_json::from_str::<Value>(&exported.cards[0].data).unwrap();

        assert_eq!(card_data["s"], serde_json::json!(9.5));
        assert_eq!(card_data["d"], serde_json::json!(4.25));
        assert_eq!(card_data["cd"]["source"], "kept");

        let reimported = v11_collection_to_engram_state(&exported).unwrap();
        assert_eq!(reimported.card_progress[0].fsrs_stability, Some(9.5));
        assert_eq!(reimported.card_progress[0].fsrs_difficulty, Some(4.25));
    }

    #[test]
    fn exports_graves_for_deleted_imported_notes_and_cards() {
        let collection = parse_v11_collection_bytes(&v11_sqlite_collection_bytes()).unwrap();
        let state = v11_collection_to_engram_state(&collection).unwrap();

        let deleted = engram_core::reduce(
            &state,
            engram_core::EngramCommand::DeleteNote {
                note_id: "1000".to_string(),
            },
        );

        assert!(deleted.notes.is_empty());
        assert!(deleted.cards.is_empty());
        assert!(deleted.external_sources.iter().any(|source| {
            source.target == ExternalSourceTarget::Deleted
                && source.original_id.as_deref() == Some("1000")
                && source.data.get("deletedTarget").map(String::as_str) == Some("note")
        }));
        assert!(deleted.external_sources.iter().any(|source| {
            source.target == ExternalSourceTarget::Deleted
                && source.original_id.as_deref() == Some("2000")
                && source.data.get("deletedTarget").map(String::as_str) == Some("card")
        }));

        let exported = parse_v11_collection_bytes(
            &write_v11_collection_bytes_from_engram_state(&deleted).unwrap(),
        )
        .unwrap();

        assert!(exported
            .graves
            .iter()
            .any(|grave| grave.object_id == 1000 && grave.kind == 1));
        assert!(exported
            .graves
            .iter()
            .any(|grave| grave.object_id == 2000 && grave.kind == 0));
        assert_eq!(exported.reviews.len(), 1);
        assert_eq!(exported.reviews[0].card_id, 2000);
    }

    #[test]
    fn exports_graves_for_deleted_imported_decks() {
        let collection = parse_v11_collection_bytes(&v11_sqlite_collection_bytes()).unwrap();
        let state = v11_collection_to_engram_state(&collection).unwrap();

        let deleted = engram_core::reduce(
            &state,
            engram_core::EngramCommand::DeleteDeck {
                deck_id: "2".to_string(),
            },
        );

        assert!(deleted.decks.iter().all(|deck| deck.id != "2"));
        assert!(deleted.external_sources.iter().any(|source| {
            source.target == ExternalSourceTarget::Deleted
                && source.original_id.as_deref() == Some("2")
                && source.data.get("deletedTarget").map(String::as_str) == Some("deck")
        }));

        let exported = parse_v11_collection_bytes(
            &write_v11_collection_bytes_from_engram_state(&deleted).unwrap(),
        )
        .unwrap();

        assert!(exported
            .graves
            .iter()
            .any(|grave| grave.object_id == 2 && grave.kind == 2));
    }

    #[test]
    fn maps_v11_cloze_cards_into_rendered_engram_cards() {
        let collection = AnkiV11Collection {
            metadata: AnkiV11CollectionMetadata {
                id: 1,
                created_at_days: 19_000,
                modified_at: 0,
                schema_modified_at: 0,
                version: 11,
                dirty: 0,
                update_sequence_number: -1,
                last_sync: 0,
                config: serde_json::json!({}),
                deck_config: serde_json::json!({}),
                tags: serde_json::json!({}),
            },
            decks: vec![AnkiV11Deck {
                id: 1,
                name: "Cloze".to_string(),
                description: String::new(),
                raw: serde_json::json!({}),
            }],
            note_types: vec![AnkiV11NoteType {
                id: 100,
                name: "Cloze".to_string(),
                kind: 1,
                css: String::new(),
                fields: vec![
                    AnkiV11Field {
                        ordinal: 0,
                        name: "Text".to_string(),
                    },
                    AnkiV11Field {
                        ordinal: 1,
                        name: "Extra".to_string(),
                    },
                ],
                templates: vec![AnkiV11Template {
                    ordinal: 0,
                    name: "Cloze".to_string(),
                    question_format: "{{#Extra}}{{type:cloze:Text}}{{/Extra}}".to_string(),
                    answer_format: "{{FrontSide}}<hr>{{text:cloze:Text}}<br>{{Extra}}".to_string(),
                    deck_id: None,
                }],
                raw: serde_json::json!({}),
            }],
            notes: vec![AnkiV11Note {
                id: 1000,
                guid: "cloze-guid".to_string(),
                note_type_id: 100,
                modified_at: 1_700_000_010,
                update_sequence_number: -1,
                tags: vec!["roots".to_string()],
                field_values: vec![
                    "The word {{c1::night::old root}} meets {{c2::nox::Latin}}.".to_string(),
                    "Proto-Indo-European stories go here.".to_string(),
                ],
                sort_field: "The word night meets nox.".to_string(),
                checksum: 0,
                flags: 0,
                data: String::new(),
            }],
            cards: vec![
                AnkiV11Card {
                    id: 2000,
                    note_id: 1000,
                    deck_id: 1,
                    ordinal: 0,
                    modified_at: 1_700_000_020,
                    update_sequence_number: -1,
                    kind: 0,
                    queue: 0,
                    due: 0,
                    interval: 0,
                    factor: 0,
                    repetitions: 0,
                    lapses: 0,
                    left: 0,
                    original_due: 0,
                    original_deck_id: 0,
                    flags: 0,
                    data: String::new(),
                },
                AnkiV11Card {
                    id: 2001,
                    note_id: 1000,
                    deck_id: 1,
                    ordinal: 1,
                    modified_at: 1_700_000_021,
                    update_sequence_number: -1,
                    kind: 0,
                    queue: 0,
                    due: 1,
                    interval: 0,
                    factor: 0,
                    repetitions: 0,
                    lapses: 0,
                    left: 0,
                    original_due: 0,
                    original_deck_id: 0,
                    flags: 0,
                    data: String::new(),
                },
            ],
            reviews: Vec::new(),
            graves: Vec::new(),
        };

        let state = v11_collection_to_engram_state(&collection).unwrap();

        assert_eq!(
            state.note_types[0].templates[0].required_field_names,
            vec!["Text"]
        );
        assert_eq!(state.cards[0].front, "[type answer: Text]");
        assert_eq!(
            state.cards[0].back,
            "[type answer: Text]<hr>The word night meets nox.<br>Proto-Indo-European stories go here."
        );
        assert_eq!(state.cards[1].front, "[type answer: Text]");
        assert_eq!(
            state.cards[1].back,
            "[type answer: Text]<hr>The word night meets nox.<br>Proto-Indo-European stories go here."
        );
        let first_lineage = state.cards[0].lineage.as_ref().unwrap();
        assert_eq!(first_lineage.template_id, "100:template:0");
        assert_eq!(first_lineage.ordinal, 0);
        assert_eq!(first_lineage.cloze_ordinal, Some(1));
        let second_lineage = state.cards[1].lineage.as_ref().unwrap();
        assert_eq!(second_lineage.template_id, "100:template:0");
        assert_eq!(second_lineage.ordinal, 1);
        assert_eq!(second_lineage.cloze_ordinal, Some(2));
        assert!(state.card_progress.is_empty());

        let exported = parse_v11_collection_bytes(
            &write_v11_collection_bytes_from_engram_state(&state).unwrap(),
        )
        .unwrap();
        assert_eq!(exported.note_types[0].kind, 1);
        assert_eq!(exported.cards[0].ordinal, 0);
        assert_eq!(exported.cards[1].ordinal, 1);
    }

    #[test]
    fn reads_v11_apkg_directly_as_engram_app_state() {
        let sqlite = v11_sqlite_collection_bytes();
        let apkg = write_legacy_apkg(&sqlite, &[]);

        let state = read_v11_collection_as_engram_state(&apkg).unwrap();

        assert_eq!(state.cards[0].front, "hola");
        assert_eq!(state.cards[0].back, "hello");
    }

    fn assert_golden_v11_apkg_fixture_round_trips_filtered_deck_and_media(apkg: &[u8]) {
        let manifest = inspect_apkg(&apkg).unwrap();
        assert_eq!(manifest.collection.name, LEGACY_COLLECTION);
        assert_eq!(manifest.media.media_files.len(), 2);
        assert!(manifest.media.missing_files.is_empty());
        assert!(manifest.media.unmapped_files.is_empty());

        let collection = read_v11_collection(&apkg).unwrap();
        let filtered_deck = collection.decks.iter().find(|deck| deck.id == 3).unwrap();
        assert_eq!(filtered_deck.name, "Filtered::Today");
        assert_eq!(filtered_deck.raw["dyn"], 1);
        assert_eq!(filtered_deck.raw["resched"], true);
        assert_eq!(
            collection.notes[0].tags,
            vec!["spanish", "media", "filtered"]
        );
        assert!(collection.notes[0].field_values[0].contains("[sound:audio/hola.mp3]"));
        assert!(collection.notes[0].field_values[1].contains("images/card.png"));
        assert_eq!(collection.cards[0].deck_id, 3);
        assert_eq!(collection.cards[0].original_due, 42);
        assert_eq!(collection.cards[0].original_deck_id, 2);
        assert_eq!(collection.cards[0].data, "filtered-card");

        let state = read_v11_collection_as_engram_state(&apkg).unwrap();
        assert_eq!(state.media_assets.len(), 2);
        assert_eq!(state.cards[0].deck_id, "3");
        assert!(state.cards[0].front.contains("[sound:audio/hola.mp3]"));
        assert!(state.cards[0].back.contains("images/card.png"));
        assert!(state.external_sources.iter().any(|source| {
            source.source == ANKI_V11_SOURCE
                && source.target == ExternalSourceTarget::Deck
                && source.target_id == "3"
                && source.data.get("dyn").map(String::as_str) == Some("1")
                && source.data.get("resched").map(String::as_str) == Some("true")
        }));
        assert!(state.external_sources.iter().any(|source| {
            source.source == ANKI_V11_SOURCE
                && source.target == ExternalSourceTarget::Card
                && source.target_id == "2000"
                && source.data.get("originalDeckId").map(String::as_str) == Some("2")
                && source.data.get("originalDue").map(String::as_str) == Some("42")
                && source.data.get("data").map(String::as_str) == Some("filtered-card")
        }));
        assert!(state.external_sources.iter().any(|source| {
            source.source == ANKI_V11_SOURCE
                && source.target == ExternalSourceTarget::Media
                && source.target_id == "anki-media:0"
                && source.original_id.as_deref() == Some("0")
                && source.data.get("filename").map(String::as_str) == Some("audio/hola.mp3")
        }));

        let media_analysis = analyze_engram_media_references(&state);
        assert_eq!(
            media_analysis.referenced_filenames,
            vec!["audio/hola.mp3".to_string(), "images/card.png".to_string()]
        );
        assert_eq!(
            media_analysis.referenced_asset_ids,
            vec!["anki-media:0".to_string(), "anki-media:1".to_string()]
        );
        assert!(media_analysis.missing_filenames.is_empty());
        assert!(media_analysis.unreferenced_asset_ids.is_empty());

        let exported = write_legacy_apkg_from_engram_state(&state, &[]).unwrap();
        let exported_collection = read_v11_collection(&exported).unwrap();
        let exported_filtered_deck = exported_collection
            .decks
            .iter()
            .find(|deck| deck.id == 3)
            .unwrap();
        assert_eq!(exported_filtered_deck.raw["dyn"], 1);
        assert_eq!(exported_filtered_deck.raw["terms"][0][0], "deck:Spanish");
        assert_eq!(exported_collection.cards[0].deck_id, 3);
        assert_eq!(exported_collection.cards[0].original_due, 42);
        assert_eq!(exported_collection.cards[0].original_deck_id, 2);
        assert_eq!(exported_collection.cards[0].data, "filtered-card");

        let exported_manifest = inspect_apkg(&exported).unwrap();
        assert_eq!(exported_manifest.media.mapping["0"], "audio/hola.mp3");
        assert_eq!(exported_manifest.media.mapping["1"], "images/card.png");
        let audio = read_media_file(&exported, "0").unwrap();
        assert_eq!(audio.filename.as_deref(), Some("audio/hola.mp3"));
        assert_eq!(audio.data, b"mp3");
    }

    #[test]
    fn generated_golden_v11_apkg_fixture_round_trips_filtered_deck_and_media() {
        let apkg = golden_v11_apkg_fixture_bytes();
        assert_golden_v11_apkg_fixture_round_trips_filtered_deck_and_media(&apkg);
    }

    #[test]
    fn checked_in_golden_v11_apkg_fixture_round_trips_filtered_deck_and_media() {
        assert_golden_v11_apkg_fixture_round_trips_filtered_deck_and_media(
            checked_in_golden_v11_apkg_fixture_bytes(),
        );
    }

    #[test]
    fn writes_v11_collection_from_engram_note_state() {
        let state = AppState {
            decks: vec![Deck {
                id: "2".to_string(),
                name: "Spanish::Latin".to_string(),
                description: "Story deck".to_string(),
                created_at: 1_641_600_000_000,
            }],
            note_types: vec![NoteType {
                id: "100".to_string(),
                name: "Basic".to_string(),
                fields: vec![
                    FieldDef {
                        id: "100:field:0".to_string(),
                        name: "Front".to_string(),
                        required: true,
                        ordinal: 0,
                    },
                    FieldDef {
                        id: "100:field:1".to_string(),
                        name: "Back".to_string(),
                        required: true,
                        ordinal: 1,
                    },
                ],
                templates: vec![CardTemplate {
                    id: "100:template:0".to_string(),
                    name: "Card 1".to_string(),
                    front_template: "{{Front}}".to_string(),
                    back_template: "{{Back}}".to_string(),
                    deck_id: Some("2".to_string()),
                    required_field_names: vec!["Front".to_string(), "Back".to_string()],
                    requirement_mode: TemplateRequirementMode::Any,
                    ordinal: 0,
                }],
                stylesheet: Some(".card { color: navy; }".to_string()),
                created_at: 1_641_600_000_000,
                updated_at: 1_641_600_000_000,
            }],
            notes: vec![Note {
                id: "1000".to_string(),
                note_type_id: "100".to_string(),
                deck_id: "2".to_string(),
                fields: vec![
                    NoteFieldValue {
                        field_id: "100:field:0".to_string(),
                        value: "hola".to_string(),
                    },
                    NoteFieldValue {
                        field_id: "100:field:1".to_string(),
                        value: "hello".to_string(),
                    },
                ],
                tags: vec!["spanish".to_string(), "roots".to_string()],
                created_at: 1_700_000_010_000,
                updated_at: 1_700_000_010_000,
            }],
            cards: vec![Card {
                id: "2000".to_string(),
                deck_id: "2".to_string(),
                front: "hola".to_string(),
                back: "hello".to_string(),
                created_at: 1_700_000_020_000,
                lineage: Some(CardLineage {
                    note_id: "1000".to_string(),
                    note_type_id: "100".to_string(),
                    template_id: "100:template:0".to_string(),
                    ordinal: 0,
                    cloze_ordinal: None,
                }),
            }],
            card_progress: vec![CardProgress {
                card_id: "2000".to_string(),
                state: CardState::Review,
                interval: 7,
                ease_factor: 2.5,
                next_due_at: 1_704_672_000_000,
                learning_step_index: None,
                buried_until: None,
                suspended_at: None,
                times_seen: 3,
                times_correct: 2,
                times_incorrect: 1,
                last_seen_at: 1_700_000_030_000,
                fsrs_stability: Some(6.5),
                fsrs_difficulty: Some(7.25),
                flag: Some(CardFlag::Blue),
                marked_at: None,
            }],
            sessions: Vec::new(),
            reviews: vec![Review {
                id: "1700000030000".to_string(),
                session_id: "session".to_string(),
                card_id: "2000".to_string(),
                rating: Rating::Good,
                reviewed_at: 1_700_000_030_000,
                answer_time_ms: Some(12_345),
                leech_event: None,
                previous_progress: None,
                resulting_progress: None,
                previous_active_session: None,
                sibling_progress_snapshots: Vec::new(),
            }],
            deck_options: Vec::new(),
            external_sources: Vec::new(),
            media_assets: Vec::new(),
            active_session: None,
        };

        let collection = parse_v11_collection_bytes(
            &write_v11_collection_bytes_from_engram_state(&state).unwrap(),
        )
        .unwrap();

        assert_eq!(collection.decks[0].id, 2);
        assert_eq!(collection.decks[0].name, "Spanish::Latin");
        assert_eq!(collection.note_types[0].id, 100);
        assert_eq!(collection.note_types[0].css, ".card { color: navy; }");
        assert_eq!(collection.note_types[0].templates[0].deck_id, Some(2));
        assert_eq!(collection.note_types[0].raw["tmpls"][0]["did"], 2);
        assert_eq!(
            collection.note_types[0].raw["req"],
            serde_json::json!([[0, "any", [0, 1]]])
        );
        assert_eq!(collection.notes[0].id, 1000);
        assert_eq!(collection.notes[0].tags, vec!["spanish", "roots"]);
        assert_eq!(collection.cards[0].id, 2000);
        assert_eq!(collection.cards[0].queue, 2);
        assert_eq!(collection.cards[0].interval, 7);
        assert_eq!(collection.cards[0].factor, 2500);
        assert_eq!(collection.cards[0].flags, 4);
        let card_data = serde_json::from_str::<Value>(&collection.cards[0].data).unwrap();
        assert_eq!(card_data["s"], serde_json::json!(6.5));
        assert_eq!(card_data["d"], serde_json::json!(7.25));
        assert_eq!(collection.reviews[0].card_id, 2000);
        assert_eq!(collection.reviews[0].ease, 3);
        assert_eq!(collection.reviews[0].time, 12_345);

        let apkg = write_legacy_apkg_from_engram_state(&state, &[]).unwrap();
        let imported = read_v11_collection_as_engram_state(&apkg).unwrap();

        assert_eq!(imported.decks[0].name, "Spanish::Latin");
        assert_eq!(
            imported.note_types[0].templates[0].deck_id.as_deref(),
            Some("2")
        );
        assert_eq!(
            imported.note_types[0].stylesheet.as_deref(),
            Some(".card { color: navy; }")
        );
        assert_eq!(imported.notes[0].fields[0].value, "hola");
        assert_eq!(imported.cards[0].front, "hola");
        assert_eq!(imported.cards[0].back, "hello");
        assert_eq!(imported.card_progress[0].interval, 7);
        assert_eq!(imported.card_progress[0].fsrs_stability, Some(6.5));
        assert_eq!(imported.card_progress[0].fsrs_difficulty, Some(7.25));
        assert_eq!(imported.card_progress[0].flag, Some(CardFlag::Blue));
        assert_eq!(imported.reviews[0].answer_time_ms, Some(12_345));
    }

    #[test]
    fn native_filtered_deck_reviews_export_as_cram_revlog_kind() {
        for resched in [true, false] {
            let mut deck_source_data = BTreeMap::new();
            deck_source_data.insert("dyn".to_string(), "1".to_string());
            deck_source_data.insert("resched".to_string(), resched.to_string());
            deck_source_data.insert(
                "rawJson".to_string(),
                serde_json::json!({
                    "id": 3,
                    "name": "Filtered::Today",
                    "desc": "Custom study",
                    "dyn": 1,
                    "conf": 1,
                    "terms": [["deck:Spanish", 10, 0]],
                    "resched": resched,
                })
                .to_string(),
            );

            let previous_progress = CardProgress {
                card_id: "card".to_string(),
                state: CardState::Review,
                interval: 2,
                ease_factor: 2.5,
                next_due_at: 1_700_086_400_000,
                learning_step_index: None,
                buried_until: None,
                suspended_at: None,
                times_seen: 2,
                times_correct: 2,
                times_incorrect: 0,
                last_seen_at: 1_700_000_000_000,
                fsrs_stability: None,
                fsrs_difficulty: None,
                flag: None,
                marked_at: None,
            };
            let resulting_progress = CardProgress {
                interval: 5,
                next_due_at: 1_700_432_000_000,
                times_seen: 3,
                times_correct: 3,
                last_seen_at: 1_700_000_005_000,
                ..previous_progress.clone()
            };

            let mut state = AppState::default();
            state.decks.push(Deck {
                id: "filtered".to_string(),
                name: "Filtered::Today".to_string(),
                description: "Custom study".to_string(),
                created_at: 1_700_000_000_000,
            });
            state.cards.push(Card {
                id: "card".to_string(),
                deck_id: "filtered".to_string(),
                front: "hola".to_string(),
                back: "hello".to_string(),
                created_at: 1_700_000_000_000,
                lineage: None,
            });
            state.card_progress.push(resulting_progress.clone());
            state.sessions.push(Session {
                id: "filtered-session".to_string(),
                deck_id: "filtered".to_string(),
                status: SessionStatus::Completed,
                started_at: 1_700_000_000_000,
                ended_at: Some(1_700_000_005_000),
                cards_reviewed: 1,
                cards_correct: 1,
            });
            state.reviews.push(Review {
                id: "1700000005000".to_string(),
                session_id: "filtered-session".to_string(),
                card_id: "card".to_string(),
                rating: Rating::Good,
                reviewed_at: 1_700_000_005_000,
                answer_time_ms: Some(987),
                leech_event: None,
                previous_progress: Some(previous_progress),
                resulting_progress: Some(resulting_progress),
                previous_active_session: None,
                sibling_progress_snapshots: Vec::new(),
            });
            state.external_sources.push(ExternalSourceRecord {
                target: ExternalSourceTarget::Deck,
                target_id: "filtered".to_string(),
                source: ANKI_V11_SOURCE.to_string(),
                original_id: Some("3".to_string()),
                data: deck_source_data,
            });

            let collection = parse_v11_collection_bytes(
                &write_v11_collection_bytes_from_engram_state(&state).unwrap(),
            )
            .unwrap();

            let filtered_deck = collection
                .decks
                .iter()
                .find(|deck| deck.name == "Filtered::Today")
                .unwrap();
            assert_eq!(filtered_deck.raw["dyn"], 1);
            assert_eq!(filtered_deck.raw["resched"], resched);
            assert_eq!(
                collection.reviews[0].kind, 3,
                "resched={resched} filtered reviews should export as Anki cram rows"
            );
        }
    }

    #[test]
    fn native_filtered_deck_source_keys_export_dynamic_deck_json_and_original_deck_ids() {
        let mut state = AppState::default();
        state.decks.push(Deck {
            id: "spanish".to_string(),
            name: "Spanish".to_string(),
            description: String::new(),
            created_at: 1_700_000_000_000,
        });
        state.decks.push(Deck {
            id: "filtered".to_string(),
            name: "Filtered::Today".to_string(),
            description: "Custom study".to_string(),
            created_at: 1_700_000_000_000,
        });
        state.cards.push(Card {
            id: "card".to_string(),
            deck_id: "filtered".to_string(),
            front: "hola".to_string(),
            back: "hello".to_string(),
            created_at: 1_700_000_000_000,
            lineage: None,
        });
        state.external_sources.push(ExternalSourceRecord {
            target: ExternalSourceTarget::Deck,
            target_id: "filtered".to_string(),
            source: ANKI_V11_SOURCE.to_string(),
            original_id: None,
            data: BTreeMap::from([
                ("dyn".to_string(), "1".to_string()),
                ("resched".to_string(), "false".to_string()),
                ("search".to_string(), "deck:Spanish is:due".to_string()),
                ("limit".to_string(), "10".to_string()),
                ("order".to_string(), "0".to_string()),
            ]),
        });
        state.external_sources.push(ExternalSourceRecord {
            target: ExternalSourceTarget::Card,
            target_id: "card".to_string(),
            source: ANKI_V11_SOURCE.to_string(),
            original_id: None,
            data: BTreeMap::from([("originalDeckId".to_string(), "spanish".to_string())]),
        });

        let collection = parse_v11_collection_bytes(
            &write_v11_collection_bytes_from_engram_state(&state).unwrap(),
        )
        .unwrap();

        let filtered_deck = collection
            .decks
            .iter()
            .find(|deck| deck.name == "Filtered::Today")
            .unwrap();
        assert_eq!(filtered_deck.raw["dyn"], 1);
        assert_eq!(filtered_deck.raw["resched"], false);
        assert_eq!(filtered_deck.raw["terms"][0][0], "deck:Spanish is:due");
        assert_eq!(filtered_deck.raw["terms"][0][1], 10);

        let original_deck_id = collection
            .decks
            .iter()
            .find(|deck| deck.name == "Spanish")
            .map(|deck| deck.id)
            .unwrap();
        assert_eq!(collection.cards[0].deck_id, filtered_deck.id);
        assert_eq!(collection.cards[0].original_deck_id, original_deck_id);
    }

    fn native_review_progress(
        state: CardState,
        interval: u32,
        times_seen: u32,
        times_correct: u32,
        times_incorrect: u32,
    ) -> CardProgress {
        CardProgress {
            card_id: "card".to_string(),
            state,
            interval,
            ease_factor: 2.5,
            next_due_at: 1_700_432_000_000,
            learning_step_index: matches!(state, CardState::Learning | CardState::Relearning)
                .then_some(0),
            buried_until: None,
            suspended_at: None,
            times_seen,
            times_correct,
            times_incorrect,
            last_seen_at: 1_700_000_005_000,
            fsrs_stability: None,
            fsrs_difficulty: None,
            flag: None,
            marked_at: None,
        }
    }

    fn exported_native_review_kind(
        previous_progress: Option<CardProgress>,
        resulting_progress: CardProgress,
        rating: Rating,
    ) -> i64 {
        let mut state = AppState::default();
        state.decks.push(Deck {
            id: "deck".to_string(),
            name: "Spanish".to_string(),
            description: String::new(),
            created_at: 1_700_000_000_000,
        });
        state.cards.push(Card {
            id: "card".to_string(),
            deck_id: "deck".to_string(),
            front: "hola".to_string(),
            back: "hello".to_string(),
            created_at: 1_700_000_000_000,
            lineage: None,
        });
        state.card_progress.push(resulting_progress.clone());
        state.sessions.push(Session {
            id: "session".to_string(),
            deck_id: "deck".to_string(),
            status: SessionStatus::Completed,
            started_at: 1_700_000_000_000,
            ended_at: Some(1_700_000_005_000),
            cards_reviewed: 1,
            cards_correct: u32::from(rating != Rating::Again),
        });
        state.reviews.push(Review {
            id: "1700000005000".to_string(),
            session_id: "session".to_string(),
            card_id: "card".to_string(),
            rating,
            reviewed_at: 1_700_000_005_000,
            answer_time_ms: Some(987),
            leech_event: None,
            previous_progress,
            resulting_progress: Some(resulting_progress),
            previous_active_session: None,
            sibling_progress_snapshots: Vec::new(),
        });

        let collection = parse_v11_collection_bytes(
            &write_v11_collection_bytes_from_engram_state(&state).unwrap(),
        )
        .unwrap();
        collection.reviews[0].kind
    }

    #[test]
    fn native_review_transitions_export_anki_revlog_kinds_from_starting_state() {
        let new_graduation = native_review_progress(CardState::Review, 4, 1, 1, 0);
        assert_eq!(
            exported_native_review_kind(None, new_graduation, Rating::Easy),
            0,
            "a first native graduation review should export as an Anki learning row"
        );

        let previous_relearning = native_review_progress(CardState::Relearning, 0, 4, 2, 2);
        let relearning_graduation = native_review_progress(CardState::Review, 3, 5, 3, 2);
        assert_eq!(
            exported_native_review_kind(
                Some(previous_relearning),
                relearning_graduation,
                Rating::Good,
            ),
            2,
            "a relearning step that graduates should export as an Anki relearning row"
        );

        let previous_review = native_review_progress(CardState::Review, 7, 5, 4, 1);
        let review_lapse = native_review_progress(CardState::Relearning, 0, 6, 4, 2);
        assert_eq!(
            exported_native_review_kind(Some(previous_review), review_lapse, Rating::Again),
            1,
            "a review card that lapses should still export as an Anki review row"
        );
    }

    #[test]
    fn writes_standalone_cards_as_synthetic_basic_notes() {
        let state = AppState {
            decks: vec![Deck {
                id: "language".to_string(),
                name: "Tamil".to_string(),
                description: String::new(),
                created_at: 1_700_000_000_000,
            }],
            note_types: Vec::new(),
            notes: Vec::new(),
            cards: vec![Card {
                id: "card-alpha".to_string(),
                deck_id: "language".to_string(),
                front: "amma".to_string(),
                back: "mother".to_string(),
                created_at: 1_700_000_000_000,
                lineage: None,
            }],
            card_progress: Vec::new(),
            sessions: Vec::new(),
            reviews: Vec::new(),
            deck_options: Vec::new(),
            external_sources: Vec::new(),
            media_assets: Vec::new(),
            active_session: None,
        };

        let collection = parse_v11_collection_bytes(
            &write_v11_collection_bytes_from_engram_state(&state).unwrap(),
        )
        .unwrap();

        assert_eq!(collection.decks[0].name, "Tamil");
        assert_eq!(collection.note_types[0].name, "Engram Basic");
        assert_eq!(collection.note_types[0].fields[0].name, "Front");
        assert_eq!(collection.notes[0].field_values, vec!["amma", "mother"]);
        assert_eq!(collection.cards[0].queue, 0);
        assert_eq!(collection.cards[0].due, 1);

        let imported = read_v11_collection_as_engram_state(
            &write_legacy_apkg_from_engram_state(&state, &[]).unwrap(),
        )
        .unwrap();
        assert_eq!(imported.cards[0].front, "amma");
        assert_eq!(imported.cards[0].back, "mother");
        assert!(imported.card_progress.is_empty());
    }

    #[test]
    fn v11_collection_reader_accepts_modern_zstd_envelope() {
        let sqlite = v11_sqlite_collection_bytes();
        let modern = modern_package(&sqlite, &[]);

        let collection = read_v11_collection(&modern).unwrap();

        assert_eq!(collection.metadata.version, 11);
        assert_eq!(collection.decks[1].name, "Spanish::Latin");
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
    fn inspects_and_reads_modern_zstd_media_entries() {
        let apkg = modern_package(
            &v11_sqlite_collection_bytes(),
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
        assert_eq!(manifest.collection.name, SQLITE_21B_COLLECTION);
        assert_eq!(manifest.media.map_present, true);
        assert_eq!(manifest.media.mapping["0"], "audio/hola.mp3");
        assert_eq!(manifest.media.mapping["1"], "images/card.png");
        assert!(manifest.media.missing_files.is_empty());
        assert!(manifest.media.unmapped_files.is_empty());
        assert_eq!(manifest.media.media_files[0].size, 3);
        let expected_mp3_sha1 = bytes_to_lower_hex(&sum1(b"mp3"));
        let expected_png_sha1 = bytes_to_lower_hex(&sum1(b"png"));
        assert_eq!(
            manifest.media.media_files[0].sha1.as_deref(),
            Some(expected_mp3_sha1.as_str())
        );
        assert_eq!(
            manifest.media.media_files[1].sha1.as_deref(),
            Some(expected_png_sha1.as_str())
        );
        assert_eq!(manifest.media.media_files[0].legacy_zip_filename, Some(0));
        assert_eq!(manifest.media.media_files[1].legacy_zip_filename, Some(1));

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

        let state = read_v11_collection_as_engram_state(&apkg).unwrap();
        assert_eq!(state.decks[1].name, "Spanish::Latin");
        assert_eq!(state.media_assets.len(), 2);
        assert_eq!(state.media_assets[0].archive_name, "0");
        assert_eq!(
            state.media_assets[0].filename.as_deref(),
            Some("audio/hola.mp3")
        );
        assert_eq!(state.media_assets[0].data, b"mp3");
    }

    #[test]
    fn reads_modern_media_payloads_via_legacy_zip_filename() {
        let mut writer = ZipWriter::new();
        let meta = PackageMetadataProto {
            version: PackageVersionProto::Latest as i32,
        }
        .encode_pb();
        writer.add_file(META, &meta, false);
        writer.add_file(
            SQLITE_21B_COLLECTION,
            &zstd_encode(&v11_sqlite_collection_bytes()),
            false,
        );
        writer.add_file(LEGACY_COLLECTION, b"dummy legacy collection", false);

        let media_entries = MediaEntriesProto {
            entries: vec![MediaEntryProto {
                name: "audio/hola.mp3".to_string(),
                size: 3,
                sha1: sum1(b"mp3").to_vec(),
                legacy_zip_filename: Some(7),
            }],
        };
        let media_map = media_entries.encode_pb();
        writer.add_file(MEDIA_MAP, &zstd_encode(&media_map), false);
        writer.add_file("7", &zstd_encode(b"mp3"), false);
        let apkg = writer.finish();

        let manifest = inspect_apkg(&apkg).unwrap();
        assert_eq!(manifest.media.mapping["0"], "audio/hola.mp3");
        assert!(manifest.media.missing_files.is_empty());
        assert!(manifest.media.unmapped_files.is_empty());
        assert_eq!(manifest.media.media_files[0].archive_name, "0");
        assert_eq!(manifest.media.media_files[0].legacy_zip_filename, Some(7));

        let media_files = read_media_files(&apkg).unwrap();
        assert_eq!(
            media_files,
            vec![ResolvedMediaFile {
                archive_name: "0".to_string(),
                filename: Some("audio/hola.mp3".to_string()),
                data: b"mp3".to_vec(),
            }]
        );

        let logical = read_media_file(&apkg, "0").unwrap();
        assert_eq!(logical.data, b"mp3");
        let legacy = read_media_file(&apkg, "7").unwrap();
        assert_eq!(legacy.archive_name, "0");
        assert_eq!(legacy.data, b"mp3");

        let state = read_v11_collection_as_engram_state(&apkg).unwrap();
        assert_eq!(state.media_assets.len(), 1);
        assert_eq!(state.media_assets[0].archive_name, "0");
        assert_eq!(state.media_assets[0].data, b"mp3");
    }

    #[test]
    fn imports_and_exports_state_media_assets() {
        let sqlite = v11_sqlite_collection_bytes();
        let apkg = write_legacy_apkg(
            &sqlite,
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

        let state = read_v11_collection_as_engram_state(&apkg).unwrap();

        assert_eq!(state.media_assets.len(), 2);
        assert_eq!(state.media_assets[0].archive_name, "0");
        assert_eq!(
            state.media_assets[0].filename.as_deref(),
            Some("audio/hola.mp3")
        );
        assert_eq!(state.media_assets[0].data, b"mp3");
        assert!(state.external_sources.iter().any(|source| {
            source.source == ANKI_V11_SOURCE
                && source.target == ExternalSourceTarget::Media
                && source.target_id == "anki-media:0"
                && source.original_id.as_deref() == Some("0")
                && source.data.get("archiveName").map(String::as_str) == Some("0")
                && source.data.get("filename").map(String::as_str) == Some("audio/hola.mp3")
        }));

        let exported = write_legacy_apkg_from_engram_state(&state, &[]).unwrap();
        let manifest = inspect_apkg(&exported).unwrap();
        assert_eq!(manifest.media.mapping["0"], "audio/hola.mp3");
        assert_eq!(manifest.media.mapping["1"], "images/card.png");

        let image = read_media_file(&exported, "1").unwrap();
        assert_eq!(image.filename.as_deref(), Some("images/card.png"));
        assert_eq!(image.data, b"png");
    }

    #[test]
    fn writes_modern_apkg_envelope_and_state_export() {
        let sqlite = v11_sqlite_collection_bytes();
        let media = [
            MediaAsset {
                filename: "audio/hola.mp3",
                data: b"mp3",
            },
            MediaAsset {
                filename: "images/card.png",
                data: b"png",
            },
        ];

        let modern = write_modern_apkg(&sqlite, &media).unwrap();
        let manifest = inspect_apkg(&modern).unwrap();
        assert_eq!(manifest.collection.name, SQLITE_21B_COLLECTION);
        assert_eq!(manifest.collection.format, CollectionFormat::Sqlite21b);
        assert_eq!(manifest.media.mapping["0"], "audio/hola.mp3");
        assert_eq!(manifest.media.mapping["1"], "images/card.png");
        assert!(manifest
            .entries
            .iter()
            .any(|entry| entry.name == META && entry.size > 0));
        assert!(manifest
            .entries
            .iter()
            .all(|entry| entry.name != LEGACY_COLLECTION));

        let collection = read_v11_collection(&modern).unwrap();
        assert_eq!(collection.decks[1].name, "Spanish::Latin");
        let audio = read_media_file(&modern, "0").unwrap();
        assert_eq!(audio.filename.as_deref(), Some("audio/hola.mp3"));
        assert_eq!(audio.data, b"mp3");

        let imported_state =
            read_v11_collection_as_engram_state(&write_legacy_apkg(&sqlite, &media)).unwrap();
        let exported = write_modern_apkg_from_engram_state(&imported_state, &[]).unwrap();
        let exported_manifest = inspect_apkg(&exported).unwrap();
        assert_eq!(
            exported_manifest.collection.format,
            CollectionFormat::Sqlite21b
        );
        assert_eq!(exported_manifest.media.mapping["1"], "images/card.png");
        let image = read_media_file(&exported, "1").unwrap();
        assert_eq!(image.data, b"png");
    }

    #[test]
    fn analyzes_referenced_missing_and_unreferenced_media() {
        let mut state = AppState::default();
        state.notes.push(Note {
            id: "note".to_string(),
            note_type_id: "basic".to_string(),
            deck_id: "deck".to_string(),
            fields: vec![NoteFieldValue {
                field_id: "front".to_string(),
                value:
                    "[sound:audio/hola.mp3] <img SRC = \"images/caps.png\"> <img src=missing-unquoted.png> <img src=\"missing.png\"> <video poster=\"video/poster.jpg\"></video> <source srcset=\"images/card@1x.png 1x, missing-srcset.png 2x\"> <object data='docs/root.pdf'></object> <div style=\"background-image:url(images/bg.png); mask:url(#fade)\"></div> <img src=\"data:image/png;base64,skip\">"
                        .to_string(),
            }],
            tags: Vec::new(),
            created_at: 0,
            updated_at: 0,
        });
        state.cards.push(Card {
            id: "card".to_string(),
            deck_id: "deck".to_string(),
            front: "<img data-src=\"ignored.png\"> <img Src='images/card.png'>".to_string(),
            back: "<style>.card{background:url('images/card-back.png')}</style> <img src = https://example.com/remote.png>".to_string(),
            created_at: 0,
            lineage: None,
        });
        state.media_assets = vec![
            MediaAssetRecord {
                id: "audio".to_string(),
                archive_name: "0".to_string(),
                filename: Some("audio/hola.mp3".to_string()),
                data: b"mp3".to_vec(),
            },
            MediaAssetRecord {
                id: "caps".to_string(),
                archive_name: "3".to_string(),
                filename: Some("images/caps.png".to_string()),
                data: b"caps".to_vec(),
            },
            MediaAssetRecord {
                id: "image".to_string(),
                archive_name: "1".to_string(),
                filename: Some("images/card.png".to_string()),
                data: b"png".to_vec(),
            },
            MediaAssetRecord {
                id: "unused".to_string(),
                archive_name: "2".to_string(),
                filename: Some("audio/unused.mp3".to_string()),
                data: b"unused".to_vec(),
            },
            MediaAssetRecord {
                id: "poster".to_string(),
                archive_name: "4".to_string(),
                filename: Some("video/poster.jpg".to_string()),
                data: b"poster".to_vec(),
            },
            MediaAssetRecord {
                id: "srcset".to_string(),
                archive_name: "5".to_string(),
                filename: Some("images/card@1x.png".to_string()),
                data: b"srcset".to_vec(),
            },
            MediaAssetRecord {
                id: "doc".to_string(),
                archive_name: "6".to_string(),
                filename: Some("docs/root.pdf".to_string()),
                data: b"doc".to_vec(),
            },
            MediaAssetRecord {
                id: "bg".to_string(),
                archive_name: "7".to_string(),
                filename: Some("images/bg.png".to_string()),
                data: b"bg".to_vec(),
            },
            MediaAssetRecord {
                id: "back-bg".to_string(),
                archive_name: "8".to_string(),
                filename: Some("images/card-back.png".to_string()),
                data: b"back".to_vec(),
            },
        ];

        let analysis = analyze_engram_media_references(&state);

        assert_eq!(
            analysis.referenced_filenames,
            vec![
                "audio/hola.mp3",
                "docs/root.pdf",
                "images/bg.png",
                "images/caps.png",
                "images/card-back.png",
                "images/card.png",
                "images/card@1x.png",
                "missing-srcset.png",
                "missing-unquoted.png",
                "missing.png",
                "video/poster.jpg"
            ]
        );
        assert_eq!(
            analysis.referenced_asset_ids,
            vec!["audio", "caps", "image", "poster", "srcset", "doc", "bg", "back-bg"]
        );
        assert_eq!(
            analysis.missing_filenames,
            vec!["missing-srcset.png", "missing-unquoted.png", "missing.png"]
        );
        assert_eq!(analysis.unreferenced_asset_ids, vec!["unused"]);
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
