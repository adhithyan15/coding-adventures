use std::collections::BTreeSet;

use crate::model::{AppState, MediaAssetRecord};

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct EngramMediaReferenceAnalysis {
    pub referenced_filenames: Vec<String>,
    pub referenced_asset_ids: Vec<String>,
    pub missing_filenames: Vec<String>,
    pub unreferenced_asset_ids: Vec<String>,
}

pub fn analyze_media_references(state: &AppState) -> EngramMediaReferenceAnalysis {
    let mut referenced = BTreeSet::new();
    for note in &state.notes {
        for field in &note.fields {
            collect_media_references_from_text(&field.value, &mut referenced);
        }
    }
    for card in &state.cards {
        collect_media_references_from_text(&card.front, &mut referenced);
        collect_media_references_from_text(&card.back, &mut referenced);
    }

    let referenced_filenames = referenced.iter().cloned().collect::<Vec<_>>();
    let referenced_asset_ids = state
        .media_assets
        .iter()
        .filter(|asset| {
            referenced
                .iter()
                .any(|filename| media_asset_matches_filename(asset, filename))
        })
        .map(|asset| asset.id.clone())
        .collect::<Vec<_>>();
    let unreferenced_asset_ids = state
        .media_assets
        .iter()
        .filter(|asset| {
            !referenced
                .iter()
                .any(|filename| media_asset_matches_filename(asset, filename))
        })
        .map(|asset| asset.id.clone())
        .collect::<Vec<_>>();
    let missing_filenames = referenced
        .iter()
        .filter(|filename| {
            !state
                .media_assets
                .iter()
                .any(|asset| media_asset_matches_filename(asset, filename))
        })
        .cloned()
        .collect();

    EngramMediaReferenceAnalysis {
        referenced_filenames,
        referenced_asset_ids,
        missing_filenames,
        unreferenced_asset_ids,
    }
}

fn media_asset_matches_filename(asset: &MediaAssetRecord, filename: &str) -> bool {
    asset.filename.as_deref() == Some(filename) || asset.archive_name == filename
}

fn collect_media_references_from_text(text: &str, references: &mut BTreeSet<String>) {
    collect_sound_markers(text, references);
    collect_media_attributes(text, references);
    collect_css_urls(text, references);
}

fn collect_sound_markers(text: &str, references: &mut BTreeSet<String>) {
    let mut rest = text;
    while let Some(start) = rest.find("[sound:") {
        rest = &rest[start + "[sound:".len()..];
        let Some(end) = rest.find(']') else {
            break;
        };
        maybe_insert_media_reference(&rest[..end], references);
        rest = &rest[end + 1..];
    }
}

fn collect_media_attributes(text: &str, references: &mut BTreeSet<String>) {
    for attribute in ["src", "poster", "data", "srcset"] {
        collect_media_attribute(text, attribute, references);
    }
}

fn collect_media_attribute(text: &str, attribute: &str, references: &mut BTreeSet<String>) {
    let bytes = text.as_bytes();
    let attribute_bytes = attribute.as_bytes();
    let mut index = 0;
    while index + attribute_bytes.len() <= bytes.len() {
        if !bytes[index..index + attribute_bytes.len()].eq_ignore_ascii_case(attribute_bytes)
            || !is_html_attr_boundary(bytes.get(index.wrapping_sub(1)).copied())
        {
            index += 1;
            continue;
        }

        let mut cursor = index + attribute_bytes.len();
        while bytes
            .get(cursor)
            .is_some_and(|byte| byte.is_ascii_whitespace())
        {
            cursor += 1;
        }
        if bytes.get(cursor) != Some(&b'=') {
            index += attribute_bytes.len();
            continue;
        }
        cursor += 1;
        while bytes
            .get(cursor)
            .is_some_and(|byte| byte.is_ascii_whitespace())
        {
            cursor += 1;
        }

        let Some(first) = bytes.get(cursor).copied() else {
            break;
        };
        let (value_start, value_end) = if first == b'"' || first == b'\'' {
            cursor += 1;
            let terminator = first;
            let start = cursor;
            while bytes.get(cursor).is_some_and(|byte| *byte != terminator) {
                cursor += 1;
            }
            (start, cursor)
        } else {
            let start = cursor;
            while bytes
                .get(cursor)
                .is_some_and(|byte| !byte.is_ascii_whitespace() && *byte != b'>')
            {
                cursor += 1;
            }
            (start, cursor)
        };

        if let Some(value) = text.get(value_start..value_end) {
            if attribute.eq_ignore_ascii_case("srcset") {
                collect_srcset_references(value, references);
            } else {
                maybe_insert_media_reference(value, references);
            }
        }
        index = cursor.saturating_add(1);
    }
}

fn collect_srcset_references(value: &str, references: &mut BTreeSet<String>) {
    for candidate in value.split(',') {
        if let Some(url) = candidate.split_whitespace().next() {
            maybe_insert_media_reference(url, references);
        }
    }
}

fn collect_css_urls(text: &str, references: &mut BTreeSet<String>) {
    let bytes = text.as_bytes();
    let mut index = 0;
    while index + 3 <= bytes.len() {
        if !bytes[index..index + 3].eq_ignore_ascii_case(b"url")
            || bytes
                .get(index.wrapping_sub(1))
                .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'-' || *byte == b'_')
        {
            index += 1;
            continue;
        }

        let mut cursor = index + 3;
        while bytes
            .get(cursor)
            .is_some_and(|byte| byte.is_ascii_whitespace())
        {
            cursor += 1;
        }
        if bytes.get(cursor) != Some(&b'(') {
            index += 3;
            continue;
        }
        cursor += 1;
        let value_start = cursor;
        while bytes.get(cursor).is_some_and(|byte| *byte != b')') {
            cursor += 1;
        }
        if let Some(value) = text.get(value_start..cursor) {
            maybe_insert_media_reference(trim_wrapping_quotes(value), references);
        }
        index = cursor.saturating_add(1);
    }
}

fn trim_wrapping_quotes(value: &str) -> &str {
    let value = value.trim();
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        if (bytes[0] == b'"' && bytes[value.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[value.len() - 1] == b'\'')
        {
            return &value[1..value.len() - 1];
        }
    }
    value
}

fn is_html_attr_boundary(previous: Option<u8>) -> bool {
    previous.is_none_or(|byte| byte.is_ascii_whitespace() || byte == b'<')
}

fn maybe_insert_media_reference(value: &str, references: &mut BTreeSet<String>) {
    let value = value.trim();
    if value.is_empty()
        || value.starts_with("http://")
        || value.starts_with("https://")
        || value.starts_with("data:")
        || value.starts_with('#')
    {
        return;
    }
    references.insert(value.to_string());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Card, Note, NoteFieldValue};

    #[test]
    fn media_reference_analysis_tracks_html_sound_css_and_srcset() {
        let state = AppState {
            notes: vec![Note {
                id: "note".to_string(),
                note_type_id: "basic".to_string(),
                deck_id: "deck".to_string(),
                fields: vec![NoteFieldValue {
                    field_id: "front".to_string(),
                    value: concat!(
                        "[sound:audio/hola.mp3] ",
                        "<img SRC = \"images/caps.png\"> ",
                        "<img src=missing-unquoted.png> ",
                        "<img src=\"missing.png\"> ",
                        "<video poster=\"video/poster.jpg\"></video> ",
                        "<source srcset=\"images/card@1x.png 1x, missing-srcset.png 2x\"> ",
                        "<object data='docs/root.pdf'></object> ",
                        "<div style=\"background-image:url(images/bg.png); mask:url(#fade)\"></div> ",
                        "<img src=\"data:image/png;base64,skip\">"
                    )
                    .to_string(),
                }],
                tags: Vec::new(),
                created_at: 1,
                updated_at: 1,
            }],
            cards: vec![Card {
                id: "card".to_string(),
                deck_id: "deck".to_string(),
                front: "card <img src=\"images/card-front.png\">".to_string(),
                back: "[sound:audio/back.mp3]".to_string(),
                created_at: 1,
                lineage: None,
            }],
            media_assets: vec![
                MediaAssetRecord {
                    id: "audio".to_string(),
                    archive_name: "0".to_string(),
                    filename: Some("audio/hola.mp3".to_string()),
                    data: Vec::new(),
                },
                MediaAssetRecord {
                    id: "caps".to_string(),
                    archive_name: "1".to_string(),
                    filename: Some("images/caps.png".to_string()),
                    data: Vec::new(),
                },
                MediaAssetRecord {
                    id: "front".to_string(),
                    archive_name: "images/card-front.png".to_string(),
                    filename: None,
                    data: Vec::new(),
                },
                MediaAssetRecord {
                    id: "unused".to_string(),
                    archive_name: "2".to_string(),
                    filename: Some("unused.png".to_string()),
                    data: Vec::new(),
                },
            ],
            ..AppState::default()
        };

        let analysis = analyze_media_references(&state);

        assert_eq!(
            analysis.referenced_filenames,
            vec![
                "audio/back.mp3",
                "audio/hola.mp3",
                "docs/root.pdf",
                "images/bg.png",
                "images/caps.png",
                "images/card-front.png",
                "images/card@1x.png",
                "missing-srcset.png",
                "missing-unquoted.png",
                "missing.png",
                "video/poster.jpg",
            ]
        );
        assert_eq!(
            analysis.referenced_asset_ids,
            vec!["audio", "caps", "front"]
        );
        assert_eq!(
            analysis.missing_filenames,
            vec![
                "audio/back.mp3",
                "docs/root.pdf",
                "images/bg.png",
                "images/card@1x.png",
                "missing-srcset.png",
                "missing-unquoted.png",
                "missing.png",
                "video/poster.jpg",
            ]
        );
        assert_eq!(analysis.unreferenced_asset_ids, vec!["unused"]);
    }
}
