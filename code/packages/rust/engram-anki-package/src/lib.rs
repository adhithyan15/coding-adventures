//! APKG archive inspection for Engram.
//!
//! This crate intentionally stops at the archive boundary: it identifies the
//! Anki collection member and media mapping inside an `.apkg`/`.colpkg` zip
//! archive, but leaves SQLite collection import/export to a later layer.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};
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
