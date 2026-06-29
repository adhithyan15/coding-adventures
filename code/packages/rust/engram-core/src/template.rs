use std::collections::{BTreeSet, HashMap};

use crate::model::{Card, CardLineage, GeneratedCard, Note, NoteType};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ClozeSide {
    Question,
    Answer,
}

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

    let mut generated = Vec::new();

    for template in &note_type.templates {
        if !template.required_field_names.iter().all(|field_name| {
            field_values
                .get(field_name)
                .is_some_and(|value| !value.trim().is_empty())
        }) {
            continue;
        }

        let cloze_fields =
            cloze_field_names_for_template(&template.front_template, &template.back_template);

        if cloze_fields.is_empty() {
            let front = render_template(&template.front_template, &field_values);
            let back =
                render_template_with_front_side(&template.back_template, &field_values, &front);
            generated.push(GeneratedCard {
                id: generated_card_id(&note.id, &template.id),
                note_id: note.id.clone(),
                note_type_id: note.note_type_id.clone(),
                template_id: template.id.clone(),
                deck_id: note.deck_id.clone(),
                ordinal: template.ordinal,
                cloze_ordinal: None,
                front,
                back,
                tags: note.tags.clone(),
            });
            continue;
        }

        let mut cloze_ordinals = BTreeSet::new();
        for field_name in cloze_fields {
            if let Some(value) = field_values.get(&field_name) {
                collect_cloze_ordinals(value, &mut cloze_ordinals);
            }
        }

        for cloze_ordinal in cloze_ordinals {
            generated.push(GeneratedCard {
                id: generated_cloze_card_id(&note.id, &template.id, cloze_ordinal),
                note_id: note.id.clone(),
                note_type_id: note.note_type_id.clone(),
                template_id: template.id.clone(),
                deck_id: note.deck_id.clone(),
                ordinal: cloze_ordinal.saturating_sub(1),
                cloze_ordinal: Some(cloze_ordinal),
                front: render_cloze_template(
                    &template.front_template,
                    &field_values,
                    cloze_ordinal,
                    ClozeSide::Question,
                ),
                back: render_cloze_template(
                    &template.back_template,
                    &field_values,
                    cloze_ordinal,
                    ClozeSide::Answer,
                ),
                tags: note.tags.clone(),
            });
        }
    }

    generated
}

pub fn render_template(template: &str, field_values: &HashMap<String, String>) -> String {
    render_template_with_front_side(template, field_values, "")
}

pub fn render_template_with_front_side(
    template: &str,
    field_values: &HashMap<String, String>,
    front_side: &str,
) -> String {
    let mut rendered = String::with_capacity(template.len());
    let mut rest = template;

    while let Some(start) = rest.find("{{") {
        let (prefix, after_start) = rest.split_at(start);
        rendered.push_str(prefix);
        let after_start = &after_start[2..];

        match after_start.find("}}") {
            Some(end) => {
                let (tag, after_end) = after_start.split_at(end);
                let tag = tag.trim();
                let after_tag = &after_end[2..];

                if let Some(section) = parse_section_tag(tag) {
                    let close_tag = format!("{{{{/{}}}}}", section.field_name);
                    if let Some(close_start) = after_tag.find(&close_tag) {
                        let (body, after_body) = after_tag.split_at(close_start);
                        if section_should_render(section, field_values) {
                            rendered.push_str(&render_template_with_front_side(
                                body,
                                field_values,
                                front_side,
                            ));
                        }
                        rest = &after_body[close_tag.len()..];
                        continue;
                    }
                }

                rendered.push_str(&render_template_tag(tag, field_values, front_side));
                rest = after_tag;
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

#[derive(Clone, Copy)]
struct SectionTag<'a> {
    field_name: &'a str,
    inverted: bool,
}

fn parse_section_tag(tag: &str) -> Option<SectionTag<'_>> {
    let (inverted, field_name) = if let Some(field_name) = tag.strip_prefix('#') {
        (false, field_name.trim())
    } else if let Some(field_name) = tag.strip_prefix('^') {
        (true, field_name.trim())
    } else {
        return None;
    };

    (!field_name.is_empty()).then_some(SectionTag {
        field_name,
        inverted,
    })
}

fn section_should_render(section: SectionTag<'_>, field_values: &HashMap<String, String>) -> bool {
    let present = field_values
        .get(section.field_name)
        .is_some_and(|value| !value.trim().is_empty());
    if section.inverted {
        !present
    } else {
        present
    }
}

fn render_template_tag(
    tag: &str,
    field_values: &HashMap<String, String>,
    front_side: &str,
) -> String {
    if tag == "FrontSide" {
        return front_side.to_string();
    }

    let field_name = tag
        .strip_prefix("hint:")
        .or_else(|| tag.strip_prefix("type:"))
        .unwrap_or(tag)
        .trim();

    field_values.get(field_name).cloned().unwrap_or_default()
}

pub fn materialize_generated_card(generated: &GeneratedCard, created_at: u64) -> Card {
    Card {
        id: generated.id.clone(),
        deck_id: generated.deck_id.clone(),
        front: generated.front.clone(),
        back: generated.back.clone(),
        created_at,
        lineage: Some(CardLineage {
            note_id: generated.note_id.clone(),
            note_type_id: generated.note_type_id.clone(),
            template_id: generated.template_id.clone(),
            ordinal: generated.ordinal,
            cloze_ordinal: generated.cloze_ordinal,
        }),
    }
}

pub fn rename_note_type_field(
    note_type: &NoteType,
    field_id: &str,
    new_name: &str,
    updated_at: u64,
) -> NoteType {
    let new_name = new_name.trim();
    if new_name.is_empty() {
        return note_type.clone();
    }

    let Some(old_name) = note_type
        .fields
        .iter()
        .find(|field| field.id == field_id)
        .map(|field| field.name.as_str())
    else {
        return note_type.clone();
    };

    if old_name == new_name {
        return note_type.clone();
    }

    let mut renamed = note_type.clone();
    for field in &mut renamed.fields {
        if field.id == field_id {
            field.name = new_name.to_string();
        }
    }
    for template in &mut renamed.templates {
        template.front_template =
            rename_template_field_references(&template.front_template, old_name, new_name);
        template.back_template =
            rename_template_field_references(&template.back_template, old_name, new_name);
        for required_field_name in &mut template.required_field_names {
            if required_field_name == old_name {
                *required_field_name = new_name.to_string();
            }
        }
    }
    renamed.updated_at = updated_at;
    renamed
}

fn generated_card_id(note_id: &str, template_id: &str) -> String {
    format!("{note_id}::{template_id}")
}

fn generated_cloze_card_id(note_id: &str, template_id: &str, cloze_ordinal: u32) -> String {
    format!("{note_id}::{template_id}::c{cloze_ordinal}")
}

fn rename_template_field_references(template: &str, old_name: &str, new_name: &str) -> String {
    let mut renamed = String::with_capacity(template.len());
    let mut rest = template;

    while let Some(start) = rest.find("{{") {
        let (prefix, after_start) = rest.split_at(start);
        renamed.push_str(prefix);
        let after_start = &after_start[2..];

        match after_start.find("}}") {
            Some(end) => {
                let (field_name, after_end) = after_start.split_at(end);
                renamed.push_str("{{");
                renamed.push_str(&rename_template_tag(field_name, old_name, new_name));
                renamed.push_str("}}");
                rest = &after_end[2..];
            }
            None => {
                renamed.push_str("{{");
                renamed.push_str(after_start);
                rest = "";
            }
        }
    }

    renamed.push_str(rest);
    renamed
}

fn rename_template_tag(tag: &str, old_name: &str, new_name: &str) -> String {
    let trimmed = tag.trim();
    if trimmed == old_name {
        return new_name.to_string();
    }

    for prefix in ["cloze:", "hint:", "type:"] {
        if let Some(field_name) = trimmed.strip_prefix(prefix) {
            if field_name.trim() == old_name {
                return format!("{prefix}{new_name}");
            }
        }
    }

    for prefix in ["#", "^", "/"] {
        if let Some(field_name) = trimmed.strip_prefix(prefix) {
            if field_name.trim() == old_name {
                return format!("{prefix}{new_name}");
            }
        }
    }

    tag.to_string()
}

fn cloze_field_names_for_template(front_template: &str, back_template: &str) -> BTreeSet<String> {
    let mut field_names = BTreeSet::new();
    collect_cloze_field_names(front_template, &mut field_names);
    collect_cloze_field_names(back_template, &mut field_names);
    field_names
}

fn collect_cloze_field_names(template: &str, field_names: &mut BTreeSet<String>) {
    let mut rest = template;

    while let Some(start) = rest.find("{{") {
        let (_, after_start) = rest.split_at(start);
        let after_start = &after_start[2..];

        match after_start.find("}}") {
            Some(end) => {
                let (field_name, after_end) = after_start.split_at(end);
                if let Some(field_name) = field_name.trim().strip_prefix("cloze:") {
                    let field_name = field_name.trim();
                    if !field_name.is_empty() {
                        field_names.insert(field_name.to_string());
                    }
                }
                rest = &after_end[2..];
            }
            None => break,
        }
    }
}

fn render_cloze_template(
    template: &str,
    field_values: &HashMap<String, String>,
    cloze_ordinal: u32,
    side: ClozeSide,
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
                        rendered.push_str(&render_cloze_text(value, cloze_ordinal, side));
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

fn collect_cloze_ordinals(value: &str, ordinals: &mut BTreeSet<u32>) {
    let mut rest = value;

    while let Some(start) = rest.find("{{c") {
        let candidate = &rest[start..];
        if let Some(marker) = parse_cloze_marker(candidate) {
            ordinals.insert(marker.ordinal);
            rest = &candidate[marker.consumed..];
        } else {
            rest = &candidate[3..];
        }
    }
}

fn render_cloze_text(value: &str, cloze_ordinal: u32, side: ClozeSide) -> String {
    let mut rendered = String::with_capacity(value.len());
    let mut rest = value;

    while let Some(start) = rest.find("{{c") {
        let (prefix, candidate) = rest.split_at(start);
        rendered.push_str(prefix);

        if let Some(marker) = parse_cloze_marker(candidate) {
            if side == ClozeSide::Question && marker.ordinal == cloze_ordinal {
                match marker.hint.map(str::trim).filter(|hint| !hint.is_empty()) {
                    Some(hint) => {
                        rendered.push('[');
                        rendered.push_str(hint);
                        rendered.push(']');
                    }
                    None => rendered.push_str("[...]"),
                }
            } else {
                rendered.push_str(&render_cloze_text(
                    marker.hidden,
                    cloze_ordinal,
                    ClozeSide::Answer,
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

struct ClozeMarker<'a> {
    ordinal: u32,
    hidden: &'a str,
    hint: Option<&'a str>,
    consumed: usize,
}

fn parse_cloze_marker(candidate: &str) -> Option<ClozeMarker<'_>> {
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

    let ordinal = after_prefix[..digit_len].parse::<u32>().ok()?;
    if ordinal == 0 {
        return None;
    }

    let after_digits = &after_prefix[digit_len..];
    if !after_digits.starts_with("::") {
        return None;
    }

    let content_start = 3 + digit_len + 2;
    let after_content_start = &candidate[content_start..];
    let content_len = after_content_start.find("}}")?;
    let content = &after_content_start[..content_len];
    let consumed = content_start + content_len + 2;
    let (hidden, hint) = match content.split_once("::") {
        Some((hidden, hint)) => (hidden, Some(hint)),
        None => (content, None),
    };

    Some(ClozeMarker {
        ordinal,
        hidden,
        hint,
        consumed,
    })
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

    fn cloze_note_type() -> NoteType {
        NoteType {
            id: "cloze".to_string(),
            name: "Cloze".to_string(),
            fields: vec![
                FieldDef {
                    id: "text".to_string(),
                    name: "Text".to_string(),
                    required: true,
                    ordinal: 0,
                },
                FieldDef {
                    id: "extra".to_string(),
                    name: "Extra".to_string(),
                    required: false,
                    ordinal: 1,
                },
            ],
            templates: vec![CardTemplate {
                id: "cloze".to_string(),
                name: "Cloze".to_string(),
                front_template: "{{cloze:Text}}".to_string(),
                back_template: "{{cloze:Text}}<hr>{{Extra}}".to_string(),
                required_field_names: vec!["Text".to_string()],
                ordinal: 0,
            }],
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

    fn cloze_note(text: &str, extra: &str) -> Note {
        Note {
            id: "cloze-note".to_string(),
            note_type_id: "cloze".to_string(),
            deck_id: "deck-1".to_string(),
            fields: vec![
                NoteFieldValue {
                    field_id: "text".to_string(),
                    value: text.to_string(),
                },
                NoteFieldValue {
                    field_id: "extra".to_string(),
                    value: extra.to_string(),
                },
            ],
            tags: vec!["grammar".to_string(), "spanish".to_string()],
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
    fn materialized_generated_cards_retain_note_lineage() {
        let generated = generate_cards_for_note(&basic_note_type(), &note("letter-a", "a"));
        let card = materialize_generated_card(&generated[0], NOW);
        let lineage = card.lineage.expect("generated cards carry lineage");

        assert_eq!(card.id, "note-1::forward");
        assert_eq!(card.deck_id, "deck-1");
        assert_eq!(lineage.note_id, "note-1");
        assert_eq!(lineage.note_type_id, "basic-and-reversed");
        assert_eq!(lineage.template_id, "forward");
        assert_eq!(lineage.ordinal, 0);
        assert_eq!(lineage.cloze_ordinal, None);
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

    #[test]
    fn anki_style_sections_hints_and_front_side_render() {
        let mut values = HashMap::new();
        values.insert("Front".to_string(), "hola".to_string());
        values.insert("Back".to_string(), "hello".to_string());
        values.insert("Extra".to_string(), String::new());

        assert_eq!(
            render_template(
                "{{#Back}}{{Back}}{{/Back}}{{^Extra}} no-extra{{/Extra}}",
                &values
            ),
            "hello no-extra"
        );
        assert_eq!(render_template("{{hint:Back}}", &values), "hello");
        assert_eq!(
            render_template_with_front_side("{{FrontSide}}<hr>{{Back}}", &values, "hola"),
            "hola<hr>hello"
        );
    }

    #[test]
    fn generated_cards_support_front_side_on_back_template() {
        let mut note_type = basic_note_type();
        note_type.templates[0].back_template = "{{FrontSide}}<hr>{{Back}}".to_string();

        let cards = generate_cards_for_note(&note_type, &note("letter-a", "a"));

        assert_eq!(cards[0].front, "letter-a");
        assert_eq!(cards[0].back, "letter-a<hr>a");
    }

    #[test]
    fn renaming_note_type_field_migrates_templates_and_required_fields() {
        let note_type = basic_note_type();
        let renamed = rename_note_type_field(&note_type, "front", "Prompt", NOW + 1);
        let cards = generate_cards_for_note(&renamed, &note("letter-a", "a"));

        assert_eq!(renamed.fields[0].name, "Prompt");
        assert_eq!(renamed.updated_at, NOW + 1);
        assert_eq!(renamed.templates[0].front_template, "{{Prompt}}");
        assert_eq!(
            renamed.templates[0].required_field_names,
            vec!["Prompt", "Back"]
        );
        assert_eq!(cards[0].id, "note-1::forward");
        assert_eq!(cards[0].front, "letter-a");
        assert_eq!(cards[1].id, "note-1::reverse");
        assert_eq!(cards[1].back, "letter-a");
    }

    #[test]
    fn renaming_field_migrates_anki_section_and_helper_references() {
        let mut note_type = basic_note_type();
        note_type.templates[0].front_template =
            "{{#Front}}{{hint:Front}}{{/Front}}{{^Back}}{{type:Front}}{{/Back}}".to_string();
        note_type.templates[0].back_template = "{{FrontSide}}<hr>{{cloze:Front}}".to_string();

        let renamed = rename_note_type_field(&note_type, "front", "Prompt", NOW + 1);

        assert_eq!(
            renamed.templates[0].front_template,
            "{{#Prompt}}{{hint:Prompt}}{{/Prompt}}{{^Back}}{{type:Prompt}}{{/Back}}"
        );
        assert_eq!(
            renamed.templates[0].back_template,
            "{{FrontSide}}<hr>{{cloze:Prompt}}"
        );
    }

    #[test]
    fn renaming_cloze_field_migrates_cloze_template_references() {
        let note_type = cloze_note_type();
        let renamed = rename_note_type_field(&note_type, "text", "Sentence", NOW + 1);
        let cards = generate_cards_for_note(
            &renamed,
            &cloze_note("A {{c1::root::base}} carries meaning.", "etymology"),
        );

        assert_eq!(renamed.fields[0].name, "Sentence");
        assert_eq!(renamed.templates[0].front_template, "{{cloze:Sentence}}");
        assert_eq!(
            renamed.templates[0].back_template,
            "{{cloze:Sentence}}<hr>{{Extra}}"
        );
        assert_eq!(renamed.templates[0].required_field_names, vec!["Sentence"]);
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].id, "cloze-note::cloze::c1");
        assert_eq!(cards[0].front, "A [base] carries meaning.");
    }

    #[test]
    fn renaming_unknown_or_blank_field_is_noop() {
        let note_type = basic_note_type();

        assert_eq!(
            rename_note_type_field(&note_type, "missing", "Prompt", NOW + 1),
            note_type
        );
        assert_eq!(
            rename_note_type_field(&note_type, "front", "   ", NOW + 1),
            note_type
        );
    }

    #[test]
    fn cloze_notes_generate_one_card_per_cloze_ordinal() {
        let cards = generate_cards_for_note(
            &cloze_note_type(),
            &cloze_note(
                "A {{c2::suffix}} changes a {{c1::root::base}} word into a new form.",
                "root + suffix",
            ),
        );

        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].id, "cloze-note::cloze::c1");
        assert_eq!(cards[0].ordinal, 0);
        assert_eq!(cards[0].cloze_ordinal, Some(1));
        assert_eq!(
            cards[0].front,
            "A suffix changes a [base] word into a new form."
        );
        assert_eq!(
            cards[0].back,
            "A suffix changes a root word into a new form.<hr>root + suffix"
        );
        assert_eq!(cards[1].id, "cloze-note::cloze::c2");
        assert_eq!(cards[1].ordinal, 1);
        assert_eq!(cards[1].cloze_ordinal, Some(2));
        assert_eq!(
            cards[1].front,
            "A [...] changes a root word into a new form."
        );
        assert_eq!(
            cards[1].back,
            "A suffix changes a root word into a new form.<hr>root + suffix"
        );
        assert_eq!(cards[1].tags, vec!["grammar", "spanish"]);
    }

    #[test]
    fn cloze_generation_deduplicates_repeated_ordinals() {
        let cards = generate_cards_for_note(
            &cloze_note_type(),
            &cloze_note("{{c1::Tamil}} and {{c1::Dravidian}}", ""),
        );

        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].id, "cloze-note::cloze::c1");
        assert_eq!(cards[0].front, "[...] and [...]");
        assert_eq!(cards[0].back, "Tamil and Dravidian<hr>");
    }

    #[test]
    fn cloze_generated_cards_materialize_lineage() {
        let cards = generate_cards_for_note(
            &cloze_note_type(),
            &cloze_note("The word {{c1::night}} traces to old roots.", ""),
        );
        let card = materialize_generated_card(&cards[0], NOW + 1);
        let lineage = card.lineage.expect("cloze cards carry lineage");

        assert_eq!(card.id, "cloze-note::cloze::c1");
        assert_eq!(card.front, "The word [...] traces to old roots.");
        assert_eq!(lineage.note_id, "cloze-note");
        assert_eq!(lineage.note_type_id, "cloze");
        assert_eq!(lineage.template_id, "cloze");
        assert_eq!(lineage.ordinal, 0);
        assert_eq!(lineage.cloze_ordinal, Some(1));
    }

    #[test]
    fn malformed_cloze_markers_render_literally() {
        let cards = generate_cards_for_note(
            &cloze_note_type(),
            &cloze_note("{{cx::root}} {{c0::bad}}", ""),
        );

        assert!(cards.is_empty());

        let mut values = HashMap::new();
        values.insert("Text".to_string(), "{{c1::root}} {{cx::kept}}".to_string());
        let rendered = render_cloze_template("{{cloze:Text}}", &values, 1, ClozeSide::Question);

        assert_eq!(rendered, "[...] {{cx::kept}}");
    }
}
