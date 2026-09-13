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

/// The key two provenance records are considered the same by.
///
/// LENGTH-PREFIXED, not separator-joined, and that is the whole point.
///
/// This used to join the four parts with `\u{1f}`, and two of them are
/// attacker-influenced when the collection came from an `.apkg`: a media
/// record's `target_id` is `format!("anki-media:{archive_name}")`, where the
/// archive name is a zip entry, and `original_id` carries the Anki
/// `notes.guid`. Moving a `\u{1f}` across the boundary between two parts
/// produced the same key from genuinely different records — measured, both
/// rendering as
/// `Media\u{1f}anki-media:art\u{1f}anki-v11\u{1f}later\u{1f}` — and
/// `upsert_by` REPLACES on a key match, so one silently displaced the other.
///
/// Prefixing each part with its byte length makes the encoding a netstring, and
/// therefore injective OVER THE FOUR RENDERED PARTS: a decoder reads to the
/// first `:`, takes exactly that many bytes, and never infers a part's extent
/// from its content, so nothing inside a part can confuse the split. A guard on
/// the separator would close this instance; the prefix closes the class, and
/// needs nothing at the producers.
///
/// Over the four parts, not over the record. `original_id: None` and
/// `Some("")` both render as the empty part and still share a key — unchanged
/// behaviour, not a regression, but the distinction matters if anyone ever
/// needs those two told apart (encode presence: a `-1:` sentinel, or a fifth
/// part).
///
/// The trailing `\u{1f}` after the last part is a visual marker, not a
/// delimiter — the lengths alone are sufficient, confirmed by a reviewer
/// brute-forcing the encoding without it. Said so that nobody removing it
/// believes they have broken the injectivity, or adds a guard protecting it.
///
/// Fixed at the ENCODING rather than by a guard, which is the opposite of the
/// choice `engram-core-wasm` made for the same shape in card ids. The reason is
/// persistence: a card id is written into saved collections, snapshots and
/// exported packages, so re-encoding it rewrites data already on disk. This key
/// is derived per merge and stored nowhere, so changing it costs nothing.
///
/// Byte length, not char count: two different strings can share a char count,
/// and the point is that the parts cannot be re-split ambiguously.
fn external_source_merge_key(source: &ExternalSourceRecord) -> String {
    let target = format!("{:?}", source.target);
    // EVERY part, including the target.
    //
    // The first version of this comment said leaving the target bare would be
    // safe "because no variant name is a prefix of another". That is FALSE:
    // `Note` is a prefix of `NoteType`. Leaving it bare would still have been
    // safe, but for a different reason -- the next byte emitted is always a
    // decimal digit, so `Note` followed by `9:` can never be read as
    // `NoteType`. Caught in review.
    //
    // Which is the argument for prefixing it rather than reasoning about it: a
    // case analysis a reader has to redo every time a variant is added, and get
    // right, is worse than three characters.
    let mut key = String::new();
    for part in [
        target.as_str(),
        source.target_id.as_str(),
        source.source.as_str(),
        source.original_id.as_deref().unwrap_or_default(),
    ] {
        key.push_str(&part.len().to_string());
        key.push(':');
        key.push_str(part);
        key.push('\u{1f}');
    }
    key
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

    /// Two distinct provenance records must not share a merge key.
    ///
    /// The key joined its four parts with `\u{1f}`, and two of them are
    /// attacker-influenced when the collection came from an `.apkg`:
    /// `target_id` for media is `format!("anki-media:{archive_name}")`, where
    /// the archive name is a zip entry, and `original_id` carries the Anki
    /// `notes.guid`. Neither was checked for the separator, so moving it across
    /// the boundary produced the same key from different records — and
    /// `upsert_by` REPLACES on a key match, so one silently displaced the
    /// other.
    ///
    /// The same shape `engram-core-wasm` already refuses for card ids, where
    /// `{note_id}::{template_id}` is ambiguous unless the halves are checked.
    /// This one is fixed at the encoding instead of by a guard: nothing
    /// persists this key, so unlike a card id it can be changed without
    /// rewriting data already on disk.
    #[test]
    fn a_separator_in_a_provenance_field_cannot_forge_another_records_key() {
        // `\u{1f}` moved across the target_id/source boundary. Both records
        // describe genuinely different things.
        let left = source(
            ExternalSourceTarget::Media,
            "anki-media:art\u{1f}anki-v11",
            "later",
            None,
            BTreeMap::new(),
        );
        let right = source(
            ExternalSourceTarget::Media,
            "anki-media:art",
            "anki-v11\u{1f}later",
            None,
            BTreeMap::new(),
        );
        assert_ne!(
            external_source_merge_key(&left),
            external_source_merge_key(&right),
            "two different records produced one merge key"
        );

        // And the consequence, asserted through the merge rather than inferred
        // from the keys: both records survive.
        let merged = merge_app_states(
            &AppState {
                external_sources: vec![left.clone()],
                ..AppState::default()
            },
            AppState {
                external_sources: vec![right.clone()],
                ..AppState::default()
            },
        );
        assert_eq!(
            merged.external_sources.len(),
            2,
            "one record displaced the other: {:?}",
            merged.external_sources
        );
        assert!(merged.external_sources.contains(&left));
        assert!(merged.external_sources.contains(&right));
    }

    /// No two distinct field-triples share a key, across an adversarial set.
    ///
    /// The single hand-built collision above is the one that was found; this is
    /// the class. Every part is varied against every other with the separator,
    /// the length delimiter and digits placed where they could confuse a
    /// re-split, and all keys must differ.
    ///
    /// Written as all-pairs rather than a few chosen pairs because the failure
    /// being guarded is precisely the one nobody thought to write down.
    #[test]
    fn no_two_distinct_provenance_records_share_a_key() {
        // Each of these is a plausible-to-hostile value for a part: a `\u{1f}`
        // at each end and in the middle, a `:` that could pass for the length
        // delimiter, digits that could pass for a length, and the empty string
        // that `original_id: None` produces.
        let parts = [
            "",
            "a",
            "ab",
            "a\u{1f}b",
            "\u{1f}ab",
            "ab\u{1f}",
            "2:ab",
            "1:a\u{1f}1:b",
            "anki-media:art",
            "anki-media:art\u{1f}anki-v11",
        ];

        // `Note` and `NoteType` specifically, because the comment on the key
        // first justified the target prefix by claiming no variant name is a
        // prefix of another -- and these two are exactly the counterexample.
        // The first version of this test held the target at `Media` while its
        // doc said "every part is varied against every other", so it could not
        // have caught the claim being wrong.
        let targets = [
            ExternalSourceTarget::Media,
            ExternalSourceTarget::Note,
            ExternalSourceTarget::NoteType,
        ];

        type Tuple = (String, String, String, String);
        let mut seen: BTreeMap<String, Tuple> = BTreeMap::new();
        for target in targets {
            for target_id in parts {
                for source_name in parts {
                    for original in parts {
                        let record = source(
                            target,
                            target_id,
                            source_name,
                            Some(original),
                            BTreeMap::new(),
                        );
                        let key = external_source_merge_key(&record);
                        let tuple = (
                            format!("{target:?}"),
                            target_id.to_string(),
                            source_name.to_string(),
                            original.to_string(),
                        );
                        if let Some(previous) = seen.insert(key.clone(), tuple.clone()) {
                            panic!("{previous:?} and {tuple:?} share the key {key:?}");
                        }
                    }
                }
            }
        }
        // 3 * 10^3, so the loop really ran over what it claims to have covered
        // rather than short-circuiting somewhere.
        assert_eq!(seen.len(), targets.len() * parts.len().pow(3));
    }

    /// The same record still merges onto itself.
    ///
    /// The point of the key is de-duplication, so a fix that made every key
    /// unique would pass the test above and break what this is for. Re-importing
    /// the same package must still update one record rather than accumulate a
    /// second.
    #[test]
    fn the_same_provenance_record_still_merges_onto_itself() {
        let mut data = BTreeMap::new();
        data.insert("guid".to_string(), "abc".to_string());
        let first = source(
            ExternalSourceTarget::Media,
            "anki-media:art.png",
            "anki-v11",
            Some("guid-1"),
            BTreeMap::new(),
        );
        let again = source(
            ExternalSourceTarget::Media,
            "anki-media:art.png",
            "anki-v11",
            Some("guid-1"),
            data,
        );
        assert_eq!(
            external_source_merge_key(&first),
            external_source_merge_key(&again),
            "the same record must keep one key"
        );

        let merged = merge_app_states(
            &AppState {
                external_sources: vec![first],
                ..AppState::default()
            },
            AppState {
                external_sources: vec![again.clone()],
                ..AppState::default()
            },
        );
        assert_eq!(merged.external_sources, vec![again]);
    }
}
