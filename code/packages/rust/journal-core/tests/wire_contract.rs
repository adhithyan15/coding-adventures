//! The JSON wire contract the host facade (journal-wasm, J2b) will speak.
//!
//! Only compiled with `--features serde`. These tests pin the *shape* of the JSON
//! — camelCase keys, bare-string ids, ISO dates, tags as plain strings, commands
//! tagged by `type` — because every Mosaic host will parse it, and a silent rename
//! here would break all nine at once.
#![cfg(feature = "serde")]

use journal_core::projections::timeline;
use journal_core::{apply, Command, Date, EntryFilter, EntryId, JournalId, JournalState};
use serde_json::json;

fn sample() -> JournalState {
    let mut s = JournalState::new(JournalId::from("p"), "Personal", 7).unwrap();
    apply(
        &mut s,
        Command::CreateEntry {
            id: EntryId::from("e1"),
            journal: JournalId::from("p"),
            date: Date::parse_iso("2026-09-23").unwrap(),
            title: "Hi".into(),
            body: "Body".into(),
        },
        10,
    )
    .unwrap();
    apply(
        &mut s,
        Command::SetTags {
            id: EntryId::from("e1"),
            tags: vec!["Travel".into()],
        },
        11,
    )
    .unwrap();
    s
}

#[test]
fn state_serialises_in_the_documented_shape() {
    let v = serde_json::to_value(sample()).unwrap();
    assert_eq!(
        v,
        json!({
            "journals": { "p": { "id": "p", "name": "Personal", "createdAtMs": 7 } },
            "entries": { "e1": {
                "id": "e1", "journal": "p", "title": "Hi", "body": "Body",
                "date": "2026-09-23", "createdAtMs": 10, "updatedAtMs": 11,
                "tags": ["Travel"], "starred": false
            } }
        })
    );
}

#[test]
fn state_round_trips_and_still_validates() {
    let s = sample();
    let text = serde_json::to_string(&s).unwrap();
    let back: JournalState = serde_json::from_str(&text).unwrap();
    assert_eq!(back, s);
    assert!(back.validate().is_ok());
}

#[test]
fn tags_and_starred_default_when_absent() {
    // An entry written before tags existed still loads.
    let v = json!({
        "journals": { "p": { "id": "p", "name": "P", "createdAtMs": 0 } },
        "entries": { "e": { "id": "e", "journal": "p", "title": "", "body": "",
                            "date": "2020-01-01", "createdAtMs": 0, "updatedAtMs": 0 } }
    });
    let s: JournalState = serde_json::from_value(v).unwrap();
    let e = s.entry(&EntryId::from("e")).unwrap();
    assert!(e.tags.is_empty() && !e.starred);
}

#[test]
fn malformed_dates_and_tags_are_refused_on_load() {
    let bad_date = json!("2026-02-30");
    assert!(serde_json::from_value::<Date>(bad_date).is_err());
    let bad_tag = json!("two\nlines");
    assert!(serde_json::from_value::<journal_core::Tag>(bad_tag).is_err());
}

#[test]
fn loaded_state_that_breaks_invariants_fails_validation() {
    // Well-formed JSON, but the entry names a journal that does not exist.
    let v = json!({
        "journals": { "p": { "id": "p", "name": "P", "createdAtMs": 0 } },
        "entries": { "e": { "id": "e", "journal": "ghost", "title": "", "body": "",
                            "date": "2020-01-01", "createdAtMs": 0, "updatedAtMs": 0 } }
    });
    let s: JournalState = serde_json::from_value(v).unwrap();
    assert_eq!(
        s.validate(),
        Err(journal_core::OpError::JournalNotFound(JournalId::from(
            "ghost"
        )))
    );
}

#[test]
fn commands_are_tagged_by_type_with_camel_case_fields() {
    let cmd: Command = serde_json::from_value(json!({
        "type": "deleteJournal", "id": "w", "moveEntriesTo": "p"
    }))
    .unwrap();
    assert_eq!(
        cmd,
        Command::DeleteJournal {
            id: JournalId::from("w"),
            move_entries_to: Some(JournalId::from("p")),
        }
    );
    let v = serde_json::to_value(Command::SetStarred {
        id: EntryId::from("e"),
        starred: true,
    })
    .unwrap();
    assert_eq!(
        v,
        json!({ "type": "setStarred", "id": "e", "starred": true })
    );
}

#[test]
fn projections_serialise_for_the_host() {
    let t = timeline(&sample(), &EntryFilter::default());
    assert_eq!(
        serde_json::to_value(t).unwrap(),
        json!([{ "date": "2026-09-23", "entries": ["e1"] }])
    );
    let f: EntryFilter = serde_json::from_value(json!({ "tag": "travel" })).unwrap();
    assert!(f.journal.is_none() && !f.starred_only);
}
