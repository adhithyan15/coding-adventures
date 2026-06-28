//! APKG archive inspection for Engram.
//!
//! This crate intentionally stops at the archive boundary: it identifies the
//! Anki collection member and media mapping inside an `.apkg`/`.colpkg` zip
//! archive, but leaves SQLite collection import/export to a later layer.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;

use coding_adventures_sha1::sum1;
use engram_core::{
    render_template, render_template_with_front_side, AppState, Card, CardFlag, CardLineage,
    CardProgress, CardState, CardTemplate, Deck, DeckOptions, DeckOptionsPreset,
    ExternalSourceRecord, ExternalSourceTarget, FieldDef, MediaAssetRecord, Note, NoteFieldValue,
    NoteType, Rating, Review, Session, SessionStatus, INITIAL_EASE_FACTOR, ONE_DAY_MS,
};
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use zip::{ZipReader, ZipWriter};

const LEGACY_COLLECTION: &str = "collection.anki2";
const SQLITE_21_COLLECTION: &str = "collection.anki21";
const SQLITE_21B_COLLECTION: &str = "collection.anki21b";
const MEDIA_MAP: &str = "media";
const ANKI_V11_SOURCE: &str = "anki-v11";

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

pub fn write_v11_collection_bytes_from_engram_state(
    state: &AppState,
) -> Result<Vec<u8>, ApkgError> {
    let export = ExportModel::from_state(state)?;
    let sqlite_file = tempfile::NamedTempFile::new()
        .map_err(|err| apkg_error(format!("failed to create temporary SQLite file: {err}")))?;

    {
        let connection = Connection::open(sqlite_file.path()).map_err(|err| {
            apkg_error(format!(
                "failed to open temporary Anki V11 SQLite collection: {err}"
            ))
        })?;
        create_v11_export_schema(&connection)?;
        write_v11_export_rows(&connection, &export)?;
    }

    std::fs::read(sqlite_file.path()).map_err(|err| {
        apkg_error(format!(
            "failed to read exported Anki V11 collection: {err}"
        ))
    })
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

pub fn read_v11_collection_as_engram_state(data: &[u8]) -> Result<AppState, ApkgError> {
    let collection = read_v11_collection(data)?;
    let mut state = v11_collection_to_engram_state(&collection)?;
    state.media_assets = read_media_files(data)?
        .into_iter()
        .map(media_asset_record_from_resolved)
        .collect();
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

    let mut cards = Vec::with_capacity(collection.cards.len());
    for card in &collection.cards {
        cards.push(map_v11_card(
            card,
            &notes_by_id,
            &note_types_by_id,
            &anki_note_types_by_id,
        )?);
    }

    let last_reviewed_at_by_card = last_reviewed_at_by_card(&collection.reviews);
    let card_progress = collection
        .cards
        .iter()
        .filter_map(|card| {
            map_v11_card_progress(
                card,
                collection.metadata.created_at_days,
                &last_reviewed_at_by_card,
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
    let deck_options = v11_deck_options(collection);
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
        if let Some(multiplier) = json_path_f64(config, &["lapse", "mult"]) {
            options.lapse_interval_multiplier = if multiplier > 10.0 {
                multiplier / 100.0
            } else {
                multiplier
            };
        }
    }

    options
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
        insert_json(&mut data, "rawJson", &deck.raw, "Anki deck JSON")?;
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
        for card in &state.cards {
            if let Some(lineage) = card
                .lineage
                .as_ref()
                .filter(|lineage| lineage_is_exportable(lineage, &notes_by_id, &note_types_by_id))
            {
                cards.push(ExportCard {
                    key: card.id.clone(),
                    note_key: lineage.note_id.clone(),
                    deck_key: fallback_deck_key(&card.deck_id, &default_deck_key),
                    template_ordinal: lineage.ordinal,
                    created_at: card.created_at,
                });
            } else {
                let note_key = synthetic_basic_note_key(&card.id);
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

        let created_at_days = export_created_at_days(state, &decks, &notes, &cards);
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
                export.modified_at_seconds,
                export.modified_at_seconds,
                11_i64,
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
        let sort_field = fields.first().cloned().unwrap_or_default();
        let source = anki_source(export, ExternalSourceTarget::Note, &note.key);
        connection
            .execute(
                "INSERT INTO notes VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                rusqlite::params![
                    export.note_ids[&note.key],
                    source_string(source, "guid")
                        .unwrap_or_else(|| export_note_guid(&note.key, export.note_ids[&note.key])),
                    export.note_type_ids[&note.note_type_key],
                    millis_to_anki_seconds(note.updated_at.max(note.created_at)),
                    source_i64(source, "updateSequenceNumber").unwrap_or(-1_i64),
                    join_anki_tags(&note.tags),
                    field_join,
                    sort_field,
                    export_note_checksum(source, &fields.first().cloned().unwrap_or_default()),
                    source_i64(source, "flags").unwrap_or_default(),
                    source_string(source, "data").unwrap_or_default(),
                ],
            )
            .map_err(|err| apkg_error(format!("failed to write Anki note {}: {err}", note.key)))?;
    }

    for (index, card) in export.cards.iter().enumerate() {
        let progress = export.progress_by_card.get(&card.key);
        let source = anki_source(export, ExternalSourceTarget::Card, &card.key);
        let scheduling = export_card_scheduling(progress, export.created_at_days, index, source);
        connection
            .execute(
                "INSERT INTO cards VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
                rusqlite::params![
                    export.card_ids[&card.key],
                    export.note_ids[&card.note_key],
                    export.deck_ids[&card.deck_key],
                    i64::from(card.template_ordinal),
                    export_card_modified_at(card, progress),
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
                    source_i64(source, "originalDeckId").unwrap_or_default(),
                    scheduling.flags,
                    source_string(source, "data").unwrap_or_default(),
                ],
            )
            .map_err(|err| apkg_error(format!("failed to write Anki card {}: {err}", card.key)))?;
    }

    let mut used_review_ids = BTreeSet::new();
    for review in &export.reviews {
        let Some(card_id) = export.card_ids.get(&review.card_id) else {
            return Err(apkg_error(format!(
                "Engram review {} references missing card {}",
                review.id, review.card_id
            )));
        };
        let review_id = unique_review_id(review, &mut used_review_ids);
        connection
            .execute(
                "INSERT INTO revlog VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    review_id,
                    card_id,
                    -1_i64,
                    rating_to_v11_ease(review.rating),
                    review
                        .resulting_progress
                        .as_ref()
                        .map(|progress| i64::from(progress.interval))
                        .unwrap_or_default(),
                    review
                        .previous_progress
                        .as_ref()
                        .map(|progress| i64::from(progress.interval))
                        .unwrap_or_default(),
                    review
                        .resulting_progress
                        .as_ref()
                        .map(progress_factor_to_anki)
                        .unwrap_or((INITIAL_EASE_FACTOR * 1000.0).round() as i64),
                    0_i64,
                    review_kind(review),
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
        let mut deck_json =
            anki_source_json(export, ExternalSourceTarget::Deck, &deck.key, "rawJson")
                .unwrap_or_else(|| serde_json::json!({}));
        let deck_object = ensure_json_object(&mut deck_json);
        deck_object.insert("id".to_string(), Value::Number(id.into()));
        deck_object.insert("name".to_string(), Value::String(deck.name.clone()));
        deck_object.insert("desc".to_string(), Value::String(deck.description.clone()));
        deck_object.insert(
            "mod".to_string(),
            Value::Number(millis_to_anki_seconds(deck.created_at).into()),
        );
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
        object.insert(id.to_string(), deck_json);
    }
    Value::Object(object)
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
        let templates = export_note_type_templates_json(note_type, &raw_templates);
        let model_object = ensure_json_object(&mut model_json);
        model_object.insert("id".to_string(), Value::Number(id.into()));
        model_object.insert("name".to_string(), Value::String(note_type.name.clone()));
        model_object.insert("type".to_string(), Value::Number(note_type.kind.into()));
        model_object.insert(
            "mod".to_string(),
            Value::Number(
                millis_to_anki_seconds(note_type.updated_at.max(note_type.created_at)).into(),
            ),
        );
        model_object
            .entry("usn".to_string())
            .or_insert_with(|| Value::Number((-1_i64).into()));
        model_object
            .entry("sortf".to_string())
            .or_insert_with(|| Value::Number(0_i64.into()));
        model_object.entry("did".to_string()).or_insert(Value::Null);
        model_object.entry("css".to_string()).or_insert_with(|| {
            Value::String(
                ".card { font-family: arial; font-size: 20px; text-align: center; color: black; background-color: white; }"
                    .to_string(),
            )
        });
        model_object
            .entry("latexPre".to_string())
            .or_insert_with(|| Value::String("\\documentclass[12pt]{article}".to_string()));
        model_object
            .entry("latexPost".to_string())
            .or_insert_with(|| Value::String("\\end{document}".to_string()));
        model_object.insert("flds".to_string(), Value::Array(fields));
        model_object.insert("tmpls".to_string(), Value::Array(templates));
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
    export_collection_json(export, "gravesJson")
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default()
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
    object.insert("new".to_string(), new_section);

    let mut review_section = object
        .get("rev")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    ensure_json_object(&mut review_section).insert(
        "perDay".to_string(),
        Value::Number(i64::from(options.reviews_per_day).into()),
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
        serde_json::Number::from_f64(options.lapse_interval_multiplier)
            .map(Value::Number)
            .unwrap_or_else(|| Value::Number(0_i64.into())),
    );
    object.insert("lapse".to_string(), lapse_section);
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
            object.entry("did".to_string()).or_insert(Value::Null);
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

fn source_json(source: Option<&ExternalSourceRecord>, key: &str) -> Option<Value> {
    source
        .and_then(|source| source.data.get(key))
        .and_then(|value| serde_json::from_str(value).ok())
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
) -> ExportCardScheduling {
    let Some(progress) = progress else {
        return ExportCardScheduling {
            kind: source_i64(source, "kind").unwrap_or(0),
            queue: source_i64(source, "queue").unwrap_or(0),
            due: source_i64(source, "due").unwrap_or(index.saturating_add(1) as i64),
            interval: source_i64(source, "interval").unwrap_or(0),
            factor: source_i64(source, "factor")
                .unwrap_or((INITIAL_EASE_FACTOR * 1000.0).round() as i64),
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
        CardState::Learning => (1, 1, millis_to_anki_seconds(progress.next_due_at).max(1)),
        CardState::Relearning => (3, 1, millis_to_anki_seconds(progress.next_due_at).max(1)),
        CardState::Suspended => (
            review_or_new_kind(progress),
            -1,
            millis_to_anki_due_day(collection_created_at_days, progress.next_due_at),
        ),
        CardState::Buried => (
            review_or_new_kind(progress),
            -2,
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
        left: progress
            .learning_step_index
            .map(i64::from)
            .unwrap_or_else(|| source_i64(source, "left").unwrap_or_default()),
        flags: progress
            .flag
            .map(card_flag_to_anki)
            .or_else(|| source_i64(source, "flags"))
            .unwrap_or_default(),
    }
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

fn export_card_modified_at(card: &ExportCard, progress: Option<&CardProgress>) -> i64 {
    let timestamp = progress
        .map(|progress| progress.last_seen_at.max(card.created_at))
        .unwrap_or(card.created_at);
    millis_to_anki_seconds(timestamp)
}

fn rating_to_v11_ease(rating: Rating) -> i64 {
    match rating {
        Rating::Again => 1,
        Rating::Hard => 2,
        Rating::Good => 3,
        Rating::Easy => 4,
    }
}

fn review_kind(review: &Review) -> i64 {
    match review
        .resulting_progress
        .as_ref()
        .map(|progress| progress.state)
    {
        Some(CardState::Learning) => 0,
        Some(CardState::Relearning) => 2,
        _ => 1,
    }
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
        template.front_template.contains("{{cloze:") || template.back_template.contains("{{cloze:")
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
            required_field_names: vec!["Front".to_string()],
            ordinal: 0,
        }],
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
        .map(|template| CardTemplate {
            id: template_id(note_type.id, template.ordinal),
            name: template.name.clone(),
            front_template: template.question_format.clone(),
            back_template: template.answer_format.clone(),
            required_field_names: required_field_names_for_anki_template(note_type, template),
            ordinal: i64_to_u32(template.ordinal),
        })
        .collect();

    NoteType {
        id,
        name: note_type.name.clone(),
        fields,
        templates,
        created_at: 0,
        updated_at: 0,
    }
}

fn required_field_names_for_anki_template(
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
    if tag == "FrontSide" || tag.is_empty() {
        return None;
    }

    Some(
        tag.strip_prefix("cloze:")
            .or_else(|| tag.strip_prefix("hint:"))
            .or_else(|| tag.strip_prefix("type:"))
            .unwrap_or(tag)
            .trim(),
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
    let template = note_type
        .templates
        .iter()
        .find(|template| template.ordinal == i64_to_u32(card.ordinal))
        .ok_or_else(|| {
            apkg_error(format!(
                "Anki card {} references missing template ordinal {}",
                card.id, card.ordinal
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
    let field_values = field_value_map(note, anki_note_type);
    let cloze_ordinal = if anki_note_type.kind == 1 {
        Some(i64_to_u32(card.ordinal.saturating_add(1)))
    } else {
        None
    };
    let (front, back) = if let Some(cloze_ordinal) = cloze_ordinal {
        (
            render_anki_cloze_template(
                &template.front_template,
                &field_values,
                cloze_ordinal,
                AnkiClozeSide::Question,
            ),
            render_anki_cloze_template(
                &template.back_template,
                &field_values,
                cloze_ordinal,
                AnkiClozeSide::Answer,
            ),
        )
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

fn card_note_type_id(note: &Note) -> i64 {
    note.note_type_id.parse().unwrap_or_default()
}

fn map_v11_card_progress(
    card: &AnkiV11Card,
    collection_created_at_days: i64,
    last_reviewed_at_by_card: &BTreeMap<i64, u64>,
) -> Option<CardProgress> {
    if card.queue == 0 {
        return anki_card_flag(card.flags).map(|flag| new_card_metadata_overlay(card, flag));
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
        learning_step_index: if matches!(state, CardState::Learning | CardState::Relearning)
            && card.left > 0
        {
            Some(i64_to_u32(card.left))
        } else {
            None
        },
        buried_until: (state == CardState::Buried).then_some(next_due_at),
        suspended_at: (state == CardState::Suspended)
            .then_some(anki_seconds_to_millis(card.modified_at)),
        times_seen: i64_to_u32(card.repetitions),
        times_correct: i64_to_u32(card.repetitions.saturating_sub(card.lapses)),
        times_incorrect: i64_to_u32(card.lapses),
        last_seen_at,
        flag: anki_card_flag(card.flags),
        marked_at: None,
    })
}

fn new_card_metadata_overlay(card: &AnkiV11Card, flag: CardFlag) -> CardProgress {
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
        flag: Some(flag),
        marked_at: None,
    }
}

fn map_v11_review(review: &AnkiV11Review, deck_id: &str) -> Review {
    Review {
        id: review.id.to_string(),
        session_id: import_session_id(deck_id),
        card_id: review.card_id.to_string(),
        rating: rating_from_v11_ease(review.ease),
        reviewed_at: i64_to_u64(review.id),
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
                sort_field: sqlite_value_to_string(row, 7)?,
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

fn json_path_f64(value: &Value, path: &[&str]) -> Option<f64> {
    json_path(value, path).and_then(Value::as_f64)
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

fn split_anki_fields(fields: &str) -> Vec<String> {
    fields.split('\u{1f}').map(str::to_string).collect()
}

fn split_anki_tags(tags: &str) -> Vec<String> {
    tags.split_whitespace().map(str::to_string).collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AnkiClozeSide {
    Question,
    Answer,
}

fn render_anki_cloze_template(
    template: &str,
    field_values: &HashMap<String, String>,
    cloze_ordinal: u32,
    side: AnkiClozeSide,
) -> String {
    let mut rendered = String::with_capacity(template.len());
    let mut rest = template;

    while let Some(start) = rest.find("{{") {
        let (prefix, after_start) = rest.split_at(start);
        rendered.push_str(prefix);
        let after_start = &after_start[2..];

        match after_start.find("}}") {
            Some(end) => {
                let (field_name, after_end) = after_start.split_at(end);
                let field_name = field_name.trim();
                if let Some(field_name) = field_name.strip_prefix("cloze:") {
                    if let Some(value) = field_values.get(field_name.trim()) {
                        rendered.push_str(&render_anki_cloze_text(value, cloze_ordinal, side));
                    }
                } else if let Some(value) = field_values.get(field_name) {
                    rendered.push_str(value);
                }
                rest = &after_end[2..];
            }
            None => {
                rendered.push_str("{{");
                rendered.push_str(after_start);
                rest = "";
            }
        }
    }

    rendered.push_str(rest);
    rendered
}

fn render_anki_cloze_text(value: &str, cloze_ordinal: u32, side: AnkiClozeSide) -> String {
    let mut rendered = String::with_capacity(value.len());
    let mut rest = value;

    while let Some(start) = rest.find("{{c") {
        let (prefix, candidate) = rest.split_at(start);
        rendered.push_str(prefix);

        if let Some(marker) = parse_anki_cloze_marker(candidate) {
            if side == AnkiClozeSide::Question && marker.ordinal == cloze_ordinal {
                match marker.hint.map(str::trim).filter(|hint| !hint.is_empty()) {
                    Some(hint) => {
                        rendered.push('[');
                        rendered.push_str(hint);
                        rendered.push(']');
                    }
                    None => rendered.push_str("[...]"),
                }
            } else {
                rendered.push_str(&render_anki_cloze_text(
                    marker.hidden,
                    cloze_ordinal,
                    AnkiClozeSide::Answer,
                ));
            }
            rest = &candidate[marker.consumed..];
        } else {
            rendered.push_str("{{c");
            rest = &candidate[3..];
        }
    }

    rendered.push_str(rest);
    rendered
}

#[derive(Debug, PartialEq, Eq)]
struct AnkiClozeMarker<'a> {
    ordinal: u32,
    hidden: &'a str,
    hint: Option<&'a str>,
    consumed: usize,
}

fn parse_anki_cloze_marker(candidate: &str) -> Option<AnkiClozeMarker<'_>> {
    if !candidate.starts_with("{{c") {
        return None;
    }

    let after_prefix = &candidate[3..];
    let digit_len = after_prefix
        .find(|ch: char| !ch.is_ascii_digit())
        .unwrap_or(after_prefix.len());
    if digit_len == 0 {
        return None;
    }
    let ordinal: u32 = after_prefix[..digit_len].parse().ok()?;
    if ordinal == 0 {
        return None;
    }
    let after_digits = &after_prefix[digit_len..];
    if !after_digits.starts_with("::") {
        return None;
    }
    let marker_body = &after_digits[2..];
    let end = marker_body.find("}}")?;
    let body = &marker_body[..end];
    let consumed = 3 + digit_len + 2 + end + 2;

    let (hidden, hint) = match body.split_once("::") {
        Some((hidden, hint)) => (hidden, Some(hint)),
        None => (body, None),
    };

    Some(AnkiClozeMarker {
        ordinal,
        hidden,
        hint,
        consumed,
    })
}

fn sqlite_value_to_string(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<String> {
    use rusqlite::types::ValueRef;

    match row.get_ref(index)? {
        ValueRef::Null => Ok(String::new()),
        ValueRef::Integer(value) => Ok(value.to_string()),
        ValueRef::Real(value) => Ok(value.to_string()),
        ValueRef::Text(value) => Ok(String::from_utf8_lossy(value).into_owned()),
        ValueRef::Blob(value) => Ok(String::from_utf8_lossy(value).into_owned()),
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
                                "new": {"perDay": 12, "delays": [3, 12], "ints": [2, 5]},
                                "rev": {"perDay": 80},
                                "lapse": {"delays": [20], "mult": 0.5}
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
        assert_eq!(options.lapse_interval_multiplier, 0.5);

        assert_eq!(state.note_types.len(), 1);
        let note_type = &state.note_types[0];
        assert_eq!(note_type.id, "100");
        assert_eq!(note_type.fields[0].id, "100:field:0");
        assert_eq!(note_type.fields[0].name, "Front");
        assert_eq!(note_type.templates[0].id, "100:template:0");

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
        assert_eq!(progress.flag, Some(CardFlag::Blue));

        assert_eq!(state.reviews.len(), 1);
        assert_eq!(state.reviews[0].id, "3000");
        assert_eq!(state.reviews[0].session_id, "anki-import:2");
        assert_eq!(state.reviews[0].rating, Rating::Good);
        assert_eq!(state.reviews[0].reviewed_at, 3000);
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
        assert_eq!(exported.metadata.deck_config["2"]["lapse"]["mult"], 0.5);

        assert_eq!(state.sessions.len(), 1);
        assert_eq!(state.sessions[0].id, "anki-import:2");
        assert_eq!(state.sessions[0].deck_id, "2");
        assert_eq!(state.sessions[0].status, SessionStatus::Completed);
        assert_eq!(state.sessions[0].cards_reviewed, 1);
        assert_eq!(state.sessions[0].cards_correct, 1);
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
        assert_eq!(state.cards[0].back, "hola<hr>hello");
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
    fn imported_v11_source_metadata_round_trips_on_export() {
        let mut collection = parse_v11_collection_bytes(&v11_sqlite_collection_bytes()).unwrap();
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
            "dyn": 1,
            "extendNew": 25,
            "extendRev": 75,
            "collapsed": true,
        });
        collection.note_types[0].raw = serde_json::json!({
            "id": 100,
            "name": "Basic",
            "type": 0,
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
        collection.notes[0].guid = "stable-guid".to_string();
        collection.notes[0].update_sequence_number = 17;
        collection.notes[0].checksum = 4242;
        collection.notes[0].flags = 5;
        collection.notes[0].data = "note-data".to_string();
        collection.cards[0].update_sequence_number = 23;
        collection.cards[0].original_due = 777;
        collection.cards[0].original_deck_id = 1;
        collection.cards[0].data = "card-data".to_string();
        collection.graves = vec![AnkiV11Grave {
            update_sequence_number: 31,
            object_id: 9001,
            kind: 2,
        }];

        let state = v11_collection_to_engram_state(&collection).unwrap();
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
        assert_eq!(exported.metadata.deck_config["2"]["name"], "Story defaults");
        assert_eq!(exported.metadata.tags["imported"], 2);
        assert_eq!(exported.graves[0].object_id, 9001);
        assert_eq!(exported.graves[0].kind, 2);

        let deck = exported.decks.iter().find(|deck| deck.id == 2).unwrap();
        assert_eq!(deck.raw["conf"], 7);
        assert_eq!(deck.raw["dyn"], 1);
        assert_eq!(deck.raw["collapsed"], true);
        assert_eq!(deck.name, "Spanish::Latin");

        let note_type = &exported.note_types[0];
        assert_eq!(note_type.css, ".card { color: teal; }");
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
        assert_eq!(note.update_sequence_number, 17);
        assert_eq!(note.checksum, 4242);
        assert_eq!(note.flags, 5);
        assert_eq!(note.data, "note-data");

        let card = &exported.cards[0];
        assert_eq!(card.update_sequence_number, 23);
        assert_eq!(card.original_due, 777);
        assert_eq!(card.original_deck_id, 1);
        assert_eq!(card.data, "card-data");
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
                    question_format: "{{cloze:Text}}".to_string(),
                    answer_format: "{{cloze:Text}}<hr>{{Extra}}".to_string(),
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
                    "The word {{c1::night::old root}} travels.".to_string(),
                    "Proto-Indo-European stories go here.".to_string(),
                ],
                sort_field: "The word night travels.".to_string(),
                checksum: 0,
                flags: 0,
                data: String::new(),
            }],
            cards: vec![AnkiV11Card {
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
            }],
            reviews: Vec::new(),
            graves: Vec::new(),
        };

        let state = v11_collection_to_engram_state(&collection).unwrap();

        assert_eq!(state.cards[0].front, "The word [old root] travels.");
        assert_eq!(
            state.cards[0].back,
            "The word night travels.<hr>Proto-Indo-European stories go here."
        );
        let lineage = state.cards[0].lineage.as_ref().unwrap();
        assert_eq!(lineage.cloze_ordinal, Some(1));
        assert!(state.card_progress.is_empty());
    }

    #[test]
    fn reads_v11_apkg_directly_as_engram_app_state() {
        let sqlite = v11_sqlite_collection_bytes();
        let apkg = write_legacy_apkg(&sqlite, &[]);

        let state = read_v11_collection_as_engram_state(&apkg).unwrap();

        assert_eq!(state.cards[0].front, "hola");
        assert_eq!(state.cards[0].back, "hello");
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
                    required_field_names: vec!["Front".to_string()],
                    ordinal: 0,
                }],
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
        assert_eq!(collection.notes[0].id, 1000);
        assert_eq!(collection.notes[0].tags, vec!["spanish", "roots"]);
        assert_eq!(collection.cards[0].id, 2000);
        assert_eq!(collection.cards[0].queue, 2);
        assert_eq!(collection.cards[0].interval, 7);
        assert_eq!(collection.cards[0].factor, 2500);
        assert_eq!(collection.cards[0].flags, 4);
        assert_eq!(collection.reviews[0].card_id, 2000);
        assert_eq!(collection.reviews[0].ease, 3);

        let apkg = write_legacy_apkg_from_engram_state(&state, &[]).unwrap();
        let imported = read_v11_collection_as_engram_state(&apkg).unwrap();

        assert_eq!(imported.decks[0].name, "Spanish::Latin");
        assert_eq!(imported.notes[0].fields[0].value, "hola");
        assert_eq!(imported.cards[0].front, "hola");
        assert_eq!(imported.cards[0].back, "hello");
        assert_eq!(imported.card_progress[0].interval, 7);
        assert_eq!(imported.card_progress[0].flag, Some(CardFlag::Blue));
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

        let exported = write_legacy_apkg_from_engram_state(&state, &[]).unwrap();
        let manifest = inspect_apkg(&exported).unwrap();
        assert_eq!(manifest.media.mapping["0"], "audio/hola.mp3");
        assert_eq!(manifest.media.mapping["1"], "images/card.png");

        let image = read_media_file(&exported, "1").unwrap();
        assert_eq!(image.filename.as_deref(), Some("images/card.png"));
        assert_eq!(image.data, b"png");
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
