use std::collections::{BTreeSet, HashMap};

use unicode_normalize::{char::is_combining_mark, UnicodeNormalize};

use crate::model::{
    Card, CardLineage, CardTemplate, GeneratedCard, Note, NoteType, TemplateRequirementMode,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClozeRenderSide {
    Question,
    Answer,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeAnswerSpec {
    pub field_name: String,
    pub expected: String,
    pub normalized_expected: String,
    pub ignore_combining: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TemplateFieldFilter {
    Cloze,
    Hint,
    Type,
    NoCombining,
    Text,
    Furigana,
    Kana,
    Kanji,
}

impl TemplateFieldFilter {
    fn prefix(self) -> &'static str {
        match self {
            Self::Cloze => "cloze:",
            Self::Hint => "hint:",
            Self::Type => "type:",
            Self::NoCombining => "nc:",
            Self::Text => "text:",
            Self::Furigana => "furigana:",
            Self::Kana => "kana:",
            Self::Kanji => "kanji:",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RubyMode {
    Furigana,
    Kana,
    Kanji,
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

    let base_field_values: HashMap<String, String> = note
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
        if !template_requirement_satisfied(template, &base_field_values) {
            continue;
        }

        let cloze_fields =
            cloze_field_names_for_template(&template.front_template, &template.back_template);

        if cloze_fields.is_empty() {
            let card_id = generated_card_id(&note.id, &template.id);
            let deck_id = template_deck_id(template, note).to_string();
            let mut field_values = base_field_values.clone();
            insert_special_template_values(
                &mut field_values,
                note_type,
                note,
                template,
                &card_id,
                &deck_id,
            );
            let front = render_template(&template.front_template, &field_values);
            let back =
                render_template_with_front_side(&template.back_template, &field_values, &front);
            generated.push(GeneratedCard {
                id: card_id,
                note_id: note.id.clone(),
                note_type_id: note.note_type_id.clone(),
                template_id: template.id.clone(),
                deck_id,
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
            if let Some(value) = base_field_values.get(&field_name) {
                collect_cloze_ordinals(value, &mut cloze_ordinals);
            }
        }

        for cloze_ordinal in cloze_ordinals {
            let card_id = generated_cloze_card_id(&note.id, &template.id, cloze_ordinal);
            let deck_id = template_deck_id(template, note).to_string();
            let mut field_values = base_field_values.clone();
            insert_special_template_values(
                &mut field_values,
                note_type,
                note,
                template,
                &card_id,
                &deck_id,
            );
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
            generated.push(GeneratedCard {
                id: card_id,
                note_id: note.id.clone(),
                note_type_id: note.note_type_id.clone(),
                template_id: template.id.clone(),
                deck_id,
                ordinal: cloze_ordinal.saturating_sub(1),
                cloze_ordinal: Some(cloze_ordinal),
                front,
                back,
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
    } else {
        let field_name = tag.strip_prefix('^')?;
        (true, field_name.trim())
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

    render_template_field_tag(tag, field_values, None)
}

fn render_template_field_tag(
    tag: &str,
    field_values: &HashMap<String, String>,
    cloze_context: Option<(u32, ClozeRenderSide)>,
) -> String {
    let (filters, field_name) = parse_template_field_filters(tag);
    let field_name = field_name.trim();
    let Some(raw_value) = field_values.get(field_name) else {
        return String::new();
    };

    if filters.contains(&TemplateFieldFilter::Hint) {
        return if raw_value.trim().is_empty() {
            String::new()
        } else {
            render_hint_placeholder(field_name)
        };
    }

    if filters.contains(&TemplateFieldFilter::Type) {
        return if raw_value.trim().is_empty() {
            String::new()
        } else {
            render_type_answer_placeholder(field_name)
        };
    }

    let mut value = raw_value.clone();

    for filter in filters.iter().rev() {
        match filter {
            TemplateFieldFilter::Cloze => {
                if let Some((cloze_ordinal, side)) = cloze_context {
                    value = render_cloze_text(&value, cloze_ordinal, side);
                }
            }
            TemplateFieldFilter::Text => value = html_to_text(&value),
            TemplateFieldFilter::Furigana => value = render_ruby_text(&value, RubyMode::Furigana),
            TemplateFieldFilter::Kana => value = render_ruby_text(&value, RubyMode::Kana),
            TemplateFieldFilter::Kanji => value = render_ruby_text(&value, RubyMode::Kanji),
            TemplateFieldFilter::Hint
            | TemplateFieldFilter::Type
            | TemplateFieldFilter::NoCombining => {}
        }
    }

    value
}

fn render_hint_placeholder(field_name: &str) -> String {
    format!("[show hint: {field_name}]")
}

fn render_type_answer_placeholder(field_name: &str) -> String {
    format!("[type answer: {field_name}]")
}

pub fn typed_answer_for_template(
    template: &str,
    field_values: &HashMap<String, String>,
    cloze_context: Option<(u32, ClozeRenderSide)>,
) -> Option<TypeAnswerSpec> {
    find_type_answer_tag(template, field_values, cloze_context)
}

pub fn normalize_type_answer(value: &str, ignore_combining: bool) -> String {
    let text = html_to_text(value);
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if ignore_combining {
        normalize_without_combining_marks(&collapsed)
    } else {
        collapsed.to_lowercase()
    }
}

pub fn type_answer_matches(input: &str, spec: &TypeAnswerSpec) -> bool {
    normalize_type_answer(input, spec.ignore_combining) == spec.normalized_expected
}

fn find_type_answer_tag(
    template: &str,
    field_values: &HashMap<String, String>,
    cloze_context: Option<(u32, ClozeRenderSide)>,
) -> Option<TypeAnswerSpec> {
    let mut rest = template;

    while let Some(start) = rest.find("{{") {
        let (_, after_start) = rest.split_at(start);
        let after_start = &after_start[2..];

        let Some(end) = after_start.find("}}") else {
            break;
        };
        let (tag, after_end) = after_start.split_at(end);
        let tag = tag.trim();
        let after_tag = &after_end[2..];

        if let Some(section) = parse_section_tag(tag) {
            let close_tag = format!("{{{{/{}}}}}", section.field_name);
            if let Some(close_start) = after_tag.find(&close_tag) {
                let (body, after_body) = after_tag.split_at(close_start);
                if section_should_render(section, field_values) {
                    if let Some(spec) = find_type_answer_tag(body, field_values, cloze_context) {
                        return Some(spec);
                    }
                }
                rest = &after_body[close_tag.len()..];
                continue;
            }
        }

        if let Some(spec) = type_answer_spec_from_tag(tag, field_values, cloze_context) {
            return Some(spec);
        }
        rest = after_tag;
    }

    None
}

fn type_answer_spec_from_tag(
    tag: &str,
    field_values: &HashMap<String, String>,
    cloze_context: Option<(u32, ClozeRenderSide)>,
) -> Option<TypeAnswerSpec> {
    let (filters, field_name) = parse_template_field_filters(tag);
    if !filters.contains(&TemplateFieldFilter::Type) {
        return None;
    }

    let field_name = field_name.trim();
    if field_name.is_empty() {
        return None;
    }

    let raw_value = field_values.get(field_name)?;
    let ignore_combining = filters.contains(&TemplateFieldFilter::NoCombining);
    let mut expected = raw_value.clone();

    for filter in filters.iter().rev() {
        match filter {
            TemplateFieldFilter::Cloze => {
                expected = cloze_type_answer_text(&expected, cloze_context);
            }
            TemplateFieldFilter::Text => expected = html_to_text(&expected),
            TemplateFieldFilter::Furigana => {
                expected = render_ruby_text(&expected, RubyMode::Furigana);
            }
            TemplateFieldFilter::Kana => expected = render_ruby_text(&expected, RubyMode::Kana),
            TemplateFieldFilter::Kanji => expected = render_ruby_text(&expected, RubyMode::Kanji),
            TemplateFieldFilter::Hint
            | TemplateFieldFilter::Type
            | TemplateFieldFilter::NoCombining => {}
        }
    }

    let normalized_expected = normalize_type_answer(&expected, ignore_combining);
    Some(TypeAnswerSpec {
        field_name: field_name.to_string(),
        expected,
        normalized_expected,
        ignore_combining,
    })
}

fn cloze_type_answer_text(value: &str, cloze_context: Option<(u32, ClozeRenderSide)>) -> String {
    let Some((cloze_ordinal, _)) = cloze_context else {
        return render_cloze_text(value, 0, ClozeRenderSide::Answer);
    };

    let mut answers = Vec::new();
    collect_cloze_type_answers(value, cloze_ordinal, &mut answers);
    if answers.is_empty() {
        render_cloze_text(value, cloze_ordinal, ClozeRenderSide::Answer)
    } else {
        answers.join(", ")
    }
}

fn collect_cloze_type_answers(value: &str, cloze_ordinal: u32, answers: &mut Vec<String>) {
    let mut rest = value;

    while let Some(start) = rest.find("{{c") {
        let (_, candidate) = rest.split_at(start);
        if let Some(marker) = parse_cloze_marker(candidate) {
            if marker.ordinal == cloze_ordinal {
                answers.push(render_cloze_text(
                    marker.hidden,
                    cloze_ordinal,
                    ClozeRenderSide::Answer,
                ));
            } else {
                collect_cloze_type_answers(marker.hidden, cloze_ordinal, answers);
            }
            rest = &candidate[marker.consumed..];
        } else {
            rest = &candidate[3..];
        }
    }
}

fn normalize_without_combining_marks(value: &str) -> String {
    let mut normalized = String::new();
    for ch in value.nfd() {
        if is_combining_mark(ch) {
            continue;
        }
        match ch {
            'ß' | 'ẞ' => normalized.push_str("ss"),
            _ => normalized.extend(ch.to_lowercase()),
        }
    }
    normalized
}

// Explicit loop with multiple break conditions reads clearer than while-let (allow 1.97 while_let_loop).
#[allow(clippy::while_let_loop)]
fn parse_template_field_filters(tag: &str) -> (Vec<TemplateFieldFilter>, &str) {
    let mut filters = Vec::new();
    let mut rest = tag.trim();

    loop {
        let Some((filter, after_filter)) = parse_template_field_filter(rest) else {
            break;
        };
        filters.push(filter);
        rest = after_filter.trim_start();
    }

    (filters, rest)
}

fn parse_template_field_filter(tag: &str) -> Option<(TemplateFieldFilter, &str)> {
    const FILTERS: [TemplateFieldFilter; 8] = [
        TemplateFieldFilter::Cloze,
        TemplateFieldFilter::Hint,
        TemplateFieldFilter::Type,
        TemplateFieldFilter::NoCombining,
        TemplateFieldFilter::Text,
        TemplateFieldFilter::Furigana,
        TemplateFieldFilter::Kana,
        TemplateFieldFilter::Kanji,
    ];

    FILTERS.iter().find_map(|filter| {
        tag.strip_prefix(filter.prefix())
            .map(|rest| (*filter, rest))
    })
}

fn html_to_text(value: &str) -> String {
    decode_html_entities(&strip_html_tags(value))
}

fn strip_html_tags(value: &str) -> String {
    let mut stripped = String::with_capacity(value.len());
    let mut rest = value;

    while let Some(start) = rest.find('<') {
        let (prefix, after_start) = rest.split_at(start);
        stripped.push_str(prefix);
        let after_start = &after_start[1..];
        let Some(end) = after_start.find('>') else {
            stripped.push('<');
            stripped.push_str(after_start);
            return stripped;
        };

        let tag_name = after_start[..end]
            .trim_start_matches('/')
            .split_whitespace()
            .next()
            .map(|name| name.trim_end_matches('/'));
        if tag_name.is_some_and(|name| name.eq_ignore_ascii_case("br")) {
            stripped.push('\n');
        }
        rest = &after_start[end + 1..];
    }

    stripped.push_str(rest);
    stripped
}

fn decode_html_entities(value: &str) -> String {
    let mut decoded = String::with_capacity(value.len());
    let mut rest = value;

    while let Some(start) = rest.find('&') {
        let (prefix, after_start) = rest.split_at(start);
        decoded.push_str(prefix);
        let after_start = &after_start[1..];
        let Some(end) = after_start.find(';') else {
            decoded.push('&');
            decoded.push_str(after_start);
            return decoded;
        };

        let entity = &after_start[..end];
        if let Some(ch) = decode_html_entity(entity) {
            decoded.push(ch);
        } else {
            decoded.push('&');
            decoded.push_str(entity);
            decoded.push(';');
        }
        rest = &after_start[end + 1..];
    }

    decoded.push_str(rest);
    decoded
}

fn decode_html_entity(entity: &str) -> Option<char> {
    match entity {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" | "#39" => Some('\''),
        "nbsp" => Some(' '),
        _ => {
            let digits = entity
                .strip_prefix("#x")
                .or_else(|| entity.strip_prefix("#X"));
            if let Some(digits) = digits {
                u32::from_str_radix(digits, 16)
                    .ok()
                    .and_then(char::from_u32)
            } else if let Some(digits) = entity.strip_prefix('#') {
                digits.parse::<u32>().ok().and_then(char::from_u32)
            } else {
                None
            }
        }
    }
}

fn render_ruby_text(value: &str, mode: RubyMode) -> String {
    let mut rendered = String::with_capacity(value.len());
    let mut index = 0;

    while index < value.len() {
        let ch = value[index..]
            .chars()
            .next()
            .expect("index stays on char boundary");
        if ch == '[' {
            let after_open = index + ch.len_utf8();
            if let Some(close) = value[after_open..].find(']') {
                if let Some(base) = take_ruby_base(&mut rendered) {
                    let ruby = &value[after_open..after_open + close];
                    match mode {
                        RubyMode::Furigana => {
                            rendered.push_str("<ruby>");
                            rendered.push_str(&base);
                            rendered.push_str("<rt>");
                            rendered.push_str(ruby);
                            rendered.push_str("</rt></ruby>");
                        }
                        RubyMode::Kana => rendered.push_str(ruby),
                        RubyMode::Kanji => rendered.push_str(&base),
                    }
                    index = after_open + close + 1;
                    continue;
                }
            }
        }

        rendered.push(ch);
        index += ch.len_utf8();
    }

    rendered
}

fn take_ruby_base(rendered: &mut String) -> Option<String> {
    let mut start = rendered.len();
    for (index, ch) in rendered.char_indices().rev() {
        if ch.is_whitespace() {
            break;
        }
        start = index;
    }

    if start == rendered.len() {
        None
    } else {
        let base = rendered[start..].to_string();
        rendered.truncate(start);
        Some(base)
    }
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

fn template_deck_id<'a>(template: &'a CardTemplate, note: &'a Note) -> &'a str {
    template.deck_id.as_deref().unwrap_or(&note.deck_id)
}

fn insert_special_template_values(
    field_values: &mut HashMap<String, String>,
    note_type: &NoteType,
    note: &Note,
    template: &CardTemplate,
    card_id: &str,
    deck_id: &str,
) {
    field_values
        .entry("Tags".to_string())
        .or_insert_with(|| note.tags.join(" "));
    field_values
        .entry("Type".to_string())
        .or_insert_with(|| note_type.name.clone());
    field_values
        .entry("Deck".to_string())
        .or_insert_with(|| deck_id.to_string());
    field_values
        .entry("Subdeck".to_string())
        .or_insert_with(|| subdeck_name(deck_id).to_string());
    field_values
        .entry("Card".to_string())
        .or_insert_with(|| template.name.clone());
    field_values
        .entry("CardFlag".to_string())
        .or_insert_with(|| "flag0".to_string());
    field_values
        .entry("CardID".to_string())
        .or_insert_with(|| card_id.to_string());
}

fn template_requirement_satisfied(
    template: &CardTemplate,
    field_values: &HashMap<String, String>,
) -> bool {
    if template.required_field_names.is_empty() {
        return true;
    }

    let field_is_nonempty = |field_name: &String| {
        field_values
            .get(field_name)
            .is_some_and(|value| !value.trim().is_empty())
    };

    match template.requirement_mode {
        TemplateRequirementMode::All => template.required_field_names.iter().all(field_is_nonempty),
        TemplateRequirementMode::Any => template.required_field_names.iter().any(field_is_nonempty),
    }
}

fn subdeck_name(deck_name: &str) -> &str {
    deck_name
        .rsplit_once("::")
        .map_or(deck_name, |(_, subdeck)| subdeck)
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

    for prefix in ["#", "^", "/"] {
        if let Some(field_name) = trimmed.strip_prefix(prefix) {
            if field_name.trim() == old_name {
                return format!("{prefix}{new_name}");
            }
        }
    }

    let (filters, field_name) = parse_template_field_filters(trimmed);
    if !filters.is_empty() && field_name.trim() == old_name {
        let mut renamed = String::new();
        for filter in filters {
            renamed.push_str(filter.prefix());
        }
        renamed.push_str(new_name);
        return renamed;
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
                let (tag, after_end) = after_start.split_at(end);
                let (filters, field_name) = parse_template_field_filters(tag);
                if filters.contains(&TemplateFieldFilter::Cloze) {
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

pub fn render_cloze_template(
    template: &str,
    field_values: &HashMap<String, String>,
    cloze_ordinal: u32,
    side: ClozeRenderSide,
) -> String {
    render_cloze_template_with_front_side(template, field_values, cloze_ordinal, side, "")
}

pub fn render_cloze_template_with_front_side(
    template: &str,
    field_values: &HashMap<String, String>,
    cloze_ordinal: u32,
    side: ClozeRenderSide,
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
                            rendered.push_str(&render_cloze_template_with_front_side(
                                body,
                                field_values,
                                cloze_ordinal,
                                side,
                                front_side,
                            ));
                        }
                        rest = &after_body[close_tag.len()..];
                        continue;
                    }
                }

                rendered.push_str(&render_cloze_template_tag(
                    tag,
                    field_values,
                    cloze_ordinal,
                    side,
                    front_side,
                ));
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

fn render_cloze_template_tag(
    tag: &str,
    field_values: &HashMap<String, String>,
    cloze_ordinal: u32,
    side: ClozeRenderSide,
    front_side: &str,
) -> String {
    if tag == "FrontSide" {
        return front_side.to_string();
    }

    render_template_field_tag(tag, field_values, Some((cloze_ordinal, side)))
}

pub fn template_references_cloze(front_template: &str, back_template: &str) -> bool {
    !cloze_field_names_for_template(front_template, back_template).is_empty()
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

fn render_cloze_text(value: &str, cloze_ordinal: u32, side: ClozeRenderSide) -> String {
    let mut rendered = String::with_capacity(value.len());
    let mut rest = value;

    while let Some(start) = rest.find("{{c") {
        let (prefix, candidate) = rest.split_at(start);
        rendered.push_str(prefix);

        if let Some(marker) = parse_cloze_marker(candidate) {
            if side == ClozeRenderSide::Question && marker.ordinal == cloze_ordinal {
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
                    ClozeRenderSide::Answer,
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
                    deck_id: None,
                    required_field_names: vec!["Front".to_string(), "Back".to_string()],
                    requirement_mode: TemplateRequirementMode::All,
                    ordinal: 0,
                },
                CardTemplate {
                    id: "reverse".to_string(),
                    name: "Reverse".to_string(),
                    front_template: "{{Back}}".to_string(),
                    back_template: "{{Front}}".to_string(),
                    deck_id: None,
                    required_field_names: vec!["Front".to_string(), "Back".to_string()],
                    requirement_mode: TemplateRequirementMode::All,
                    ordinal: 1,
                },
            ],
            stylesheet: None,
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
                deck_id: None,
                required_field_names: vec!["Text".to_string()],
                requirement_mode: TemplateRequirementMode::All,
                ordinal: 0,
            }],
            stylesheet: None,
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
    fn any_required_field_can_generate_from_one_nonempty_field() {
        let mut note_type = basic_note_type();
        note_type.templates[0].front_template = "{{Front}}{{Back}}".to_string();
        note_type.templates[0].required_field_names = vec!["Front".to_string(), "Back".to_string()];
        note_type.templates[0].requirement_mode = TemplateRequirementMode::Any;
        let note = note("letter-a", "  ");

        let cards = generate_cards_for_note(&note_type, &note);

        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].id, "note-1::forward");
        assert_eq!(cards[0].front, "letter-a  ");
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
    fn template_deck_override_controls_generated_card_deck() {
        let mut note_type = basic_note_type();
        note_type.templates[1].deck_id = Some("Languages::Tamil".to_string());
        note_type.templates[1].front_template = "{{Deck}}|{{Subdeck}}|{{Back}}".to_string();

        let generated = generate_cards_for_note(&note_type, &note("letter-a", "a"));
        let reverse = generated
            .iter()
            .find(|card| card.template_id == "reverse")
            .expect("reverse card");
        let materialized = materialize_generated_card(reverse, NOW);

        assert_eq!(reverse.deck_id, "Languages::Tamil");
        assert_eq!(reverse.front, "Languages::Tamil|Tamil|a");
        assert_eq!(materialized.deck_id, "Languages::Tamil");
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
        let hint = render_template("{{hint:Back}}", &values);
        assert_eq!(hint, "[show hint: Back]");
        assert!(!hint.contains("hello"));
        assert_eq!(
            render_template_with_front_side("{{FrontSide}}<hr>{{Back}}", &values, "hola"),
            "hola<hr>hello"
        );
    }

    #[test]
    fn anki_style_field_filters_render_text_and_ruby_variants() {
        let mut values = HashMap::new();
        values.insert(
            "Expression".to_string(),
            "<b>amiga</b> &amp; amigo<br/>root&nbsp;word".to_string(),
        );
        values.insert(
            "Reading".to_string(),
            "root[ruut] stem[stem]ling".to_string(),
        );

        assert_eq!(
            render_template("{{text:Expression}}", &values),
            "amiga & amigo\nroot word"
        );
        assert_eq!(
            render_template("{{furigana:Reading}}", &values),
            "<ruby>root<rt>ruut</rt></ruby> <ruby>stem<rt>stem</rt></ruby>ling"
        );
        assert_eq!(
            render_template("{{kana:Reading}}", &values),
            "ruut stemling"
        );
        assert_eq!(
            render_template("{{kanji:Reading}}", &values),
            "root stemling"
        );
        let type_placeholder = render_template("{{type:nc:Expression}}", &values);
        assert_eq!(type_placeholder, "[type answer: Expression]");
        assert!(!type_placeholder.contains("amiga"));

        let spec = typed_answer_for_template("{{type:nc:Expression}}", &values, None).unwrap();
        assert_eq!(spec.expected, "<b>amiga</b> &amp; amigo<br/>root&nbsp;word");
        assert_eq!(spec.normalized_expected, "amiga & amigo root word");
        assert!(type_answer_matches("amiga & amigo root word", &spec));

        values.insert("Accent".to_string(), "caf\u{e9}".to_string());
        let accent_spec = typed_answer_for_template("{{type:nc:Accent}}", &values, None).unwrap();
        assert!(type_answer_matches("cafe", &accent_spec));
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
    fn generated_cards_render_anki_special_template_values() {
        let mut note_type = basic_note_type();
        note_type.templates[0].name = "Forward".to_string();
        note_type.templates[0].front_template =
            "{{Tags}}|{{Type}}|{{Deck}}|{{Subdeck}}|{{Card}}|{{CardFlag}}|{{CardID}}".to_string();
        note_type.templates[0].back_template = "{{Back}}".to_string();
        let mut note = note("letter-a", "a");
        note.deck_id = "Languages::Tamil".to_string();
        note.tags = vec!["script".to_string(), "vowel".to_string()];

        let cards = generate_cards_for_note(&note_type, &note);

        assert_eq!(
            cards[0].front,
            "script vowel|Basic and reversed|Languages::Tamil|Tamil|Forward|flag0|note-1::forward"
        );
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
            "{{#Front}}{{hint:Front}}{{/Front}}{{^Back}}{{type:nc:Front}}{{/Back}}".to_string();
        note_type.templates[0].back_template =
            "{{FrontSide}}<hr>{{cloze:Front}} {{text:Front}} {{furigana:Front}}".to_string();

        let renamed = rename_note_type_field(&note_type, "front", "Prompt", NOW + 1);

        assert_eq!(
            renamed.templates[0].front_template,
            "{{#Prompt}}{{hint:Prompt}}{{/Prompt}}{{^Back}}{{type:nc:Prompt}}{{/Back}}"
        );
        assert_eq!(
            renamed.templates[0].back_template,
            "{{FrontSide}}<hr>{{cloze:Prompt}} {{text:Prompt}} {{furigana:Prompt}}"
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
    fn filtered_cloze_tags_generate_and_render_cards() {
        let mut note_type = cloze_note_type();
        note_type.templates[0].front_template =
            "{{#Extra}}{{type:cloze:Text}}{{/Extra}}{{^Extra}}missing{{/Extra}}".to_string();
        note_type.templates[0].back_template =
            "{{FrontSide}}<hr>{{text:cloze:Text}}<br>{{Extra}}".to_string();

        let cards = generate_cards_for_note(
            &note_type,
            &cloze_note("A <b>{{c1::root::base}}</b> carries meaning.", "etymology"),
        );

        assert!(template_references_cloze(
            &note_type.templates[0].front_template,
            &note_type.templates[0].back_template
        ));
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].front, "[type answer: Text]");
        assert_eq!(
            cards[0].back,
            "[type answer: Text]<hr>A root carries meaning.<br>etymology"
        );
        let mut field_values = HashMap::new();
        field_values.insert(
            "Text".to_string(),
            "A <b>{{c1::root::base}}</b> carries meaning.".to_string(),
        );
        field_values.insert("Extra".to_string(), "etymology".to_string());
        let spec = typed_answer_for_template(
            &note_type.templates[0].front_template,
            &field_values,
            Some((1, ClozeRenderSide::Question)),
        )
        .unwrap();
        assert_eq!(spec.expected, "root");
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
        let rendered =
            render_cloze_template("{{cloze:Text}}", &values, 1, ClozeRenderSide::Question);

        assert_eq!(rendered, "[...] {{cx::kept}}");
    }
}
