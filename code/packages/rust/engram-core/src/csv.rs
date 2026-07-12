use crate::model::{
    Card, CardTemplate, Deck, ExternalSourceRecord, ExternalSourceTarget, FieldDef, Note,
    NoteFieldValue, NoteType, TemplateRequirementMode,
};
use crate::template::{generate_cards_for_note, materialize_generated_card};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const CARD_CSV_HEADER: [&str; 5] = ["id", "deckId", "front", "back", "createdAt"];
const BASIC_CARD_CSV_HEADER: [&str; 2] = ["front", "back"];

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct CsvError {
    pub message: String,
    pub row: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct BasicCardCsvImportOptions {
    pub deck_id: String,
    pub id_prefix: String,
    pub created_at: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct AnkiNoteTsvImportOptions {
    pub deck_id: String,
    pub note_type_id: String,
    pub note_type_name: String,
    pub note_id_prefix: String,
    pub created_at: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct AnkiNoteTsvImport {
    pub note_types: Vec<NoteType>,
    pub notes: Vec<Note>,
    pub cards: Vec<Card>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub external_sources: Vec<ExternalSourceRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnkiBasicTsvExportOptions {
    pub deck_name: String,
    pub note_type_name: String,
    pub html: bool,
    pub include_headers: bool,
}

impl Default for AnkiBasicTsvExportOptions {
    fn default() -> Self {
        Self {
            deck_name: "Engram".to_string(),
            note_type_name: "Basic".to_string(),
            html: false,
            include_headers: true,
        }
    }
}

pub fn export_cards_csv(cards: &[Card]) -> String {
    let mut output = String::new();
    output.push_str(&CARD_CSV_HEADER.join(","));
    output.push('\n');

    for card in cards {
        let created_at = card.created_at.to_string();
        write_csv_row(
            &mut output,
            &[
                card.id.as_str(),
                card.deck_id.as_str(),
                card.front.as_str(),
                card.back.as_str(),
                created_at.as_str(),
            ],
        );
        output.push('\n');
    }

    output
}

pub fn export_cards_anki_basic_tsv(cards: &[Card], options: &AnkiBasicTsvExportOptions) -> String {
    let mut output = String::new();
    if options.include_headers {
        output.push_str("#separator:tab\n");
        output.push_str(if options.html {
            "#html:true\n"
        } else {
            "#html:false\n"
        });
        output.push_str("#notetype:");
        output.push_str(&header_value(&options.note_type_name));
        output.push('\n');
        output.push_str("#deck:");
        output.push_str(&header_value(&options.deck_name));
        output.push('\n');
        output.push_str("#columns:Front\tBack\n");
    }

    for card in cards {
        write_tsv_row(&mut output, &[card.front.as_str(), card.back.as_str()]);
    }

    output
}

pub fn export_notes_anki_tsv(
    note_type: &NoteType,
    notes: &[Note],
    options: &AnkiBasicTsvExportOptions,
) -> String {
    export_notes_anki_tsv_with_context(note_type, notes, &[], &[], options)
}

pub fn export_notes_anki_tsv_with_context(
    note_type: &NoteType,
    notes: &[Note],
    decks: &[Deck],
    external_sources: &[ExternalSourceRecord],
    options: &AnkiBasicTsvExportOptions,
) -> String {
    let mut output = String::new();
    let note_type_name = if options.note_type_name.trim().is_empty() {
        note_type.name.as_str()
    } else {
        options.note_type_name.as_str()
    };
    let default_deck_name = options.deck_name.trim();
    let deck_names = deck_names_by_id(decks);
    let row_deck_names: Vec<String> = notes
        .iter()
        .map(|note| note_export_deck_name(note, &deck_names))
        .collect();
    let include_deck_column = !decks.is_empty()
        && row_deck_names
            .iter()
            .any(|deck_name| deck_name != default_deck_name);
    let note_guids = note_guids_by_id(external_sources);
    let include_guid_column = notes.iter().any(|note| note_guids.contains_key(&note.id));
    let guid_column = note_type.fields.len() + 1;
    let deck_column = guid_column + usize::from(include_guid_column);

    if options.include_headers {
        output.push_str("#separator:tab\n");
        output.push_str(if options.html {
            "#html:true\n"
        } else {
            "#html:false\n"
        });
        output.push_str("#notetype:");
        output.push_str(&header_value(note_type_name));
        output.push('\n');
        output.push_str("#deck:");
        output.push_str(&header_value(&options.deck_name));
        output.push('\n');
        if include_guid_column {
            output.push_str("#guid column:");
            output.push_str(&guid_column.to_string());
            output.push('\n');
        }
        if include_deck_column {
            output.push_str("#deck column:");
            output.push_str(&deck_column.to_string());
            output.push('\n');
        }
        output.push_str("#columns:");
        for (index, field) in note_type.fields.iter().enumerate() {
            if index > 0 {
                output.push('\t');
            }
            write_tsv_field(&mut output, &field.name);
        }
        if include_guid_column {
            output.push('\t');
            write_tsv_field(&mut output, "Guid");
        }
        if include_deck_column {
            output.push('\t');
            write_tsv_field(&mut output, "Deck");
        }
        output.push_str("\tTags\n");
    }

    for (note, row_deck_name) in notes.iter().zip(row_deck_names.iter()) {
        let mut field_values = Vec::new();
        for field in &note_type.fields {
            let value = note
                .fields
                .iter()
                .find(|candidate| candidate.field_id == field.id)
                .map_or_else(String::new, |field| field.value.clone());
            field_values.push(value);
        }
        if include_guid_column {
            field_values.push(note_guids.get(&note.id).cloned().unwrap_or_default());
        }
        if include_deck_column {
            field_values.push(row_deck_name.clone());
        }
        let tags = note.tags.join(" ");
        field_values.push(tags);
        let field_refs: Vec<&str> = field_values.iter().map(String::as_str).collect();
        write_tsv_row(&mut output, &field_refs);
    }

    output
}

fn deck_names_by_id(decks: &[Deck]) -> BTreeMap<String, String> {
    decks
        .iter()
        .map(|deck| {
            let name = if deck.name.trim().is_empty() {
                deck.id.clone()
            } else {
                deck.name.clone()
            };
            (deck.id.clone(), name)
        })
        .collect()
}

fn note_export_deck_name(note: &Note, deck_names: &BTreeMap<String, String>) -> String {
    deck_names
        .get(&note.deck_id)
        .cloned()
        .unwrap_or_else(|| note.deck_id.clone())
}

fn note_guids_by_id(external_sources: &[ExternalSourceRecord]) -> BTreeMap<String, String> {
    let mut guids = BTreeMap::new();
    for source in external_sources {
        if source.target != ExternalSourceTarget::Note {
            continue;
        }
        if !(source.source == "anki-text" || source.source.starts_with("anki-")) {
            continue;
        }
        let Some(guid) = source_note_guid(source) else {
            continue;
        };
        guids.entry(source.target_id.clone()).or_insert(guid);
    }
    guids
}

fn source_note_guid(source: &ExternalSourceRecord) -> Option<String> {
    source
        .data
        .get("guid")
        .map(|guid| guid.trim())
        .filter(|guid| !guid.is_empty())
        .or_else(|| source.original_id.as_deref().map(str::trim))
        .filter(|guid| !guid.is_empty())
        .map(str::to_string)
}

pub fn import_cards_csv(input: &str) -> Result<Vec<Card>, CsvError> {
    let records = parse_csv_records(input)?;
    if records.is_empty() {
        return Err(csv_error("card CSV is missing a header row", None));
    }
    if !matches_csv_header(&records[0], &CARD_CSV_HEADER) {
        return Err(csv_error(
            "card CSV header must be id,deckId,front,back,createdAt",
            Some(1),
        ));
    }

    records
        .into_iter()
        .enumerate()
        .skip(1)
        .filter(|(_, fields)| !is_blank_record(fields))
        .map(|(index, fields)| card_from_fields(index + 1, fields))
        .collect()
}

pub fn import_basic_cards_csv(
    input: &str,
    options: &BasicCardCsvImportOptions,
) -> Result<Vec<Card>, CsvError> {
    let records = parse_csv_records(input)?;
    if records.is_empty() {
        return Err(csv_error("basic card CSV is missing a header row", None));
    }
    if !matches_csv_header(&records[0], &BASIC_CARD_CSV_HEADER) {
        return Err(csv_error(
            "basic card CSV header must be front,back",
            Some(1),
        ));
    }

    let mut cards = Vec::new();
    for (index, fields) in records.into_iter().enumerate().skip(1) {
        if is_blank_record(&fields) {
            continue;
        }
        let sequence = cards.len() + 1;
        cards.push(basic_card_from_fields(
            index + 1,
            sequence,
            fields,
            options,
        )?);
    }

    Ok(cards)
}

pub fn import_anki_basic_tsv(
    input: &str,
    options: &BasicCardCsvImportOptions,
) -> Result<Vec<Card>, CsvError> {
    let records = parse_anki_text_records(input)?;
    let mut cards = Vec::new();
    let mut html = false;

    for (index, fields) in records.into_iter().enumerate() {
        if is_blank_record(&fields) {
            continue;
        }
        if let Some(first) = fields.first().filter(|field| field.starts_with('#')) {
            if let Some(value) = first.strip_prefix("#html:") {
                html = parse_anki_header_bool(value, index + 1)?;
            }
            continue;
        }
        if fields.len() < 2 {
            return Err(csv_error(
                &format!(
                    "Anki TSV row must have at least 2 fields, found {}",
                    fields.len()
                ),
                Some(index + 1),
            ));
        }

        let sequence = cards.len() + 1;
        cards.push(Card {
            id: generated_basic_card_id(&options.id_prefix, sequence),
            deck_id: options.deck_id.clone(),
            front: anki_import_field_value(&fields[0], html),
            back: anki_import_field_value(&fields[1], html),
            created_at: options.created_at,
            lineage: None,
        });
    }

    Ok(cards)
}

pub fn import_anki_notes_tsv(
    input: &str,
    options: &AnkiNoteTsvImportOptions,
) -> Result<AnkiNoteTsvImport, CsvError> {
    let records = parse_anki_text_records(input)?;
    let mut headers = AnkiTsvHeaders::default();
    let mut rows = Vec::new();

    for (index, fields) in records.into_iter().enumerate() {
        if is_blank_record(&fields) {
            continue;
        }
        if fields.first().is_some_and(|field| field.starts_with('#')) {
            headers.read_header(&fields, index + 1)?;
            continue;
        }
        rows.push((index + 1, fields));
    }

    let note_type_name = headers
        .note_type_name
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            if options.note_type_name.trim().is_empty() {
                "Basic".to_string()
            } else {
                options.note_type_name.clone()
            }
        });
    let header_tags = headers.tags.clone();
    let default_import_kind = AnkiNoteImportKind::from_note_type_name(&note_type_name);
    let columns = headers
        .columns
        .clone()
        .unwrap_or_else(|| default_import_kind.default_columns(&rows));
    let use_note_type_column = headers.note_type_column.is_some();

    let mut note_types = Vec::new();
    let mut note_type_positions = BTreeMap::new();
    let mut notes = Vec::new();
    let mut cards = Vec::new();
    let mut external_sources = Vec::new();

    for (row, fields) in rows {
        let row_note_type_name =
            note_type_name_for_row(&note_type_name, headers.note_type_column, &fields);
        let row_note_type_id = if use_note_type_column || options.note_type_id.trim().is_empty() {
            default_note_type_id(&row_note_type_name)
        } else {
            options.note_type_id.clone()
        };
        let import_kind = AnkiNoteImportKind::from_note_type_name(&row_note_type_name);
        let column_plan = import_kind.column_plan(
            &columns,
            headers.tags_column,
            headers.guid_column,
            headers.deck_column,
            headers.note_type_column,
        )?;
        if fields.len() < column_plan.required_columns {
            return Err(csv_error(
                &format!(
                    "Anki note TSV row must have at least {} fields, found {}",
                    column_plan.required_columns,
                    fields.len()
                ),
                Some(row),
            ));
        }

        let note_type_index = if let Some(index) = note_type_positions.get(&row_note_type_id) {
            *index
        } else {
            let index = note_types.len();
            note_types.push(import_kind.note_type(
                &row_note_type_id,
                &row_note_type_name,
                options.created_at,
                &column_plan,
            ));
            note_type_positions.insert(row_note_type_id.clone(), index);
            index
        };
        let note_type = &note_types[note_type_index];
        let sequence = notes.len() + 1;
        let deck_id = note_deck_id_for_row(
            &options.deck_id,
            headers.deck_name.as_deref(),
            &column_plan,
            &fields,
        );
        let note = Note {
            id: generated_basic_card_id(&options.note_id_prefix, sequence),
            note_type_id: note_type.id.clone(),
            deck_id,
            fields: column_plan
                .field_columns
                .iter()
                .map(|field| NoteFieldValue {
                    field_id: field.field_id.clone(),
                    value: anki_import_field_value(
                        &fields[field.column_index],
                        headers.html.unwrap_or(false),
                    ),
                })
                .collect(),
            tags: note_tags_for_row(&header_tags, &column_plan, &fields),
            created_at: options.created_at,
            updated_at: options.created_at,
        };

        cards.extend(
            generate_cards_for_note(note_type, &note)
                .iter()
                .map(|generated| materialize_generated_card(generated, options.created_at)),
        );
        if let Some(source) = note_guid_source_for_row(&note, &column_plan, &fields) {
            external_sources.push(source);
        }
        notes.push(note);
    }

    Ok(AnkiNoteTsvImport {
        note_types,
        notes,
        cards,
        external_sources,
    })
}

#[derive(Default)]
struct AnkiTsvHeaders {
    note_type_name: Option<String>,
    deck_name: Option<String>,
    columns: Option<Vec<String>>,
    html: Option<bool>,
    tags: Vec<String>,
    tags_column: Option<usize>,
    guid_column: Option<usize>,
    deck_column: Option<usize>,
    note_type_column: Option<usize>,
}

impl AnkiTsvHeaders {
    fn read_header(&mut self, fields: &[String], row: usize) -> Result<(), CsvError> {
        let Some(first) = fields.first() else {
            return Ok(());
        };

        if let Some(note_type_name) = first.strip_prefix("#notetype:") {
            self.note_type_name = Some(note_type_name.trim().to_string());
            return Ok(());
        }

        if let Some(deck_name) = first.strip_prefix("#deck:") {
            self.deck_name = Some(deck_name.trim().to_string());
            return Ok(());
        }

        if let Some(html) = first.strip_prefix("#html:") {
            self.html = Some(parse_anki_header_bool(html, row)?);
            return Ok(());
        }

        if let Some(tags) = first.strip_prefix("#tags:") {
            self.tags = split_anki_tags(tags);
            return Ok(());
        }

        if let Some(tags_column) = first.strip_prefix("#tags column:") {
            self.tags_column = Some(parse_anki_header_column_index(tags_column, row)?);
            return Ok(());
        }

        if let Some(guid_column) = first.strip_prefix("#guid column:") {
            self.guid_column = Some(parse_anki_header_column_index(guid_column, row)?);
            return Ok(());
        }

        if let Some(deck_column) = first.strip_prefix("#deck column:") {
            self.deck_column = Some(parse_anki_header_column_index(deck_column, row)?);
            return Ok(());
        }

        if let Some(note_type_column) = first.strip_prefix("#notetype column:") {
            self.note_type_column = Some(parse_anki_header_column_index(note_type_column, row)?);
            return Ok(());
        }

        if let Some(first_column) = first.strip_prefix("#columns:") {
            let mut columns = Vec::with_capacity(fields.len());
            columns.push(first_column.trim().to_string());
            columns.extend(fields.iter().skip(1).map(|field| field.trim().to_string()));
            self.columns = Some(columns);
        }

        Ok(())
    }
}

fn note_tags_for_row(
    header_tags: &[String],
    column_plan: &AnkiColumnPlan,
    fields: &[String],
) -> Vec<String> {
    let mut tags = header_tags.to_vec();
    if let Some(row_tags) = column_plan
        .tag_index
        .and_then(|index| fields.get(index))
        .map(|tags| split_anki_tags(tags))
    {
        extend_unique_tags(&mut tags, row_tags);
    }
    tags
}

fn note_guid_source_for_row(
    note: &Note,
    column_plan: &AnkiColumnPlan,
    fields: &[String],
) -> Option<ExternalSourceRecord> {
    let guid = column_plan
        .guid_index
        .and_then(|index| fields.get(index))
        .map(|guid| guid.trim())
        .filter(|guid| !guid.is_empty())?;

    Some(ExternalSourceRecord {
        target: ExternalSourceTarget::Note,
        target_id: note.id.clone(),
        source: "anki-text".to_string(),
        original_id: Some(guid.to_string()),
        data: BTreeMap::from([("guid".to_string(), guid.to_string())]),
    })
}

fn note_deck_id_for_row(
    fallback_deck_id: &str,
    header_deck_id: Option<&str>,
    column_plan: &AnkiColumnPlan,
    fields: &[String],
) -> String {
    column_plan
        .deck_index
        .and_then(|index| fields.get(index))
        .map(|deck| deck.trim())
        .filter(|deck| !deck.is_empty())
        .or_else(|| {
            header_deck_id
                .map(str::trim)
                .filter(|deck| !deck.is_empty())
        })
        .unwrap_or(fallback_deck_id)
        .to_string()
}

fn note_type_name_for_row(
    fallback_note_type_name: &str,
    note_type_column: Option<usize>,
    fields: &[String],
) -> String {
    note_type_column
        .and_then(|index| fields.get(index))
        .map(|note_type| note_type.trim())
        .filter(|note_type| !note_type.is_empty())
        .unwrap_or(fallback_note_type_name)
        .to_string()
}

fn extend_unique_tags(tags: &mut Vec<String>, incoming: Vec<String>) {
    for tag in incoming {
        if !tags.iter().any(|existing| existing == &tag) {
            tags.push(tag);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AnkiNoteImportKind {
    Basic {
        reverse: bool,
        optional_reverse: bool,
        type_answer: bool,
    },
    Cloze,
    Custom,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AnkiColumnPlan {
    field_columns: Vec<AnkiFieldColumn>,
    tag_index: Option<usize>,
    guid_index: Option<usize>,
    deck_index: Option<usize>,
    note_type_index: Option<usize>,
    required_columns: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AnkiFieldColumn {
    field_id: String,
    field_name: String,
    column_index: usize,
}

impl AnkiNoteImportKind {
    fn from_note_type_name(note_type_name: &str) -> Self {
        if is_cloze_note_type(note_type_name) {
            Self::Cloze
        } else if is_basic_note_type(note_type_name) {
            Self::Basic {
                reverse: is_basic_and_reversed(note_type_name),
                optional_reverse: is_basic_optional_reversed(note_type_name),
                type_answer: is_basic_type_answer(note_type_name),
            }
        } else {
            Self::Custom
        }
    }

    fn note_type(
        &self,
        id: &str,
        name: &str,
        created_at: u64,
        column_plan: &AnkiColumnPlan,
    ) -> NoteType {
        match self {
            Self::Basic {
                reverse,
                optional_reverse,
                type_answer,
            } => basic_note_type(
                id,
                name,
                created_at,
                *reverse,
                *optional_reverse,
                *type_answer,
            ),
            Self::Cloze => cloze_note_type(id, name, created_at),
            Self::Custom => custom_note_type(id, name, created_at, column_plan),
        }
    }

    fn default_columns(&self, rows: &[(usize, Vec<String>)]) -> Vec<String> {
        match self {
            Self::Basic {
                optional_reverse, ..
            } => {
                if *optional_reverse {
                    match rows.first().map(|(_, fields)| fields.len()).unwrap_or(0) {
                        len if len >= 4 => vec![
                            "Front".to_string(),
                            "Back".to_string(),
                            "Add Reverse".to_string(),
                            "Tags".to_string(),
                        ],
                        len if len >= 3 => vec![
                            "Front".to_string(),
                            "Back".to_string(),
                            "Add Reverse".to_string(),
                        ],
                        _ => vec!["Front".to_string(), "Back".to_string()],
                    }
                } else if rows.first().is_some_and(|(_, fields)| fields.len() >= 3) {
                    vec!["Front".to_string(), "Back".to_string(), "Tags".to_string()]
                } else {
                    vec!["Front".to_string(), "Back".to_string()]
                }
            }
            Self::Cloze => match rows.first().map(|(_, fields)| fields.len()).unwrap_or(0) {
                len if len >= 3 => {
                    vec!["Text".to_string(), "Extra".to_string(), "Tags".to_string()]
                }
                2 => vec!["Text".to_string(), "Extra".to_string()],
                _ => vec!["Text".to_string()],
            },
            Self::Custom => rows
                .first()
                .map(|(_, fields)| {
                    (0..fields.len())
                        .map(|index| format!("Field {}", index + 1))
                        .collect()
                })
                .unwrap_or_default(),
        }
    }

    fn column_plan(
        &self,
        columns: &[String],
        tags_column: Option<usize>,
        guid_column: Option<usize>,
        deck_column: Option<usize>,
        note_type_column: Option<usize>,
    ) -> Result<AnkiColumnPlan, CsvError> {
        match self {
            Self::Basic {
                optional_reverse, ..
            } => {
                let tag_index = tags_column.or_else(|| column_index(columns, "Tags"));
                let (front_index, back_index, add_reverse_index) = if note_type_column.is_some() {
                    let regular_columns = regular_column_indexes(
                        columns.len(),
                        tag_index,
                        guid_column,
                        deck_column,
                        note_type_column,
                    );
                    let front_index = *regular_columns.first().ok_or_else(|| {
                        csv_error(
                            "Anki Basic note TSV must include at least two regular field columns",
                            Some(1),
                        )
                    })?;
                    let back_index = *regular_columns.get(1).ok_or_else(|| {
                        csv_error(
                            "Anki Basic note TSV must include at least two regular field columns",
                            Some(1),
                        )
                    })?;
                    (front_index, back_index, regular_columns.get(2).copied())
                } else {
                    let front_index = column_index(columns, "Front").ok_or_else(|| {
                        csv_error(
                            "Anki note TSV #columns must include Front and Back",
                            Some(1),
                        )
                    })?;
                    let back_index = column_index(columns, "Back").ok_or_else(|| {
                        csv_error(
                            "Anki note TSV #columns must include Front and Back",
                            Some(1),
                        )
                    })?;
                    (
                        front_index,
                        back_index,
                        column_index(columns, "Add Reverse"),
                    )
                };
                let mut field_columns = vec![
                    AnkiFieldColumn {
                        field_id: "front".to_string(),
                        field_name: "Front".to_string(),
                        column_index: front_index,
                    },
                    AnkiFieldColumn {
                        field_id: "back".to_string(),
                        field_name: "Back".to_string(),
                        column_index: back_index,
                    },
                ];
                if *optional_reverse {
                    if let Some(add_reverse_index) = add_reverse_index {
                        field_columns.push(AnkiFieldColumn {
                            field_id: "add-reverse".to_string(),
                            field_name: "Add Reverse".to_string(),
                            column_index: add_reverse_index,
                        });
                    }
                }
                Ok(column_plan(
                    field_columns,
                    tag_index,
                    guid_column,
                    deck_column,
                    note_type_column,
                ))
            }
            Self::Cloze => {
                let tag_index = tags_column.or_else(|| column_index(columns, "Tags"));
                let (text_index, extra_index) = if note_type_column.is_some() {
                    let regular_columns = regular_column_indexes(
                        columns.len(),
                        tag_index,
                        guid_column,
                        deck_column,
                        note_type_column,
                    );
                    let text_index = *regular_columns.first().ok_or_else(|| {
                        csv_error(
                            "Anki Cloze TSV must include at least one regular field column",
                            Some(1),
                        )
                    })?;
                    (text_index, regular_columns.get(1).copied())
                } else {
                    let text_index = column_index(columns, "Text").ok_or_else(|| {
                        csv_error("Anki Cloze TSV #columns must include Text", Some(1))
                    })?;
                    (text_index, column_index(columns, "Extra"))
                };
                let mut field_columns = vec![AnkiFieldColumn {
                    field_id: "text".to_string(),
                    field_name: "Text".to_string(),
                    column_index: text_index,
                }];
                if let Some(extra_index) = extra_index {
                    field_columns.push(AnkiFieldColumn {
                        field_id: "extra".to_string(),
                        field_name: "Extra".to_string(),
                        column_index: extra_index,
                    });
                }
                Ok(column_plan(
                    field_columns,
                    tag_index,
                    guid_column,
                    deck_column,
                    note_type_column,
                ))
            }
            Self::Custom => custom_column_plan(
                columns,
                tags_column,
                guid_column,
                deck_column,
                note_type_column,
            ),
        }
    }
}

fn column_plan(
    field_columns: Vec<AnkiFieldColumn>,
    tag_index: Option<usize>,
    guid_index: Option<usize>,
    deck_index: Option<usize>,
    note_type_index: Option<usize>,
) -> AnkiColumnPlan {
    let required_columns = field_columns
        .iter()
        .map(|field| field.column_index)
        .chain(tag_index)
        .chain(guid_index)
        .chain(deck_index)
        .chain(note_type_index)
        .max()
        .map_or(0, |index| index + 1);
    AnkiColumnPlan {
        field_columns,
        tag_index,
        guid_index,
        deck_index,
        note_type_index,
        required_columns,
    }
}

fn regular_column_indexes(
    column_count: usize,
    tag_index: Option<usize>,
    guid_index: Option<usize>,
    deck_index: Option<usize>,
    note_type_index: Option<usize>,
) -> Vec<usize> {
    (0..column_count)
        .filter(|index| {
            Some(*index) != tag_index
                && Some(*index) != guid_index
                && Some(*index) != deck_index
                && Some(*index) != note_type_index
        })
        .collect()
}

fn matches_csv_header(fields: &[String], expected: &[&str]) -> bool {
    fields.len() == expected.len()
        && fields
            .iter()
            .zip(expected)
            .all(|(actual, expected)| actual == *expected)
}

fn write_csv_row(output: &mut String, fields: &[&str]) {
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        write_csv_field(output, field);
    }
}

fn write_csv_field(output: &mut String, field: &str) {
    if field.contains([',', '"', '\r', '\n']) {
        output.push('"');
        for ch in field.chars() {
            if ch == '"' {
                output.push('"');
            }
            output.push(ch);
        }
        output.push('"');
    } else {
        output.push_str(field);
    }
}

fn write_tsv_row(output: &mut String, fields: &[&str]) {
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            output.push('\t');
        }
        write_tsv_field(output, field);
    }
    output.push('\n');
}

fn write_tsv_field(output: &mut String, field: &str) {
    if field.contains(['\t', '"', '\r', '\n']) {
        output.push('"');
        for ch in field.chars() {
            if ch == '"' {
                output.push('"');
            }
            output.push(ch);
        }
        output.push('"');
    } else {
        output.push_str(field);
    }
}

fn anki_import_field_value(value: &str, html: bool) -> String {
    if html {
        value.to_string()
    } else {
        escape_html_text(value)
    }
}

fn escape_html_text(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn header_value(value: &str) -> String {
    value.replace(['\r', '\n'], " ")
}

fn parse_csv_records(input: &str) -> Result<Vec<Vec<String>>, CsvError> {
    parse_delimited_records(input, ',', "CSV")
}

fn parse_anki_text_records(input: &str) -> Result<Vec<Vec<String>>, CsvError> {
    let delimiter = anki_text_delimiter(input)?;
    parse_delimited_records(input, delimiter, "Anki text")
}

fn anki_text_delimiter(input: &str) -> Result<char, CsvError> {
    for (index, line) in input.lines().enumerate() {
        let trimmed = line.trim_start_matches('\u{feff}').trim();
        if trimmed.is_empty() {
            continue;
        }
        let Some(value) = trimmed.strip_prefix("#separator:") else {
            break;
        };
        return anki_separator_value(value, index + 1);
    }
    Ok('\t')
}

fn anki_separator_value(value: &str, row: usize) -> Result<char, CsvError> {
    let trimmed = value.trim();
    let normalized = trimmed.to_ascii_lowercase();
    match normalized.as_str() {
        "tab" | "\\t" => Ok('\t'),
        "comma" => Ok(','),
        "semicolon" => Ok(';'),
        "pipe" => Ok('|'),
        "colon" => Ok(':'),
        "space" => Ok(' '),
        _ if trimmed.chars().count() == 1 => Ok(trimmed.chars().next().unwrap()),
        _ => Err(csv_error(
            &format!("unsupported Anki #separator value: {trimmed}"),
            Some(row),
        )),
    }
}

fn parse_anki_header_column_index(value: &str, row: usize) -> Result<usize, CsvError> {
    let trimmed = value.trim();
    let column = trimmed.parse::<usize>().map_err(|_| {
        csv_error(
            &format!("Anki special column header must use a positive column number: {trimmed}"),
            Some(row),
        )
    })?;
    if column == 0 {
        return Err(csv_error(
            "Anki special column header must use a positive column number",
            Some(row),
        ));
    }
    Ok(column - 1)
}

fn parse_anki_header_bool(value: &str, row: usize) -> Result<bool, CsvError> {
    let trimmed = value.trim();
    match trimmed.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" => Ok(true),
        "false" | "0" | "no" => Ok(false),
        _ => Err(csv_error(
            &format!("Anki #html header must be true or false: {trimmed}"),
            Some(row),
        )),
    }
}

fn parse_delimited_records(
    input: &str,
    delimiter: char,
    format_name: &str,
) -> Result<Vec<Vec<String>>, CsvError> {
    let mut records = Vec::new();
    let mut record = Vec::new();
    let mut field = String::new();
    let mut chars = input.chars().peekable();
    let mut in_quotes = false;
    let mut after_quote = false;
    let mut row = 1;

    while let Some(ch) = chars.next() {
        if in_quotes {
            match ch {
                '"' => {
                    if chars.peek() == Some(&'"') {
                        chars.next();
                        field.push('"');
                    } else {
                        in_quotes = false;
                        after_quote = true;
                    }
                }
                _ => field.push(ch),
            }
            continue;
        }

        if after_quote {
            match ch {
                ch if ch == delimiter => {
                    record.push(std::mem::take(&mut field));
                    after_quote = false;
                }
                '\n' => {
                    record.push(std::mem::take(&mut field));
                    records.push(std::mem::take(&mut record));
                    after_quote = false;
                    row += 1;
                }
                '\r' => {
                    if chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                    record.push(std::mem::take(&mut field));
                    records.push(std::mem::take(&mut record));
                    after_quote = false;
                    row += 1;
                }
                _ => {
                    return Err(csv_error(
                        &format!("quoted {format_name} field must end before the next character"),
                        Some(row),
                    ));
                }
            }
            continue;
        }

        match ch {
            '"' if field.is_empty() => in_quotes = true,
            '"' => {
                return Err(csv_error(
                    &format!("unexpected quote in {format_name} field"),
                    Some(row),
                ));
            }
            ch if ch == delimiter => record.push(std::mem::take(&mut field)),
            '\n' => {
                record.push(std::mem::take(&mut field));
                records.push(std::mem::take(&mut record));
                row += 1;
            }
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                record.push(std::mem::take(&mut field));
                records.push(std::mem::take(&mut record));
                row += 1;
            }
            _ => field.push(ch),
        }
    }

    if in_quotes {
        return Err(csv_error(
            &format!("unterminated quoted {format_name} field"),
            Some(row),
        ));
    }

    if after_quote || !field.is_empty() || !record.is_empty() {
        record.push(field);
        records.push(record);
    }

    Ok(records)
}

fn card_from_fields(row: usize, fields: Vec<String>) -> Result<Card, CsvError> {
    let [id, deck_id, front, back, created_at]: [String; 5] =
        fields.try_into().map_err(|fields: Vec<String>| {
            csv_error(
                &format!("card CSV row must have 5 fields, found {}", fields.len()),
                Some(row),
            )
        })?;

    let created_at = created_at
        .parse::<u64>()
        .map_err(|_| csv_error("card CSV createdAt must be an unsigned integer", Some(row)))?;

    Ok(Card {
        id,
        deck_id,
        front,
        back,
        created_at,
        lineage: None,
    })
}

fn basic_card_from_fields(
    row: usize,
    sequence: usize,
    fields: Vec<String>,
    options: &BasicCardCsvImportOptions,
) -> Result<Card, CsvError> {
    let [front, back]: [String; 2] = fields.try_into().map_err(|fields: Vec<String>| {
        csv_error(
            &format!(
                "basic card CSV row must have 2 fields, found {}",
                fields.len()
            ),
            Some(row),
        )
    })?;

    Ok(Card {
        id: generated_basic_card_id(&options.id_prefix, sequence),
        deck_id: options.deck_id.clone(),
        front,
        back,
        created_at: options.created_at,
        lineage: None,
    })
}

fn generated_basic_card_id(id_prefix: &str, sequence: usize) -> String {
    if id_prefix.is_empty() {
        sequence.to_string()
    } else {
        format!("{id_prefix}-{sequence}")
    }
}

fn basic_note_type(
    id: &str,
    name: &str,
    created_at: u64,
    reverse: bool,
    optional_reverse: bool,
    type_answer: bool,
) -> NoteType {
    let mut templates = vec![CardTemplate {
        id: "forward".to_string(),
        name: "Forward".to_string(),
        front_template: if type_answer {
            "{{Front}}{{type:Back}}".to_string()
        } else {
            "{{Front}}".to_string()
        },
        back_template: if type_answer {
            "{{FrontSide}}<hr id=answer>{{Back}}".to_string()
        } else {
            "{{Back}}".to_string()
        },
        deck_id: None,
        required_field_names: vec!["Front".to_string(), "Back".to_string()],
        requirement_mode: TemplateRequirementMode::All,
        ordinal: 0,
    }];

    if reverse {
        templates.push(CardTemplate {
            id: "reverse".to_string(),
            name: "Reverse".to_string(),
            front_template: "{{Back}}".to_string(),
            back_template: "{{Front}}".to_string(),
            deck_id: None,
            required_field_names: if optional_reverse {
                vec![
                    "Front".to_string(),
                    "Back".to_string(),
                    "Add Reverse".to_string(),
                ]
            } else {
                vec!["Front".to_string(), "Back".to_string()]
            },
            requirement_mode: TemplateRequirementMode::All,
            ordinal: 1,
        });
    }

    let mut fields = vec![
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
    ];
    if optional_reverse {
        fields.push(FieldDef {
            id: "add-reverse".to_string(),
            name: "Add Reverse".to_string(),
            required: false,
            ordinal: 2,
        });
    }

    NoteType {
        id: id.to_string(),
        name: name.to_string(),
        fields,
        templates,
        stylesheet: None,
        created_at,
        updated_at: created_at,
    }
}

fn cloze_note_type(id: &str, name: &str, created_at: u64) -> NoteType {
    NoteType {
        id: id.to_string(),
        name: name.to_string(),
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
        created_at,
        updated_at: created_at,
    }
}

fn custom_note_type(
    id: &str,
    name: &str,
    created_at: u64,
    column_plan: &AnkiColumnPlan,
) -> NoteType {
    NoteType {
        id: id.to_string(),
        name: name.to_string(),
        fields: column_plan
            .field_columns
            .iter()
            .enumerate()
            .map(|(ordinal, field)| FieldDef {
                id: field.field_id.clone(),
                name: field.field_name.clone(),
                required: false,
                ordinal: ordinal as u32,
            })
            .collect(),
        templates: Vec::new(),
        stylesheet: None,
        created_at,
        updated_at: created_at,
    }
}

fn default_note_type_id(note_type_name: &str) -> String {
    let id = slugify_identifier(note_type_name);
    if id.is_empty() {
        "anki-basic".to_string()
    } else {
        id
    }
}

fn slugify_identifier(value: &str) -> String {
    let mut id = String::new();
    let mut last_was_dash = false;

    for ch in value.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            id.push(ch);
            last_was_dash = false;
        } else if !last_was_dash && !id.is_empty() {
            id.push('-');
            last_was_dash = true;
        }
    }

    if id.ends_with('-') {
        id.pop();
    }

    id
}

fn custom_column_plan(
    columns: &[String],
    tags_column: Option<usize>,
    guid_column: Option<usize>,
    deck_column: Option<usize>,
    note_type_column: Option<usize>,
) -> Result<AnkiColumnPlan, CsvError> {
    let tag_index = tags_column.or_else(|| column_index(columns, "Tags"));
    let mut used_ids = BTreeSet::new();
    let mut field_columns = Vec::new();

    for (index, column) in columns.iter().enumerate() {
        if Some(index) == tag_index
            || Some(index) == guid_column
            || Some(index) == deck_column
            || Some(index) == note_type_column
        {
            continue;
        }
        let field_name = custom_field_name(column, index);
        let field_id = unique_custom_field_id(&field_name, index, &mut used_ids);
        field_columns.push(AnkiFieldColumn {
            field_id,
            field_name,
            column_index: index,
        });
    }

    if field_columns.is_empty() {
        return Err(csv_error(
            "Anki custom note TSV #columns must include at least one note field",
            Some(1),
        ));
    }

    Ok(column_plan(
        field_columns,
        tag_index,
        guid_column,
        deck_column,
        note_type_column,
    ))
}

fn custom_field_name(column: &str, index: usize) -> String {
    let trimmed = column.trim();
    if trimmed.is_empty() {
        format!("Field {}", index + 1)
    } else {
        trimmed.to_string()
    }
}

fn unique_custom_field_id(
    field_name: &str,
    index: usize,
    used_ids: &mut BTreeSet<String>,
) -> String {
    let base = {
        let slug = slugify_identifier(field_name);
        if slug.is_empty() {
            format!("field-{}", index + 1)
        } else {
            slug
        }
    };

    if used_ids.insert(base.clone()) {
        return base;
    }

    for suffix in 2.. {
        let candidate = format!("{base}-{suffix}");
        if used_ids.insert(candidate.clone()) {
            return candidate;
        }
    }

    unreachable!("unbounded suffix search must return a unique field id")
}

fn is_basic_and_reversed(note_type_name: &str) -> bool {
    let normalized = note_type_name.to_ascii_lowercase();
    normalized.contains("reversed") || normalized.contains("reverse")
}

fn is_basic_optional_reversed(note_type_name: &str) -> bool {
    let normalized = note_type_name.to_ascii_lowercase();
    normalized.contains("optional")
        && (normalized.contains("reversed") || normalized.contains("reverse"))
}

fn is_basic_type_answer(note_type_name: &str) -> bool {
    let normalized = note_type_name.to_ascii_lowercase();
    normalized.contains("type") && normalized.contains("answer")
}

fn is_cloze_note_type(note_type_name: &str) -> bool {
    note_type_name
        .split(|ch: char| !ch.is_alphanumeric())
        .any(|part| part.eq_ignore_ascii_case("cloze"))
}

fn is_basic_note_type(note_type_name: &str) -> bool {
    let normalized = note_type_name.trim().to_ascii_lowercase();
    normalized == "basic" || normalized.starts_with("basic (")
}

fn column_index(columns: &[String], name: &str) -> Option<usize> {
    columns
        .iter()
        .position(|column| column.trim().eq_ignore_ascii_case(name))
}

fn split_anki_tags(tags: &str) -> Vec<String> {
    tags.split_whitespace()
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn is_blank_record(fields: &[String]) -> bool {
    fields.iter().all(|field| field.trim().is_empty())
}

fn csv_error(message: &str, row: Option<usize>) -> CsvError {
    CsvError {
        message: message.to_string(),
        row,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(id: &str, front: &str, back: &str) -> Card {
        Card {
            id: id.to_string(),
            deck_id: "deck".to_string(),
            front: front.to_string(),
            back: back.to_string(),
            created_at: 123,
            lineage: None,
        }
    }

    #[test]
    fn card_csv_round_trips_with_escaping() {
        let cards = vec![
            card("card-1", "letter-a", "a"),
            card("card-2", "hello, \"friend\"", "line one\nline two"),
        ];

        let csv = export_cards_csv(&cards);
        let restored = import_cards_csv(&csv).unwrap();

        assert_eq!(restored, cards);
        assert!(csv.contains("\"hello, \"\"friend\"\"\""));
        assert!(csv.contains("\"line one\nline two\""));
    }

    #[test]
    fn anki_basic_tsv_export_uses_headers_and_quoted_fields() {
        let cards = vec![
            card("card-1", "letter-a", "a"),
            card("card-2", "hello\t\"friend\"", "line one\nline two"),
        ];
        let options = AnkiBasicTsvExportOptions {
            deck_name: "Tamil::Script".to_string(),
            ..AnkiBasicTsvExportOptions::default()
        };

        let tsv = export_cards_anki_basic_tsv(&cards, &options);

        assert!(tsv.starts_with(
            "#separator:tab\n#html:false\n#notetype:Basic\n#deck:Tamil::Script\n#columns:Front\tBack\n"
        ));
        assert!(tsv.contains("letter-a\ta\n"));
        assert!(tsv.contains("\"hello\t\"\"friend\"\"\"\t\"line one\nline two\"\n"));
    }

    #[test]
    fn anki_basic_tsv_export_can_omit_headers() {
        let cards = vec![card("card-1", "front", "back")];
        let options = AnkiBasicTsvExportOptions {
            include_headers: false,
            ..AnkiBasicTsvExportOptions::default()
        };

        let tsv = export_cards_anki_basic_tsv(&cards, &options);

        assert_eq!(tsv, "front\tback\n");
    }

    #[test]
    fn anki_basic_tsv_import_skips_headers_and_preserves_quoted_fields() {
        let tsv = "#separator:tab\n#html:false\n#notetype:Basic\n#columns:Front\tBack\nletter-a\ta\n\"hello\t\"\"friend\"\"\"\t\"line one\nline two\"\ttag\n";
        let options = BasicCardCsvImportOptions {
            deck_id: "deck".to_string(),
            id_prefix: "anki".to_string(),
            created_at: 456,
        };

        let cards = import_anki_basic_tsv(tsv, &options).unwrap();

        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].id, "anki-1");
        assert_eq!(cards[0].front, "letter-a");
        assert_eq!(cards[1].front, "hello\t\"friend\"");
        assert_eq!(cards[1].back, "line one\nline two");
        assert_eq!(cards[1].created_at, 456);
    }

    #[test]
    fn anki_basic_tsv_import_honors_html_header() {
        let options = BasicCardCsvImportOptions {
            deck_id: "deck".to_string(),
            id_prefix: "anki".to_string(),
            created_at: 456,
        };

        let plain = import_anki_basic_tsv(
            "#separator:tab\n#html:false\n#columns:Front\tBack\n<b>hola</b>\t\"mother & aunt\"\n",
            &options,
        )
        .unwrap();
        assert_eq!(plain[0].front, "&lt;b&gt;hola&lt;/b&gt;");
        assert_eq!(plain[0].back, "mother &amp; aunt");

        let html = import_anki_basic_tsv(
            "#separator:tab\n#html:true\n#columns:Front\tBack\n<b>hola</b>\t\"mother & aunt\"\n",
            &options,
        )
        .unwrap();
        assert_eq!(html[0].front, "<b>hola</b>");
        assert_eq!(html[0].back, "mother & aunt");
    }

    #[test]
    fn anki_basic_text_import_honors_separator_header() {
        let text = "#separator:comma\n#html:false\n#notetype:Basic\n#columns:Front,Back\n\"hello, friend\",hola\namma,mother\n";
        let options = BasicCardCsvImportOptions {
            deck_id: "deck".to_string(),
            id_prefix: "anki".to_string(),
            created_at: 456,
        };

        let cards = import_anki_basic_tsv(text, &options).unwrap();

        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].front, "hello, friend");
        assert_eq!(cards[0].back, "hola");
        assert_eq!(cards[1].front, "amma");
        assert_eq!(cards[1].back, "mother");

        let pipe_text =
            "#separator:pipe\n#html:false\n#notetype:Basic\n#columns:Front|Back\nhej|hello\n";
        let pipe_cards = import_anki_basic_tsv(pipe_text, &options).unwrap();
        assert_eq!(pipe_cards[0].front, "hej");
        assert_eq!(pipe_cards[0].back, "hello");
    }

    #[test]
    fn anki_basic_tsv_import_reports_short_rows() {
        let options = BasicCardCsvImportOptions {
            deck_id: "deck".to_string(),
            id_prefix: "anki".to_string(),
            created_at: 456,
        };

        let error = import_anki_basic_tsv("#separator:tab\nfront-only\n", &options).unwrap_err();

        assert!(error.message.contains("at least 2 fields"));
        assert_eq!(error.row, Some(2));
    }

    #[test]
    fn anki_note_tsv_import_creates_basic_notes_and_lineage_cards() {
        let tsv = "#separator:tab\n#notetype:Basic\n#deck:Tamil::Script\n#columns:Front\tBack\tTags\nletter-a\ta\ttamil script\n\"hello\t\"\"friend\"\"\"\t\"line one\nline two\"\tspanish\n";
        let options = AnkiNoteTsvImportOptions {
            deck_id: "deck".to_string(),
            note_type_id: String::new(),
            note_type_name: String::new(),
            note_id_prefix: "anki-note".to_string(),
            created_at: 456,
        };

        let imported = import_anki_notes_tsv(tsv, &options).unwrap();

        assert_eq!(imported.note_types.len(), 1);
        assert_eq!(imported.note_types[0].id, "basic");
        assert_eq!(imported.note_types[0].name, "Basic");
        assert_eq!(imported.notes.len(), 2);
        assert_eq!(imported.notes[0].id, "anki-note-1");
        assert_eq!(imported.notes[0].fields[0].value, "letter-a");
        assert_eq!(imported.notes[0].fields[1].value, "a");
        assert_eq!(imported.notes[0].tags, vec!["tamil", "script"]);
        assert_eq!(imported.notes[1].fields[0].value, "hello\t\"friend\"");
        assert_eq!(imported.notes[1].fields[1].value, "line one\nline two");
        assert_eq!(imported.cards.len(), 2);
        assert_eq!(imported.cards[0].id, "anki-note-1::forward");
        assert_eq!(imported.cards[0].front, "letter-a");
        let lineage = imported.cards[0].lineage.as_ref().unwrap();
        assert_eq!(lineage.note_id, "anki-note-1");
        assert_eq!(lineage.note_type_id, "basic");
        assert_eq!(lineage.template_id, "forward");
    }

    #[test]
    fn anki_note_text_import_honors_html_header() {
        let options = AnkiNoteTsvImportOptions {
            deck_id: "deck".to_string(),
            note_type_id: "basic".to_string(),
            note_type_name: "Basic".to_string(),
            note_id_prefix: "note".to_string(),
            created_at: 456,
        };

        let plain = import_anki_notes_tsv(
            "#separator:tab\n#html:false\n#notetype:Basic\n#columns:Front\tBack\n<b>hola</b>\t\"mother & aunt\"\n",
            &options,
        )
        .unwrap();
        assert_eq!(plain.notes[0].fields[0].value, "&lt;b&gt;hola&lt;/b&gt;");
        assert_eq!(plain.notes[0].fields[1].value, "mother &amp; aunt");
        assert_eq!(plain.cards[0].front, "&lt;b&gt;hola&lt;/b&gt;");
        assert_eq!(plain.cards[0].back, "mother &amp; aunt");

        let html = import_anki_notes_tsv(
            "#separator:tab\n#html:true\n#notetype:Basic\n#columns:Front\tBack\n<b>hola</b>\t\"mother & aunt\"\n",
            &options,
        )
        .unwrap();
        assert_eq!(html.notes[0].fields[0].value, "<b>hola</b>");
        assert_eq!(html.notes[0].fields[1].value, "mother & aunt");
        assert_eq!(html.cards[0].front, "<b>hola</b>");
        assert_eq!(html.cards[0].back, "mother & aunt");
    }

    #[test]
    fn anki_note_tsv_import_creates_reversed_sibling_cards() {
        let tsv = "#separator:tab\n#notetype:Basic (and reversed card)\n#columns:Front\tBack\tTags\nhola\thello\tspanish\n";
        let options = AnkiNoteTsvImportOptions {
            deck_id: "deck".to_string(),
            note_type_id: "basic-reversed".to_string(),
            note_type_name: String::new(),
            note_id_prefix: "note".to_string(),
            created_at: 456,
        };

        let imported = import_anki_notes_tsv(tsv, &options).unwrap();

        assert_eq!(imported.note_types[0].templates.len(), 2);
        assert_eq!(imported.cards.len(), 2);
        assert_eq!(imported.cards[0].id, "note-1::forward");
        assert_eq!(imported.cards[0].front, "hola");
        assert_eq!(imported.cards[1].id, "note-1::reverse");
        assert_eq!(imported.cards[1].front, "hello");
        assert_eq!(
            imported.cards[1].lineage.as_ref().unwrap().note_id,
            "note-1"
        );
    }

    #[test]
    fn anki_note_tsv_import_honors_optional_reversed_cards() {
        let tsv = "#separator:tab\n#notetype:Basic (optional reversed card)\n#columns:Front\tBack\tAdd Reverse\tTags\nhola\thello\ty\tspanish\namma\tmother\t\ttamil\n";
        let options = AnkiNoteTsvImportOptions {
            deck_id: "deck".to_string(),
            note_type_id: String::new(),
            note_type_name: String::new(),
            note_id_prefix: "note".to_string(),
            created_at: 456,
        };

        let imported = import_anki_notes_tsv(tsv, &options).unwrap();

        assert_eq!(imported.note_types[0].fields[2].id, "add-reverse");
        assert_eq!(imported.note_types[0].fields[2].name, "Add Reverse");
        assert_eq!(
            imported.note_types[0].templates[1].required_field_names,
            vec!["Front", "Back", "Add Reverse"]
        );
        assert_eq!(imported.notes[0].fields[2].value, "y");
        assert_eq!(imported.notes[1].fields[2].value, "");
        assert_eq!(imported.cards.len(), 3);
        assert_eq!(imported.cards[0].id, "note-1::forward");
        assert_eq!(imported.cards[1].id, "note-1::reverse");
        assert_eq!(imported.cards[2].id, "note-2::forward");
    }

    #[test]
    fn anki_note_tsv_import_creates_basic_type_answer_cards() {
        let tsv = "#separator:tab\n#notetype:Basic (type in the answer)\n#columns:Front\tBack\tTags\nhola\thello\tspanish\n";
        let options = AnkiNoteTsvImportOptions {
            deck_id: "deck".to_string(),
            note_type_id: String::new(),
            note_type_name: String::new(),
            note_id_prefix: "note".to_string(),
            created_at: 456,
        };

        let imported = import_anki_notes_tsv(tsv, &options).unwrap();

        let template = &imported.note_types[0].templates[0];
        assert_eq!(imported.note_types[0].id, "basic-type-in-the-answer");
        assert_eq!(template.front_template, "{{Front}}{{type:Back}}");
        assert_eq!(
            template.back_template,
            "{{FrontSide}}<hr id=answer>{{Back}}"
        );
        assert_eq!(imported.cards.len(), 1);
        assert_eq!(imported.cards[0].front, "hola[type answer: Back]");
        assert_eq!(
            imported.cards[0].back,
            "hola[type answer: Back]<hr id=answer>hello"
        );
    }

    #[test]
    fn anki_note_text_import_honors_separator_header() {
        let text = "#separator:semicolon\n#notetype:Basic\n#tags:imported spanish\n#columns:Front;Back;Tags\n\"hola;salve\";hello;spanish latin\n";
        let options = AnkiNoteTsvImportOptions {
            deck_id: "deck".to_string(),
            note_type_id: String::new(),
            note_type_name: String::new(),
            note_id_prefix: "note".to_string(),
            created_at: 456,
        };

        let imported = import_anki_notes_tsv(text, &options).unwrap();

        assert_eq!(imported.notes.len(), 1);
        assert_eq!(imported.notes[0].fields[0].value, "hola;salve");
        assert_eq!(imported.notes[0].fields[1].value, "hello");
        assert_eq!(imported.notes[0].tags, vec!["imported", "spanish", "latin"]);
        assert_eq!(imported.cards[0].front, "hola;salve");

        let colon_text = "#separator:colon\n#notetype:Basic\n#columns:Front:Back\namma:mother\n";
        let colon_imported = import_anki_notes_tsv(colon_text, &options).unwrap();
        assert_eq!(colon_imported.notes[0].fields[0].value, "amma");
        assert_eq!(colon_imported.notes[0].fields[1].value, "mother");
    }

    #[test]
    fn anki_note_text_import_honors_tags_column_header() {
        let text = "#separator:tab\n#notetype:Basic\n#tags:global\n#tags column:3\n#columns:Front\tBack\tLabels\nhola\thello\tspanish common\n";
        let options = AnkiNoteTsvImportOptions {
            deck_id: "deck".to_string(),
            note_type_id: String::new(),
            note_type_name: String::new(),
            note_id_prefix: "note".to_string(),
            created_at: 456,
        };

        let imported = import_anki_notes_tsv(text, &options).unwrap();

        assert_eq!(imported.notes[0].tags, vec!["global", "spanish", "common"]);
        assert_eq!(
            imported.notes[0]
                .fields
                .iter()
                .map(|field| field.field_id.as_str())
                .collect::<Vec<_>>(),
            vec!["front", "back"]
        );

        let custom_text = "#separator:tab\n#notetype:Vocabulary Story\n#tags column:4\n#columns:Word\tRoot\tCognate\tLabels\nhablar\tfabl-\tfable\tspanish latin\n";
        let custom_imported = import_anki_notes_tsv(custom_text, &options).unwrap();
        assert_eq!(custom_imported.notes[0].tags, vec!["spanish", "latin"]);
        assert_eq!(
            custom_imported.note_types[0]
                .fields
                .iter()
                .map(|field| field.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Word", "Root", "Cognate"]
        );
    }

    #[test]
    fn anki_note_text_import_preserves_guid_column_sources() {
        let text = "#separator:tab\n#notetype:Basic\n#guid column:3\n#columns:Front\tBack\tStableGuid\nhola\thello\tguid-123\n";
        let options = AnkiNoteTsvImportOptions {
            deck_id: "deck".to_string(),
            note_type_id: String::new(),
            note_type_name: String::new(),
            note_id_prefix: "note".to_string(),
            created_at: 456,
        };

        let imported = import_anki_notes_tsv(text, &options).unwrap();

        assert_eq!(imported.notes[0].id, "note-1");
        assert_eq!(imported.external_sources.len(), 1);
        assert_eq!(
            imported.external_sources[0].target,
            ExternalSourceTarget::Note
        );
        assert_eq!(imported.external_sources[0].target_id, "note-1");
        assert_eq!(imported.external_sources[0].source, "anki-text");
        assert_eq!(
            imported.external_sources[0].original_id.as_deref(),
            Some("guid-123")
        );
        assert_eq!(
            imported.external_sources[0]
                .data
                .get("guid")
                .map(String::as_str),
            Some("guid-123")
        );

        let custom_text = "#separator:tab\n#notetype:Vocabulary Story\n#guid column:3\n#columns:Word\tRoot\tStableGuid\nhablar\tfabl-\tguid-456\n";
        let custom_imported = import_anki_notes_tsv(custom_text, &options).unwrap();
        assert_eq!(
            custom_imported.note_types[0]
                .fields
                .iter()
                .map(|field| field.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Word", "Root"]
        );
        assert_eq!(
            custom_imported.external_sources[0].original_id.as_deref(),
            Some("guid-456")
        );
    }

    #[test]
    fn anki_note_text_import_honors_deck_headers() {
        let options = AnkiNoteTsvImportOptions {
            deck_id: "fallback-deck".to_string(),
            note_type_id: String::new(),
            note_type_name: String::new(),
            note_id_prefix: "note".to_string(),
            created_at: 456,
        };
        let text = "#separator:tab\n#deck:Tamil::Script\n#notetype:Basic\n#columns:Front\tBack\nletter-a\ta\n";

        let imported = import_anki_notes_tsv(text, &options).unwrap();

        assert_eq!(imported.notes[0].deck_id, "Tamil::Script");
        assert_eq!(imported.cards[0].deck_id, "Tamil::Script");

        let column_text = "#separator:tab\n#deck:Tamil::Script\n#deck column:3\n#notetype:Basic\n#columns:Front\tBack\tDeck\nhola\thello\tSpanish::Common\n";
        let column_imported = import_anki_notes_tsv(column_text, &options).unwrap();
        assert_eq!(column_imported.notes[0].deck_id, "Spanish::Common");
        assert_eq!(column_imported.cards[0].deck_id, "Spanish::Common");

        let custom_text = "#separator:tab\n#notetype:Vocabulary Story\n#deck column:3\n#columns:Word\tRoot\tDeck\nhablar\tfabl-\tSpanish::Common\n";
        let custom_imported = import_anki_notes_tsv(custom_text, &options).unwrap();
        assert_eq!(custom_imported.notes[0].deck_id, "Spanish::Common");
        assert_eq!(
            custom_imported.note_types[0]
                .fields
                .iter()
                .map(|field| field.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Word", "Root"]
        );
    }

    #[test]
    fn anki_note_text_import_honors_note_type_column_header() {
        let options = AnkiNoteTsvImportOptions {
            deck_id: "deck".to_string(),
            note_type_id: String::new(),
            note_type_name: String::new(),
            note_id_prefix: "note".to_string(),
            created_at: 456,
        };
        let text = "#separator:tab\n#notetype column:3\n#columns:Prompt\tAnswer\tModel\nhola\thello\tBasic (type in the answer)\namma\tmother\tBasic (and reversed card)\n";

        let imported = import_anki_notes_tsv(text, &options).unwrap();

        assert_eq!(imported.note_types.len(), 2);
        assert_eq!(imported.note_types[0].id, "basic-type-in-the-answer");
        assert_eq!(
            imported.note_types[0].templates[0].front_template,
            "{{Front}}{{type:Back}}"
        );
        assert_eq!(imported.note_types[1].id, "basic-and-reversed-card");
        assert_eq!(imported.note_types[1].templates.len(), 2);
        assert_eq!(imported.notes[0].note_type_id, "basic-type-in-the-answer");
        assert_eq!(imported.notes[1].note_type_id, "basic-and-reversed-card");
        assert_eq!(imported.notes[0].fields[0].field_id, "front");
        assert_eq!(imported.notes[0].fields[0].value, "hola");
        assert_eq!(imported.notes[0].fields[1].field_id, "back");
        assert_eq!(imported.notes[0].fields[1].value, "hello");
        assert_eq!(imported.cards.len(), 3);
        assert_eq!(imported.cards[0].id, "note-1::forward");
        assert_eq!(imported.cards[0].front, "hola[type answer: Back]");
        assert_eq!(imported.cards[1].id, "note-2::forward");
        assert_eq!(imported.cards[2].id, "note-2::reverse");

        let custom_text = "#separator:tab\n#notetype column:3\n#columns:Word\tRoot\tModel\nhablar\tfabl-\tVocabulary Story\n";
        let custom_imported = import_anki_notes_tsv(custom_text, &options).unwrap();
        assert_eq!(custom_imported.note_types[0].id, "vocabulary-story");
        assert_eq!(
            custom_imported.note_types[0]
                .fields
                .iter()
                .map(|field| field.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Word", "Root"]
        );
        assert!(custom_imported.cards.is_empty());
    }

    #[test]
    fn anki_note_tsv_import_creates_cloze_notes_and_cards() {
        let tsv = "#separator:tab\n#notetype:Cloze\n#columns:Text\tExtra\tTags\n\"A {{c1::root::base}} plus {{c2::suffix}}\"\tetymology\tgrammar spanish\n";
        let options = AnkiNoteTsvImportOptions {
            deck_id: "deck".to_string(),
            note_type_id: String::new(),
            note_type_name: String::new(),
            note_id_prefix: "cloze-note".to_string(),
            created_at: 456,
        };

        let imported = import_anki_notes_tsv(tsv, &options).unwrap();

        assert_eq!(imported.note_types[0].id, "cloze");
        assert_eq!(imported.note_types[0].name, "Cloze");
        assert_eq!(imported.note_types[0].templates[0].id, "cloze");
        assert_eq!(imported.notes[0].fields[0].field_id, "text");
        assert_eq!(imported.notes[0].fields[1].field_id, "extra");
        assert_eq!(imported.notes[0].tags, vec!["grammar", "spanish"]);
        assert_eq!(imported.cards.len(), 2);
        assert_eq!(imported.cards[0].id, "cloze-note-1::cloze::c1");
        assert_eq!(imported.cards[0].front, "A [base] plus suffix");
        assert_eq!(
            imported.cards[0].lineage.as_ref().unwrap().cloze_ordinal,
            Some(1)
        );
        assert_eq!(imported.cards[1].id, "cloze-note-1::cloze::c2");
        assert_eq!(imported.cards[1].front, "A root plus [...]");
    }

    #[test]
    fn anki_note_tsv_import_preserves_custom_note_type_columns() {
        let tsv = "#separator:tab\n#notetype:Basic Grammar Story\n#columns:Infinitive\tRoot\tCognate\tTags\nhablar\tfabl-\tfable\tspanish latin\n";
        let options = AnkiNoteTsvImportOptions {
            deck_id: "deck".to_string(),
            note_type_id: String::new(),
            note_type_name: String::new(),
            note_id_prefix: "custom-note".to_string(),
            created_at: 456,
        };

        let imported = import_anki_notes_tsv(tsv, &options).unwrap();

        assert_eq!(imported.note_types[0].id, "basic-grammar-story");
        assert_eq!(imported.note_types[0].name, "Basic Grammar Story");
        assert_eq!(imported.note_types[0].templates, Vec::new());
        assert_eq!(
            imported.note_types[0]
                .fields
                .iter()
                .map(|field| (field.id.as_str(), field.name.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("infinitive", "Infinitive"),
                ("root", "Root"),
                ("cognate", "Cognate"),
            ]
        );
        assert_eq!(imported.notes[0].tags, vec!["spanish", "latin"]);
        assert_eq!(imported.notes[0].fields[0].value, "hablar");
        assert_eq!(imported.notes[0].fields[1].value, "fabl-");
        assert_eq!(imported.notes[0].fields[2].value, "fable");
        assert!(imported.cards.is_empty());
    }

    #[test]
    fn anki_note_tsv_import_infers_custom_columns_without_headers() {
        let options = AnkiNoteTsvImportOptions {
            deck_id: "deck".to_string(),
            note_type_id: String::new(),
            note_type_name: "Tamil Lexeme".to_string(),
            note_id_prefix: "custom-note".to_string(),
            created_at: 456,
        };

        let imported = import_anki_notes_tsv("amma\tmother\n", &options).unwrap();

        assert_eq!(imported.note_types[0].id, "tamil-lexeme");
        assert_eq!(
            imported.note_types[0]
                .fields
                .iter()
                .map(|field| (field.id.as_str(), field.name.as_str()))
                .collect::<Vec<_>>(),
            vec![("field-1", "Field 1"), ("field-2", "Field 2")]
        );
        assert_eq!(imported.notes[0].fields[0].value, "amma");
        assert_eq!(imported.notes[0].fields[1].value, "mother");
        assert!(imported.cards.is_empty());
    }

    #[test]
    fn anki_note_tsv_export_writes_fields_tags_and_headers() {
        let note_type = basic_note_type("basic", "Basic", 456, false, false, false);
        let notes = vec![
            Note {
                id: "note-1".to_string(),
                note_type_id: "basic".to_string(),
                deck_id: "deck".to_string(),
                fields: vec![
                    NoteFieldValue {
                        field_id: "front".to_string(),
                        value: "letter-a".to_string(),
                    },
                    NoteFieldValue {
                        field_id: "back".to_string(),
                        value: "a".to_string(),
                    },
                ],
                tags: vec!["tamil".to_string(), "script".to_string()],
                created_at: 456,
                updated_at: 456,
            },
            Note {
                id: "note-2".to_string(),
                note_type_id: "basic".to_string(),
                deck_id: "deck".to_string(),
                fields: vec![
                    NoteFieldValue {
                        field_id: "front".to_string(),
                        value: "hello\t\"friend\"".to_string(),
                    },
                    NoteFieldValue {
                        field_id: "back".to_string(),
                        value: "line one\nline two".to_string(),
                    },
                ],
                tags: vec!["spanish".to_string()],
                created_at: 456,
                updated_at: 456,
            },
        ];
        let options = AnkiBasicTsvExportOptions {
            deck_name: "Tamil::Script".to_string(),
            note_type_name: String::new(),
            html: false,
            include_headers: true,
        };

        let tsv = export_notes_anki_tsv(&note_type, &notes, &options);

        assert!(tsv.starts_with(
            "#separator:tab\n#html:false\n#notetype:Basic\n#deck:Tamil::Script\n#columns:Front\tBack\tTags\n"
        ));
        assert!(tsv.contains("letter-a\ta\ttamil script\n"));
        assert!(tsv.contains("\"hello\t\"\"friend\"\"\"\t\"line one\nline two\"\tspanish\n"));
    }

    #[test]
    fn anki_note_tsv_export_can_round_trip_guid_and_deck_columns() {
        let note_type = basic_note_type("basic", "Basic", 456, false, false, false);
        let notes = vec![
            Note {
                id: "note-1".to_string(),
                note_type_id: "basic".to_string(),
                deck_id: "child".to_string(),
                fields: vec![
                    NoteFieldValue {
                        field_id: "front".to_string(),
                        value: "letter-a".to_string(),
                    },
                    NoteFieldValue {
                        field_id: "back".to_string(),
                        value: "a".to_string(),
                    },
                ],
                tags: vec!["tamil".to_string(), "script".to_string()],
                created_at: 456,
                updated_at: 456,
            },
            Note {
                id: "note-2".to_string(),
                note_type_id: "basic".to_string(),
                deck_id: "deck".to_string(),
                fields: vec![
                    NoteFieldValue {
                        field_id: "front".to_string(),
                        value: "amma".to_string(),
                    },
                    NoteFieldValue {
                        field_id: "back".to_string(),
                        value: "mother".to_string(),
                    },
                ],
                tags: vec!["tamil".to_string(), "family".to_string()],
                created_at: 456,
                updated_at: 456,
            },
        ];
        let decks = vec![
            Deck {
                id: "deck".to_string(),
                name: "Tamil::Script".to_string(),
                description: String::new(),
                created_at: 456,
            },
            Deck {
                id: "child".to_string(),
                name: "Tamil::Verbs".to_string(),
                description: String::new(),
                created_at: 456,
            },
        ];
        let external_sources = vec![
            ExternalSourceRecord {
                target: ExternalSourceTarget::Note,
                target_id: "note-1".to_string(),
                source: "anki-v11".to_string(),
                original_id: Some("1000".to_string()),
                data: BTreeMap::from([("guid".to_string(), "guid-1".to_string())]),
            },
            ExternalSourceRecord {
                target: ExternalSourceTarget::Note,
                target_id: "note-2".to_string(),
                source: "anki-text".to_string(),
                original_id: Some("guid-2".to_string()),
                data: BTreeMap::new(),
            },
        ];
        let options = AnkiBasicTsvExportOptions {
            deck_name: "Tamil::Script".to_string(),
            note_type_name: String::new(),
            html: false,
            include_headers: true,
        };

        let tsv = export_notes_anki_tsv_with_context(
            &note_type,
            &notes,
            &decks,
            &external_sources,
            &options,
        );

        assert!(tsv.starts_with(
            "#separator:tab\n#html:false\n#notetype:Basic\n#deck:Tamil::Script\n#guid column:3\n#deck column:4\n#columns:Front\tBack\tGuid\tDeck\tTags\n"
        ));
        assert!(tsv.contains("letter-a\ta\tguid-1\tTamil::Verbs\ttamil script\n"));
        assert!(tsv.contains("amma\tmother\tguid-2\tTamil::Script\ttamil family\n"));

        let imported = import_anki_notes_tsv(
            &tsv,
            &AnkiNoteTsvImportOptions {
                deck_id: "fallback".to_string(),
                note_type_id: "basic".to_string(),
                note_type_name: String::new(),
                note_id_prefix: "round".to_string(),
                created_at: 789,
            },
        )
        .unwrap();

        assert_eq!(imported.notes[0].deck_id, "Tamil::Verbs");
        assert_eq!(imported.notes[1].deck_id, "Tamil::Script");
        assert_eq!(
            imported
                .external_sources
                .iter()
                .map(|source| source.original_id.as_deref())
                .collect::<Vec<_>>(),
            vec![Some("guid-1"), Some("guid-2")]
        );
    }

    #[test]
    fn anki_note_tsv_import_reports_missing_columns_and_short_rows() {
        let options = AnkiNoteTsvImportOptions {
            deck_id: "deck".to_string(),
            note_type_id: "basic".to_string(),
            note_type_name: "Basic".to_string(),
            note_id_prefix: "note".to_string(),
            created_at: 456,
        };

        let error =
            import_anki_notes_tsv("#separator:tab\n#columns:Front\tTags\nx\ttag\n", &options)
                .unwrap_err();
        assert_eq!(error.row, Some(1));
        assert!(error.message.contains("Front and Back"));

        let error = import_anki_notes_tsv(
            "#separator:tab\n#columns:Front\tBack\tTags\nfront\tback\n",
            &options,
        )
        .unwrap_err();
        assert_eq!(error.row, Some(3));
        assert!(error.message.contains("at least 3 fields"));

        let cloze_error = import_anki_notes_tsv(
            "#separator:tab\n#notetype:Cloze\n#columns:Extra\tTags\nhint\tgrammar\n",
            &options,
        )
        .unwrap_err();
        assert_eq!(cloze_error.row, Some(1));
        assert!(cloze_error.message.contains("must include Text"));

        let custom_error = import_anki_notes_tsv(
            "#separator:tab\n#notetype:Only Tags\n#columns:Tags\nspanish\n",
            &options,
        )
        .unwrap_err();
        assert_eq!(custom_error.row, Some(1));
        assert!(custom_error.message.contains("at least one note field"));

        let tags_column_error = import_anki_notes_tsv(
            "#separator:tab\n#tags column:0\n#columns:Front\tBack\tLabels\nfront\tback\ttag\n",
            &options,
        )
        .unwrap_err();
        assert_eq!(tags_column_error.row, Some(2));
        assert!(tags_column_error.message.contains("positive column number"));

        let guid_column_error = import_anki_notes_tsv(
            "#separator:tab\n#guid column:not-a-number\n#columns:Front\tBack\tGuid\nfront\tback\tguid\n",
            &options,
        )
        .unwrap_err();
        assert_eq!(guid_column_error.row, Some(2));
        assert!(guid_column_error.message.contains("positive column number"));

        let deck_column_error = import_anki_notes_tsv(
            "#separator:tab\n#deck column:not-a-number\n#columns:Front\tBack\tDeck\nfront\tback\tdeck\n",
            &options,
        )
        .unwrap_err();
        assert_eq!(deck_column_error.row, Some(2));
        assert!(deck_column_error.message.contains("positive column number"));

        let note_type_column_error = import_anki_notes_tsv(
            "#separator:tab\n#notetype column:not-a-number\n#columns:Front\tBack\tModel\nfront\tback\tBasic\n",
            &options,
        )
        .unwrap_err();
        assert_eq!(note_type_column_error.row, Some(2));
        assert!(note_type_column_error
            .message
            .contains("positive column number"));
    }

    #[test]
    fn card_csv_import_accepts_crlf_line_endings() {
        let csv = "id,deckId,front,back,createdAt\r\ncard,deck,front,back,123\r\n";

        let restored = import_cards_csv(csv).unwrap();

        assert_eq!(restored, vec![card("card", "front", "back")]);
    }

    #[test]
    fn card_csv_import_skips_blank_trailing_rows() {
        let csv = "id,deckId,front,back,createdAt\ncard,deck,front,back,123\n,,,,\n";

        let restored = import_cards_csv(csv).unwrap();

        assert_eq!(restored, vec![card("card", "front", "back")]);
    }

    #[test]
    fn basic_card_csv_import_generates_deterministic_cards() {
        let csv = "front,back\nletter-a,a\n\"hello, friend\",\"line one\nline two\"\n,\n";
        let options = BasicCardCsvImportOptions {
            deck_id: "deck".to_string(),
            id_prefix: "import".to_string(),
            created_at: 456,
        };

        let cards = import_basic_cards_csv(csv, &options).unwrap();

        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].id, "import-1");
        assert_eq!(cards[0].deck_id, "deck");
        assert_eq!(cards[0].front, "letter-a");
        assert_eq!(cards[0].created_at, 456);
        assert_eq!(cards[1].id, "import-2");
        assert_eq!(cards[1].back, "line one\nline two");
    }

    #[test]
    fn card_csv_import_reports_shape_errors() {
        let error = import_cards_csv("front,back\nx,y\n").unwrap_err();
        assert_eq!(error.row, Some(1));
        assert!(error.message.contains("header"));

        let error =
            import_cards_csv("id,deckId,front,back,createdAt\ncard,deck,front\n").unwrap_err();
        assert_eq!(error.row, Some(2));
        assert!(error.message.contains("5 fields"));

        let error = import_cards_csv("id,deckId,front,back,createdAt\ncard,deck,front,back,now\n")
            .unwrap_err();
        assert_eq!(error.row, Some(2));
        assert!(error.message.contains("createdAt"));
    }

    #[test]
    fn basic_card_csv_import_reports_shape_errors() {
        let options = BasicCardCsvImportOptions {
            deck_id: "deck".to_string(),
            id_prefix: "import".to_string(),
            created_at: 456,
        };

        let error = import_basic_cards_csv("id,front,back\n1,x,y\n", &options).unwrap_err();
        assert_eq!(error.row, Some(1));
        assert!(error.message.contains("front,back"));

        let error = import_basic_cards_csv("front,back\nfront\n", &options).unwrap_err();
        assert_eq!(error.row, Some(2));
        assert!(error.message.contains("2 fields"));
    }
}
