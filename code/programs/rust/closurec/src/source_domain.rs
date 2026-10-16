//! Compiler-private, read-only admission of the physical source schema.
//! Generic graph checks prove edges and present chronology; shape checks add
//! closed fields and exact inventory ownership. Neither binds trusted source
//! bytes or replays grammar. D3b must supply that stronger certification.
use coding_adventures_correlation_vector::{CVEntry, CVLog, Origin};
use coding_adventures_javascript_tokens::EsVersion;
use serde_json::{Map, Value};
use std::mem::size_of;
const NS: &str = "closurec_source_v1";
const INPUT: &[&str] = &[
    "schema",
    "role",
    "buffer_domain",
    "encoding",
    "grammar",
    "edition",
    "sha256",
    "byte_length",
];
const PIECE: &[&str] = &[
    "schema",
    "role",
    "input_id",
    "piece_index",
    "kind",
    "start",
    "end",
    "token_index",
    "token_type",
];
const POSITION: &[&str] = &[
    "byte_offset",
    "scalar_offset",
    "utf16_offset",
    "line",
    "scalar_column",
    "utf16_column",
];
const INVENTORY: &[&str] = &[
    "schema",
    "role",
    "status",
    "piece_count",
    "token_count",
    "piece_ids",
    "reason",
];
// Closed wire alphabet; compare the legacy lowercase spelling without cloning.
const TOKEN_NAMES: &[(&str, &str)] = &[
    ("Name", "name"),
    ("Number", "number"),
    ("String", "string"),
    ("Keyword", "keyword"),
    ("Plus", "plus"),
    ("Minus", "minus"),
    ("Star", "star"),
    ("Slash", "slash"),
    ("Equals", "equals"),
    ("EqualsEquals", "equalsequals"),
    ("LParen", "lparen"),
    ("RParen", "rparen"),
    ("Comma", "comma"),
    ("Colon", "colon"),
    ("Semicolon", "semicolon"),
    ("LBrace", "lbrace"),
    ("RBrace", "rbrace"),
    ("LBracket", "lbracket"),
    ("RBracket", "rbracket"),
    ("Dot", "dot"),
    ("Bang", "bang"),
    ("Newline", "newline"),
    ("Indent", "indent"),
    ("Dedent", "dedent"),
    ("EOF", "eof"),
];
#[derive(Clone, Copy)]
pub(super) struct ShapeLimits {
    work: usize,
    index_bytes: usize,
}
impl Default for ShapeLimits {
    fn default() -> Self {
        Self {
            work: 64_000_000,
            index_bytes: 64 * 1024 * 1024,
        }
    }
}
struct Work {
    used: usize,
    limit: usize,
}
impl Work {
    fn take(&mut self, amount: usize) -> Result<(), String> {
        let next = self
            .used
            .checked_add(amount)
            .ok_or("source shape work overflow")?;
        if next > self.limit {
            return Err("source shape work limit".into());
        }
        self.used = next;
        Ok(())
    }
    fn text(&mut self, text: &str) -> Result<(), String> {
        self.take(text.len())
    }
}
/// Returns no owned evidence or source-certified capability.
pub(super) fn validate_shape(log: &CVLog, limits: ShapeLimits) -> Result<(), String> {
    validate_shape_core(log, limits).map(|_| ())
}
fn index_storage(count: usize) -> Result<usize, String> {
    count
        .checked_mul(size_of::<Option<&Value>>())
        .ok_or_else(|| "source shape index overflow".into())
}
fn compact_index(id: &str, count: usize) -> Result<usize, String> {
    if id.len() != 20
        || !id.starts_with("cv1.")
        || !id.as_bytes()[4..]
            .iter()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b))
    {
        return Err("source shape compact identity required".into());
    }
    let sequence =
        u64::from_str_radix(&id[4..], 16).map_err(|_| "source shape compact sequence")?;
    let sequence = usize::try_from(sequence).map_err(|_| "source shape compact index overflow")?;
    if sequence == 0 || sequence > count {
        return Err("source shape compact index range".into());
    }
    Ok(sequence - 1)
}
fn indexed(id: &str, count: usize, work: &mut Work) -> Result<usize, String> {
    work.take(1)?;
    work.text(id)?; // canonical spelling scan
    work.text(id)?; // checked sequence conversion
    compact_index(id, count)
}
fn object<'a>(
    value: &'a Value,
    fields: &[&str],
    work: &mut Work,
) -> Result<&'a Map<String, Value>, String> {
    work.take(1)?;
    let map = value.as_object().ok_or("source shape object required")?;
    if map.len() != fields.len() {
        return Err("source shape closed field count".into());
    }
    for key in fields {
        work.take(1 + key.len())?;
        if !map.contains_key(*key) {
            return Err("source shape closed field missing".into());
        }
    }
    Ok(map)
}
fn field<'a>(map: &'a Map<String, Value>, key: &str, work: &mut Work) -> Result<&'a Value, String> {
    work.take(1 + key.len())?;
    map.get(key)
        .ok_or_else(|| "source shape missing field".into())
}
fn text<'a>(map: &'a Map<String, Value>, key: &str, work: &mut Work) -> Result<&'a str, String> {
    let value = field(map, key, work)?
        .as_str()
        .ok_or("source shape string required")?;
    work.text(value)?;
    Ok(value)
}
fn literal(
    map: &Map<String, Value>,
    key: &str,
    expected: &str,
    work: &mut Work,
) -> Result<(), String> {
    if text(map, key, work)? != expected {
        return Err("source shape literal mismatch".into());
    }
    Ok(())
}
fn decimal(value: &str, work: &mut Work) -> Result<u64, String> {
    // Charge both canonicality and conversion scans before examining digits.
    work.text(value)?;
    work.text(value)?;
    if value.is_empty()
        || value.len() > 20
        || (value.len() > 1 && value.starts_with('0'))
        || !value.bytes().all(|b| b.is_ascii_digit())
    {
        return Err("source shape canonical decimal required".into());
    }
    value
        .parse()
        .map_err(|_| "source shape decimal overflow".into())
}
fn number(map: &Map<String, Value>, key: &str, work: &mut Work) -> Result<u64, String> {
    decimal(text(map, key, work)?, work)
}
fn ordinal(map: &Map<String, Value>, key: &str, work: &mut Work) -> Result<usize, String> {
    usize::try_from(number(map, key, work)?).map_err(|_| "source shape ordinal overflow".into())
}
fn position(value: &Value, work: &mut Work) -> Result<[u64; 6], String> {
    let map = object(value, POSITION, work)?;
    let mut point = [0; 6];
    for (slot, key) in point.iter_mut().zip(POSITION) {
        *slot = number(map, key, work)?;
    }
    Ok(point)
}
fn input(origin: &Origin, work: &mut Work) -> Result<u64, String> {
    let value = origin
        .meta
        .get(NS)
        .ok_or("source shape input namespace required")?;
    let map = object(value, INPUT, work)?;
    for (key, expected) in [
        ("schema", "1"),
        ("role", "input"),
        ("buffer_domain", "read_to_string_utf8"),
        ("encoding", "utf8"),
        ("grammar", "javascript_standard"),
    ] {
        literal(map, key, expected, work)?;
    }
    let edition = text(map, "edition", work)?;
    work.take(EsVersion::ALL.len())?;
    if !EsVersion::ALL.iter().any(|v| v.as_str() == edition) {
        return Err("source shape edition".into());
    }
    let hash = text(map, "sha256", work)?;
    work.text(hash)?;
    if hash.len() != 64
        || !hash
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("source shape hash spelling".into());
    }
    let bytes = number(map, "byte_length", work)?;
    work.take(2)?;
    if origin.meta.get("byte_len").and_then(Value::as_u64) != Some(bytes)
        || origin.meta.get("content_sha256").and_then(Value::as_str) != Some(hash)
    {
        return Err("source shape legacy input disagreement".into());
    }
    Ok(bytes)
}
fn legacy_location(location: &str, path: &str, work: &mut Work) -> Result<(), String> {
    // Split from the right: Windows drive/path colons are not position fields.
    // Legacy Token positions are not inferred from independent raw coordinates.
    work.text(location)?;
    work.text(location)?;
    work.text(path)?;
    let (prefix, column) = location
        .rsplit_once(':')
        .ok_or("source shape legacy location")?;
    let (prefix, line) = prefix
        .rsplit_once(':')
        .ok_or("source shape legacy location")?;
    if prefix != path || decimal(line, work)? == 0 || decimal(column, work)? == 0 {
        return Err("source shape legacy location".into());
    }
    Ok(())
}
fn validate_shape_core(log: &CVLog, limits: ShapeLimits) -> Result<(usize, usize), String> {
    if limits.work > 512_000_000 || limits.index_bytes > 256 * 1024 * 1024 {
        return Err("source shape configuration ceiling".into());
    }
    log.validate_graph()?;
    let mut work = Work {
        used: 0,
        limit: limits.work,
    };
    let count = log.entries().len();
    // Generic validation ALSO admits hierarchical logs. Prove compact dense
    // coverage before allocation: unique canonical keys in 1..=count suffice.
    for (id, entry) in log.entries() {
        work.take(1)?;
        work.text(&entry.id)?;
        indexed(id, count, &mut work)?;
        if id != &entry.id {
            return Err("source shape compact key mismatch".into());
        }
    }
    let bytes = index_storage(count)?;
    if bytes > limits.index_bytes {
        return Err("source shape index storage limit".into());
    }
    work.take(count)?;
    let mut inventories: Vec<Option<&Value>> = Vec::new();
    inventories
        .try_reserve_exact(count)
        .map_err(|_| "source shape index allocation")?;
    if index_storage(inventories.capacity())? > bytes {
        return Err("source shape index capacity exceeds admission".into());
    }
    inventories.resize(count, None);
    // Pass A: each contribution list is visited once, while ALL legacy physical
    // origin rows enter the census, even if the namespace is missing or forged.
    let mut physical = 0usize;
    for (id, entry) in log.entries() {
        work.take(1)?;
        let mut root = false;
        if let Some(origin) = &entry.origin {
            work.take(1)?;
            work.text(&origin.source)?;
            let reserved = origin.meta.get(NS);
            match origin.source.as_str() {
                "input_file" => {
                    root = true;
                    if !entry.parent_ids.is_empty() || origin.timestamp.is_some() {
                        return Err("source shape input placement".into());
                    }
                    input(origin, &mut work)?;
                }
                "lexer_token" | "lexer_trivia" => {
                    physical = physical
                        .checked_add(1)
                        .ok_or("source shape census overflow")?;
                    let map = reserved
                        .and_then(Value::as_object)
                        .ok_or("source shape physical namespace required")?;
                    literal(map, "role", "piece", &mut work)?;
                    if origin.timestamp.is_some() {
                        return Err("source shape physical timestamp".into());
                    }
                }
                _ if reserved.is_some() => {
                    return Err("source shape reserved origin placement".into())
                }
                _ => {}
            }
        }
        let index = indexed(id, count, &mut work)?;
        for contribution in &entry.contributions {
            work.take(1)?;
            work.text(&contribution.source)?;
            work.text(&contribution.tag)?;
            let reserved = contribution.meta.get(NS);
            if contribution.source == "source.inventory" {
                if !root
                    || contribution.tag != "finalized"
                    || reserved.is_none()
                    || inventories[index].is_some()
                {
                    return Err("source shape inventory placement".into());
                }
                inventories[index] = reserved;
            } else if reserved.is_some() {
                return Err("source shape reserved contribution placement".into());
            }
        }
        if let Some(deleted) = &entry.deleted {
            work.take(1)?;
            if deleted.meta.contains_key(NS) {
                return Err("source shape reserved deletion placement".into());
            }
        }
        if root && inventories[index].is_none() {
            return Err("source shape inventory missing".into());
        }
    }
    // Pass B: fixed ordinals and sole parents prevent duplicate or cross-root
    // ownership. The independent physical/listed count then proves no omissions.
    let mut listed = 0usize;
    for (id, entry) in log.entries() {
        work.take(1)?;
        let Some(inventory) = inventories[indexed(id, count, &mut work)?] else {
            continue;
        };
        validate_inventory(log, id, entry, inventory, &mut listed, &mut work)?;
    }
    if physical != listed {
        return Err("source shape physical census mismatch".into());
    }
    Ok((work.used, bytes))
}
fn validate_inventory(
    log: &CVLog,
    root: &str,
    entry: &CVEntry,
    value: &Value,
    listed: &mut usize,
    work: &mut Work,
) -> Result<(), String> {
    work.take(1)?;
    let map = value.as_object().ok_or("source shape inventory object")?;
    let available = match text(map, "status", work)? {
        "available" => true,
        "unavailable" => false,
        _ => return Err("source shape inventory status".into()),
    };
    object(value, &INVENTORY[..if available { 6 } else { 7 }], work)?;
    literal(map, "schema", "1", work)?;
    literal(map, "role", "inventory", work)?;
    let pieces = ordinal(map, "piece_count", work)?;
    let tokens = ordinal(map, "token_count", work)?;
    let ids = field(map, "piece_ids", work)?
        .as_array()
        .ok_or("source shape piece ID array")?;
    if pieces != ids.len() {
        return Err("source shape inventory piece count".into());
    }
    if !available {
        literal(map, "reason", "lexical_error", work)?;
        if pieces != 0 || tokens != 0 {
            return Err("source shape unavailable counts".into());
        }
        return Ok(());
    }
    if pieces == 0 || tokens == 0 {
        return Err("source shape available EOF required".into());
    }
    let origin = entry.origin.as_ref().ok_or("source shape input origin")?;
    let bytes = input(origin, work)?;
    let mut previous = [0; 6];
    let mut token_index = 0usize;
    for (piece_index, id) in ids.iter().enumerate() {
        work.take(1)?;
        let id = id.as_str().ok_or("source shape piece ID string")?;
        work.text(id)?;
        let entry = log.get(id).ok_or("source shape unknown physical ID")?;
        let (end, is_token) = validate_piece(
            entry,
            root,
            &origin.location,
            piece_index,
            token_index,
            piece_index == pieces - 1,
            previous,
            work,
        )?;
        previous = end;
        if is_token {
            token_index = token_index
                .checked_add(1)
                .ok_or("source shape token count overflow")?;
        }
        *listed = listed
            .checked_add(1)
            .ok_or("source shape census overflow")?;
    }
    if token_index != tokens {
        return Err("source shape inventory token count".into());
    }
    if previous[0] != bytes {
        return Err("source shape final byte length".into());
    }
    Ok(())
}
#[allow(clippy::too_many_arguments)]
fn validate_piece(
    entry: &CVEntry,
    root: &str,
    path: &str,
    piece_index: usize,
    token_index: usize,
    last: bool,
    previous: [u64; 6],
    work: &mut Work,
) -> Result<([u64; 6], bool), String> {
    work.take(1)?;
    if entry.parent_ids.len() != 1 {
        return Err("source shape sole physical parent".into());
    }
    work.text(&entry.parent_ids[0])?;
    if entry.parent_ids[0] != root {
        return Err("source shape physical parent mismatch".into());
    }
    let origin = entry
        .origin
        .as_ref()
        .ok_or("source shape physical origin")?;
    let value = origin
        .meta
        .get(NS)
        .ok_or("source shape physical namespace")?;
    let map = value.as_object().ok_or("source shape piece object")?;
    let kind = text(map, "kind", work)?;
    let is_token = match kind {
        "token" | "eof" => true,
        "trivia" => false,
        _ => return Err("source shape piece kind".into()),
    };
    object(value, &PIECE[..if is_token { 9 } else { 7 }], work)?;
    literal(map, "schema", "1", work)?;
    literal(map, "role", "piece", work)?;
    literal(map, "input_id", root, work)?;
    if ordinal(map, "piece_index", work)? != piece_index {
        return Err("source shape piece ordinal".into());
    }
    if origin.source
        != if is_token {
            "lexer_token"
        } else {
            "lexer_trivia"
        }
    {
        return Err("source shape physical source".into());
    }
    legacy_location(&origin.location, path, work)?;
    let start = position(field(map, "start", work)?, work)?;
    let end = position(field(map, "end", work)?, work)?;
    if start != previous {
        return Err("source shape noncontiguous coordinates".into());
    }
    if (0..4).any(|i| end[i] < start[i])
        || (end[3] == start[3] && (end[4] < start[4] || end[5] < start[5]))
    {
        return Err("source shape backward coordinates".into());
    }
    if (kind == "eof") != last
        || (kind == "eof" && start != end)
        || (kind != "eof" && start[0] >= end[0])
    {
        return Err("source shape terminal EOF or interval width".into());
    }
    if is_token {
        let name = text(map, "token_type", work)?;
        work.take(TOKEN_NAMES.len())?;
        let legacy = TOKEN_NAMES
            .iter()
            .find(|(candidate, _)| *candidate == name)
            .map(|(_, legacy)| *legacy)
            .ok_or("source shape token alphabet")?;
        if (name == "EOF") != (kind == "eof") || ordinal(map, "token_index", work)? != token_index {
            return Err("source shape token ordinal or EOF type".into());
        }
        work.take(3)?;
        let index = u64::try_from(token_index).map_err(|_| "source shape legacy index overflow")?;
        if origin.meta.get("kind").and_then(Value::as_str) != Some(legacy)
            || origin.meta.get("token_index").and_then(Value::as_u64) != Some(index)
            || origin
                .meta
                .get("lexeme_byte_len")
                .and_then(Value::as_u64)
                .is_none()
        {
            return Err("source shape legacy token disagreement".into());
        }
    }
    Ok((end, is_token))
}
#[cfg(test)]
#[path = "source_domain_tests.rs"]
mod tests;
