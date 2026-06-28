use std::collections::HashMap;

use crate::model::{GeneratedCard, Note, NoteType};

pub fn generate_cards_for_note(note_type: &NoteType, note: &Note) -> Vec<GeneratedCard> {
    if note.note_type_id != note_type.id {
        return Vec::new();
    }

    let field_names_by_id: HashMap<&str, &str> = note_type
        .fields
        .iter()
        .map(|field| (field.id.as_str(), field.name.as_str()))
        .collect();

    let field_values: HashMap<String, String> = note
        .fields
        .iter()
        .filter_map(|value| {
            field_names_by_id
                .get(value.field_id.as_str())
                .map(|name| ((*name).to_string(), value.value.clone()))
        })
        .collect();

    note_type
        .templates
        .iter()
        .filter(|template| {
            template.required_field_names.iter().all(|field_name| {
                field_values
                    .get(field_name)
                    .is_some_and(|value| !value.trim().is_empty())
            })
        })
        .map(|template| GeneratedCard {
            id: generated_card_id(&note.id, &template.id),
            note_id: note.id.clone(),
            note_type_id: note.note_type_id.clone(),
            template_id: template.id.clone(),
            deck_id: note.deck_id.clone(),
            ordinal: template.ordinal,
            front: render_template(&template.front_template, &field_values),
            back: render_template(&template.back_template, &field_values),
            tags: note.tags.clone(),
        })
        .collect()
}

pub fn render_template(template: &str, field_values: &HashMap<String, String>) -> String {
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
                if let Some(value) = field_values.get(field_name) {
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

fn generated_card_id(note_id: &str, template_id: &str) -> String {
    format!("{note_id}::{template_id}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CardTemplate, FieldDef, NoteFieldValue};

    const NOW: u64 = 1_700_000_000_000;

    fn basic_note_type() -> NoteType {
        NoteType {
            id: "basic-and-reversed".to_string(),
            name: "Basic and reversed".to_string(),
            fields: vec![
                FieldDef {
                    id: "front".to_string(),
                    name: "Front".to_string(),
                    required: true,
                    ordinal: 0,
                },
                FieldDef {
                    id: "back".to_string(),
                    name: "Back".to_string(),
                    required: true,
                    ordinal: 1,
                },
            ],
            templates: vec![
                CardTemplate {
                    id: "forward".to_string(),
                    name: "Forward".to_string(),
                    front_template: "{{Front}}".to_string(),
                    back_template: "{{Back}}".to_string(),
                    required_field_names: vec!["Front".to_string(), "Back".to_string()],
                    ordinal: 0,
                },
                CardTemplate {
                    id: "reverse".to_string(),
                    name: "Reverse".to_string(),
                    front_template: "{{Back}}".to_string(),
                    back_template: "{{Front}}".to_string(),
                    required_field_names: vec!["Front".to_string(), "Back".to_string()],
                    ordinal: 1,
                },
            ],
            created_at: NOW,
            updated_at: NOW,
        }
    }

    fn note(front: &str, back: &str) -> Note {
        Note {
            id: "note-1".to_string(),
            note_type_id: "basic-and-reversed".to_string(),
            deck_id: "deck-1".to_string(),
            fields: vec![
                NoteFieldValue {
                    field_id: "front".to_string(),
                    value: front.to_string(),
                },
                NoteFieldValue {
                    field_id: "back".to_string(),
                    value: back.to_string(),
                },
            ],
            tags: vec!["tamil".to_string(), "script".to_string()],
            created_at: NOW,
            updated_at: NOW,
        }
    }

    #[test]
    fn one_note_can_generate_multiple_cards() {
        let cards = generate_cards_for_note(&basic_note_type(), &note("letter-a", "a"));

        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].id, "note-1::forward");
        assert_eq!(cards[0].front, "letter-a");
        assert_eq!(cards[0].back, "a");
        assert_eq!(cards[1].id, "note-1::reverse");
        assert_eq!(cards[1].front, "a");
        assert_eq!(cards[1].back, "letter-a");
    }

    #[test]
    fn empty_required_fields_suppress_generated_cards() {
        let cards = generate_cards_for_note(&basic_note_type(), &note("letter-a", "  "));

        assert!(cards.is_empty());
    }

    #[test]
    fn generated_card_ids_survive_harmless_note_edits() {
        let before = generate_cards_for_note(&basic_note_type(), &note("letter-a", "a"));
        let after = generate_cards_for_note(&basic_note_type(), &note("letter-aa", "aa"));

        let before_ids: Vec<_> = before.into_iter().map(|card| card.id).collect();
        let after_ids: Vec<_> = after.into_iter().map(|card| card.id).collect();

        assert_eq!(before_ids, after_ids);
    }

    #[test]
    fn mismatched_note_type_generates_no_cards() {
        let mut note = note("letter-a", "a");
        note.note_type_id = "other".to_string();

        let cards = generate_cards_for_note(&basic_note_type(), &note);

        assert!(cards.is_empty());
    }

    #[test]
    fn unknown_template_fields_render_as_empty() {
        let mut values = HashMap::new();
        values.insert("Known".to_string(), "value".to_string());

        let rendered = render_template("{{Known}} {{Missing}}", &values);

        assert_eq!(rendered, "value ");
    }
}
