use crate::model::{AppState, ExternalSourceRecord, ExternalSourceTarget, MediaAssetRecord};
use std::collections::HashMap;

pub fn merge_app_states(current: &AppState, imported: AppState) -> AppState {
    let mut merged = current.clone();
    let mut imported_external_sources = imported.external_sources;

    upsert_by(&mut merged.decks, imported.decks, |deck| deck.id.clone());
    upsert_by(&mut merged.note_types, imported.note_types, |note_type| {
        note_type.id.clone()
    });
    upsert_by(&mut merged.notes, imported.notes, |note| note.id.clone());
    upsert_by(&mut merged.cards, imported.cards, |card| card.id.clone());
    upsert_by(
        &mut merged.card_progress,
        imported.card_progress,
        |progress| progress.card_id.clone(),
    );
    upsert_by(&mut merged.sessions, imported.sessions, |session| {
        session.id.clone()
    });
    upsert_by(&mut merged.reviews, imported.reviews, |review| {
        review.id.clone()
    });
    upsert_by(&mut merged.deck_options, imported.deck_options, |preset| {
        preset.deck_id.clone()
    });

    let media_remaps = merge_media_assets(&mut merged.media_assets, imported.media_assets);
    retarget_external_sources(
        &mut imported_external_sources,
        ExternalSourceTarget::Media,
        &media_remaps.ids,
    );
    retarget_media_archive_names(&mut imported_external_sources, &media_remaps.archive_names);
    upsert_by(
        &mut merged.external_sources,
        imported_external_sources,
        external_source_merge_key,
    );

    if let Some(active_session) = imported.active_session {
        merged.active_session = Some(active_session);
    }
    merged
}

fn upsert_by<T>(target: &mut Vec<T>, incoming: Vec<T>, key: impl Fn(&T) -> String) {
    for item in incoming {
        let item_key = key(&item);
        if let Some(existing) = target.iter_mut().find(|existing| key(existing) == item_key) {
            *existing = item;
        } else {
            target.push(item);
        }
    }
}

fn external_source_merge_key(source: &ExternalSourceRecord) -> String {
    format!(
        "{:?}\u{1f}{}\u{1f}{}\u{1f}{}",
        source.target,
        source.target_id,
        source.source,
        source.original_id.as_deref().unwrap_or_default()
    )
}

fn retarget_external_sources(
    sources: &mut [ExternalSourceRecord],
    target: ExternalSourceTarget,
    id_remaps: &HashMap<String, String>,
) {
    if id_remaps.is_empty() {
        return;
    }

    for source in sources {
        if source.target == target {
            if let Some(next_id) = id_remaps.get(&source.target_id) {
                source.target_id = next_id.clone();
            }
        }
    }
}

fn retarget_media_archive_names(
    sources: &mut [ExternalSourceRecord],
    archive_name_remaps: &HashMap<String, String>,
) {
    if archive_name_remaps.is_empty() {
        return;
    }

    for source in sources {
        if source.target != ExternalSourceTarget::Media {
            continue;
        }
        if let Some(archive_name) = source.data.get_mut("archiveName") {
            if let Some(next_archive_name) = archive_name_remaps.get(archive_name) {
                *archive_name = next_archive_name.clone();
            }
        }
    }
}

#[derive(Default)]
struct MediaMergeRemaps {
    ids: HashMap<String, String>,
    archive_names: HashMap<String, String>,
}

fn merge_media_assets(
    target: &mut Vec<MediaAssetRecord>,
    incoming: Vec<MediaAssetRecord>,
) -> MediaMergeRemaps {
    let mut remaps = MediaMergeRemaps::default();
    for mut asset in incoming {
        match target.iter().position(|existing| existing.id == asset.id) {
            Some(index)
                if target[index].filename == asset.filename && target[index].data == asset.data =>
            {
                target[index] = asset;
            }
            Some(_) => {
                let original_id = asset.id.clone();
                let original_archive_name = asset.archive_name.clone();
                let unique = next_unique_media_suffix(target, &asset.id, &asset.archive_name);
                asset.id = format!("{}-merge-{unique}", asset.id);
                asset.archive_name = format!("{}-merge-{unique}", asset.archive_name);
                remaps.ids.insert(original_id, asset.id.clone());
                remaps
                    .archive_names
                    .insert(original_archive_name, asset.archive_name.clone());
                target.push(asset);
            }
            None if target
                .iter()
                .any(|existing| existing.archive_name == asset.archive_name) =>
            {
                let original_archive_name = asset.archive_name.clone();
                let unique = next_unique_media_suffix(target, &asset.id, &asset.archive_name);
                asset.archive_name = format!("{}-merge-{unique}", asset.archive_name);
                remaps
                    .archive_names
                    .insert(original_archive_name, asset.archive_name.clone());
                target.push(asset);
            }
            None => target.push(asset),
        }
    }
    remaps
}

fn next_unique_media_suffix(
    target: &[MediaAssetRecord],
    base_id: &str,
    base_archive_name: &str,
) -> usize {
    let mut suffix = 1;
    while target.iter().any(|asset| {
        asset.id == format!("{base_id}-merge-{suffix}")
            || asset.archive_name == format!("{base_archive_name}-merge-{suffix}")
    }) {
        suffix += 1;
    }
    suffix
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Card, Deck, Note, NoteFieldValue};
    use std::collections::BTreeMap;

    const NOW: u64 = 1_700_000_000_000;

    #[test]
    fn merge_app_states_upserts_imported_notes_without_replacing_local_state() {
        let current = AppState {
            decks: vec![deck("local", "Tamil")],
            cards: vec![card("local-card", "local", "amma", "mother")],
            ..AppState::default()
        };
        let imported = AppState {
            notes: vec![note("note-1", "deck")],
            cards: vec![card("note-1::forward", "deck", "hola", "hello")],
            external_sources: vec![source(
                ExternalSourceTarget::Note,
                "note-1",
                "anki-text",
                Some("guid-1"),
                BTreeMap::new(),
            )],
            ..AppState::default()
        };

        let merged = merge_app_states(&current, imported);

        assert!(merged.decks.iter().any(|deck| deck.id == "local"));
        assert!(merged.cards.iter().any(|card| card.id == "local-card"));
        assert!(merged.cards.iter().any(|card| card.id == "note-1::forward"));
        assert!(merged.notes.iter().any(|note| note.id == "note-1"));
        assert!(merged.external_sources.iter().any(|source| {
            source.target == ExternalSourceTarget::Note
                && source.target_id == "note-1"
                && source.source == "anki-text"
                && source.original_id.as_deref() == Some("guid-1")
        }));
    }

    #[test]
    fn merge_app_states_retargets_conflicting_media_sources() {
        let current = AppState {
            media_assets: vec![
                media("anki-media:0", "0", Some("audio/local.mp3"), b"local"),
                media("local-image", "1", Some("images/local.png"), b"local"),
            ],
            external_sources: vec![source(
                ExternalSourceTarget::Media,
                "anki-media:0",
                "local-fixture",
                Some("0"),
                BTreeMap::from([("archiveName".to_string(), "0".to_string())]),
            )],
            ..AppState::default()
        };
        let imported = AppState {
            media_assets: vec![
                media("anki-media:0", "0", Some("audio/hola.mp3"), b"hola"),
                media("anki-media:1", "1", Some("images/card.png"), b"card"),
            ],
            external_sources: vec![
                source(
                    ExternalSourceTarget::Media,
                    "anki-media:0",
                    "anki-v11",
                    Some("0"),
                    BTreeMap::from([("archiveName".to_string(), "0".to_string())]),
                ),
                source(
                    ExternalSourceTarget::Media,
                    "anki-media:1",
                    "anki-v11",
                    Some("1"),
                    BTreeMap::from([("archiveName".to_string(), "1".to_string())]),
                ),
            ],
            ..AppState::default()
        };

        let merged = merge_app_states(&current, imported);

        assert!(merged.media_assets.iter().any(|asset| {
            asset.id == "anki-media:0-merge-1"
                && asset.archive_name == "0-merge-1"
                && asset.filename.as_deref() == Some("audio/hola.mp3")
        }));
        assert!(merged.media_assets.iter().any(|asset| {
            asset.id == "anki-media:1"
                && asset.archive_name == "1-merge-1"
                && asset.filename.as_deref() == Some("images/card.png")
        }));
        assert!(merged.external_sources.iter().any(|source| {
            source.target == ExternalSourceTarget::Media
                && source.target_id == "anki-media:0-merge-1"
                && source.source == "anki-v11"
                && source.original_id.as_deref() == Some("0")
                && source.data.get("archiveName").map(String::as_str) == Some("0-merge-1")
        }));
        assert!(merged.external_sources.iter().any(|source| {
            source.target == ExternalSourceTarget::Media
                && source.target_id == "anki-media:1"
                && source.source == "anki-v11"
                && source.original_id.as_deref() == Some("1")
                && source.data.get("archiveName").map(String::as_str) == Some("1-merge-1")
        }));
    }

    fn deck(id: &str, name: &str) -> Deck {
        Deck {
            id: id.to_string(),
            name: name.to_string(),
            description: String::new(),
            created_at: NOW,
        }
    }

    fn card(id: &str, deck_id: &str, front: &str, back: &str) -> Card {
        Card {
            id: id.to_string(),
            deck_id: deck_id.to_string(),
            front: front.to_string(),
            back: back.to_string(),
            created_at: NOW,
            lineage: None,
        }
    }

    fn note(id: &str, deck_id: &str) -> Note {
        Note {
            id: id.to_string(),
            note_type_id: "basic".to_string(),
            deck_id: deck_id.to_string(),
            fields: vec![NoteFieldValue {
                field_id: "front".to_string(),
                value: "hola".to_string(),
            }],
            tags: vec!["spanish".to_string()],
            created_at: NOW,
            updated_at: NOW,
        }
    }

    fn media(
        id: &str,
        archive_name: &str,
        filename: Option<&str>,
        data: &[u8],
    ) -> MediaAssetRecord {
        MediaAssetRecord {
            id: id.to_string(),
            archive_name: archive_name.to_string(),
            filename: filename.map(str::to_string),
            data: data.to_vec(),
        }
    }

    fn source(
        target: ExternalSourceTarget,
        target_id: &str,
        source: &str,
        original_id: Option<&str>,
        data: BTreeMap<String, String>,
    ) -> ExternalSourceRecord {
        ExternalSourceRecord {
            target,
            target_id: target_id.to_string(),
            source: source.to_string(),
            original_id: original_id.map(str::to_string),
            data,
        }
    }
}
