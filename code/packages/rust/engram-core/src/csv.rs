use crate::model::{Card, CardTemplate, FieldDef, Note, NoteFieldValue, NoteType};
use crate::template::{generate_cards_for_note, materialize_generated_card};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

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
    let mut output = String::new();
    let note_type_name = if options.note_type_name.trim().is_empty() {
        note_type.name.as_str()
    } else {
        options.note_type_name.as_str()
    };

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
        output.push_str("#columns:");
        for (index, field) in note_type.fields.iter().enumerate() {
            if index > 0 {
                output.push('\t');
            }
            write_tsv_field(&mut output, &field.name);
        }
        output.push_str("\tTags\n");
    }

    for note in notes {
        let mut field_values = Vec::new();
        for field in &note_type.fields {
            let value = note
                .fields
                .iter()
                .find(|candidate| candidate.field_id == field.id)
                .map_or("", |field| field.value.as_str());
            field_values.push(value);
        }
        let tags = note.tags.join(" ");
        field_values.push(tags.as_str());
        write_tsv_row(&mut output, &field_values);
    }

    output
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
    let records = parse_tsv_records(input)?;
    let mut cards = Vec::new();

    for (index, fields) in records.into_iter().enumerate() {
        if is_blank_record(&fields) || fields.first().is_some_and(|field| field.starts_with('#')) {
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
            front: fields[0].clone(),
            back: fields[1].clone(),
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
    let records = parse_tsv_records(input)?;
    let mut headers = AnkiTsvHeaders::default();
    let mut rows = Vec::new();

    for (index, fields) in records.into_iter().enumerate() {
        if is_blank_record(&fields) {
            continue;
        }
        if fields.first().is_some_and(|field| field.starts_with('#')) {
            headers.read_header(&fields);
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
    let note_type_id = if options.note_type_id.trim().is_empty() {
        default_note_type_id(&note_type_name)
    } else {
        options.note_type_id.clone()
    };
    let reverse = is_basic_and_reversed(&note_type_name);
    let note_type = basic_note_type(&note_type_id, &note_type_name, options.created_at, reverse);
    let columns = headers.columns.unwrap_or_else(|| {
        if rows.first().is_some_and(|(_, fields)| fields.len() >= 3) {
            vec!["Front".to_string(), "Back".to_string(), "Tags".to_string()]
        } else {
            vec!["Front".to_string(), "Back".to_string()]
        }
    });
    let front_index = column_index(&columns, "Front").ok_or_else(|| {
        csv_error(
            "Anki note TSV #columns must include Front and Back",
            Some(1),
        )
    })?;
    let back_index = column_index(&columns, "Back").ok_or_else(|| {
        csv_error(
            "Anki note TSV #columns must include Front and Back",
            Some(1),
        )
    })?;
    let tag_index = column_index(&columns, "Tags");
    let required_columns = [Some(front_index), Some(back_index), tag_index]
        .into_iter()
        .flatten()
        .max()
        .map_or(0, |index| index + 1);

    let mut notes = Vec::new();
    let mut cards = Vec::new();

    for (row, fields) in rows {
        if fields.len() < required_columns {
            return Err(csv_error(
                &format!(
                    "Anki note TSV row must have at least {required_columns} fields, found {}",
                    fields.len()
                ),
                Some(row),
            ));
        }

        let sequence = notes.len() + 1;
        let note = Note {
            id: generated_basic_card_id(&options.note_id_prefix, sequence),
            note_type_id: note_type.id.clone(),
            deck_id: options.deck_id.clone(),
            fields: vec![
                NoteFieldValue {
                    field_id: "front".to_string(),
                    value: fields[front_index].clone(),
                },
                NoteFieldValue {
                    field_id: "back".to_string(),
                    value: fields[back_index].clone(),
                },
            ],
            tags: tag_index
                .and_then(|index| fields.get(index))
                .map_or_else(Vec::new, |tags| split_anki_tags(tags)),
            created_at: options.created_at,
            updated_at: options.created_at,
        };

        cards.extend(
            generate_cards_for_note(&note_type, &note)
                .iter()
                .map(|generated| materialize_generated_card(generated, options.created_at)),
        );
        notes.push(note);
    }

    Ok(AnkiNoteTsvImport {
        note_types: vec![note_type],
        notes,
        cards,
    })
}

#[derive(Default)]
struct AnkiTsvHeaders {
    note_type_name: Option<String>,
    columns: Option<Vec<String>>,
}

impl AnkiTsvHeaders {
    fn read_header(&mut self, fields: &[String]) {
        let Some(first) = fields.first() else {
            return;
        };

        if let Some(note_type_name) = first.strip_prefix("#notetype:") {
            self.note_type_name = Some(note_type_name.trim().to_string());
            return;
        }

        if let Some(first_column) = first.strip_prefix("#columns:") {
            let mut columns = Vec::with_capacity(fields.len());
            columns.push(first_column.trim().to_string());
            columns.extend(fields.iter().skip(1).map(|field| field.trim().to_string()));
            self.columns = Some(columns);
        }
    }
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

fn header_value(value: &str) -> String {
    value.replace(['\r', '\n'], " ")
}

fn parse_csv_records(input: &str) -> Result<Vec<Vec<String>>, CsvError> {
    parse_delimited_records(input, ',', "CSV")
}

fn parse_tsv_records(input: &str) -> Result<Vec<Vec<String>>, CsvError> {
    parse_delimited_records(input, '\t', "TSV")
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

fn basic_note_type(id: &str, name: &str, created_at: u64, reverse: bool) -> NoteType {
    let mut templates = vec![CardTemplate {
        id: "forward".to_string(),
        name: "Forward".to_string(),
        front_template: "{{Front}}".to_string(),
        back_template: "{{Back}}".to_string(),
        required_field_names: vec!["Front".to_string(), "Back".to_string()],
        ordinal: 0,
    }];

    if reverse {
        templates.push(CardTemplate {
            id: "reverse".to_string(),
            name: "Reverse".to_string(),
            front_template: "{{Back}}".to_string(),
            back_template: "{{Front}}".to_string(),
            required_field_names: vec!["Front".to_string(), "Back".to_string()],
            ordinal: 1,
        });
    }

    NoteType {
        id: id.to_string(),
        name: name.to_string(),
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
        templates,
        created_at,
        updated_at: created_at,
    }
}

fn default_note_type_id(note_type_name: &str) -> String {
    let mut id = String::new();
    let mut last_was_dash = false;

    for ch in note_type_name.chars().flat_map(char::to_lowercase) {
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

    if id.is_empty() {
        "anki-basic".to_string()
    } else {
        id
    }
}

fn is_basic_and_reversed(note_type_name: &str) -> bool {
    let normalized = note_type_name.to_ascii_lowercase();
    normalized.contains("reversed") || normalized.contains("reverse")
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
    fn anki_note_tsv_export_writes_fields_tags_and_headers() {
        let note_type = basic_note_type("basic", "Basic", 456, false);
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
