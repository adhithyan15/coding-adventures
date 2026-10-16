//! Producer-owned source evidence. The borrowed successful matcher snapshot is
//! the authority; a public caller-built range table is not an ingestion proof.
//! Precharge the entire file's closed metadata before constructing any Value.
//! The generic graph budget remains independent. Typed-parser ID reuse follows
//! in D2; this module binds only actual input/trivia/token/EOF ingestion.
use coding_adventures_correlation_vector::{CVLog, Origin};
use coding_adventures_javascript_tokens::EsVersion;
use lexer::lexical::{
    LexicalError, LexicalLimits, LexicalPiece, LexicalPieceKind, LexicalSnapshot, SourcePosition,
};
use lexer::token::{Token, TokenType};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::mem::size_of;
use std::path::Path;

const NS: &str = "closurec_source_v1";
const ID: &str = "cv1.0000000000000000";
const HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Clone, Copy)]
pub(super) struct SourceInventoryLimits {
    values: usize,
    metadata_bytes: usize,
    reference_bytes: usize,
    work: usize,
}
impl Default for SourceInventoryLimits {
    fn default() -> Self {
        Self {
            values: 1_000_000,
            metadata_bytes: 64 * 1024 * 1024,
            reference_bytes: 16 * 1024 * 1024,
            work: 64_000_000,
        }
    }
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct Cost {
    values: usize,
    metadata_bytes: usize,
    reference_bytes: usize,
    work: usize,
}
impl Cost {
    fn plus(self, other: Self) -> Result<Self, String> {
        Ok(Self {
            values: add(self.values, other.values)?,
            metadata_bytes: add(self.metadata_bytes, other.metadata_bytes)?,
            reference_bytes: add(self.reference_bytes, other.reference_bytes)?,
            work: add(self.work, other.work)?,
        })
    }
}
pub(super) struct SourceInventoryBudget {
    limits: SourceInventoryLimits,
    used: Cost,
}
impl SourceInventoryBudget {
    pub(super) fn new(limits: SourceInventoryLimits) -> Result<Self, String> {
        if limits.values > 4_000_000
            || limits.metadata_bytes > 128 * 1024 * 1024
            || limits.reference_bytes > 64 * 1024 * 1024
            || limits.work > 512_000_000
        {
            return Err("source inventory configuration ceiling".into());
        }
        Ok(Self {
            limits,
            used: Cost::default(),
        })
    }
    fn precharge(
        &mut self,
        snapshot: Option<&LexicalSnapshot<'_>>,
        bytes: usize,
        edition: EsVersion,
    ) -> Result<(), String> {
        let n = snapshot.map_or(0, |s| s.pieces().len());
        let t = snapshot.map_or(0, |s| s.tokens().len());
        // Admission precedes the descriptor walk. Each next piece is charged
        // before visiting it; cumulative namespace costs precede any ownership.
        let mut cost = Cost {
            values: if snapshot.is_some() { add(18, n)? } else { 19 },
            metadata_bytes: add(
                input_bytes(bytes, edition)?,
                inventory_bytes(n, t, snapshot.is_some())?,
            )?,
            reference_bytes: 0,
            work: bytes,
        };
        self.check_work(cost.work)?;
        if let Some(snapshot) = snapshot {
            cost.reference_bytes = add(
                multiply(n, add(size_of::<Value>(), 40)?)?,
                multiply(t, add(size_of::<String>(), 20)?)?,
            )?;
            if n > 0 {
                cost.reference_bytes = add(cost.reference_bytes, 64)?;
            }
            for (index, piece) in snapshot.pieces().iter().enumerate() {
                cost.work = add(cost.work, 1)?;
                self.check_work(cost.work)?;
                let (ordinal, name) = piece_token(piece, snapshot.tokens())?;
                cost.values = add(cost.values, if ordinal.is_some() { 23 } else { 21 })?;
                cost.metadata_bytes = add(
                    cost.metadata_bytes,
                    piece_bytes(index, piece, ordinal, name)?,
                )?;
                self.check_work(add(cost.work, add(cost.values, cost.metadata_bytes)?)?)?;
            }
        }
        cost.work = add(
            cost.work,
            add(
                add(cost.values, cost.metadata_bytes)?,
                add(multiply(n, 2)?, t)?,
            )?,
        )?;
        let next = self.used.plus(cost)?;
        if next.values > self.limits.values {
            return Err("source inventory values limit".into());
        }
        if next.metadata_bytes > self.limits.metadata_bytes {
            return Err("source inventory encoded metadata limit".into());
        }
        if next.reference_bytes > self.limits.reference_bytes {
            return Err("source inventory reference storage limit".into());
        }
        if next.work > self.limits.work {
            return Err("source inventory work limit".into());
        }
        self.used = next;
        Ok(())
    }
    fn check_work(&self, work: usize) -> Result<(), String> {
        if add(self.used.work, work)? > self.limits.work {
            Err("source inventory work limit".into())
        } else {
            Ok(())
        }
    }
}
fn add(a: usize, b: usize) -> Result<usize, String> {
    a.checked_add(b)
        .ok_or_else(|| "source inventory arithmetic overflow".into())
}
fn multiply(a: usize, b: usize) -> Result<usize, String> {
    a.checked_mul(b)
        .ok_or_else(|| "source inventory arithmetic overflow".into())
}
fn decimal_len(n: usize) -> Result<usize, String> {
    let n = u64::try_from(n).map_err(|_| "source coordinate exceeds u64")?;
    Ok(if n == 0 { 1 } else { n.ilog10() as usize + 1 })
}
fn decimal(n: usize) -> String {
    n.to_string()
}
fn json_string_bytes(s: &str) -> Result<usize, String> {
    let mut n = 2;
    for ch in s.chars() {
        n = add(
            n,
            match ch {
                '"' | '\\' | '\u{0008}' | '\u{000c}' | '\n' | '\r' | '\t' => 2,
                ch if ch < '\u{0020}' => 6,
                ch => ch.len_utf8(),
            },
        )?;
    }
    Ok(n)
}
fn number_bytes(n: usize) -> Result<usize, String> {
    add(decimal_len(n)?, 2)
}
fn object_bytes(fields: &[(&str, usize)]) -> Result<usize, String> {
    let mut n = add(2, fields.len().saturating_sub(1))?;
    for (key, value) in fields {
        n = add(n, add(add(json_string_bytes(key)?, 1)?, *value)?)?;
    }
    Ok(n)
}
fn namespace_bytes(value: usize) -> Result<usize, String> {
    object_bytes(&[(NS, value)])
}
fn input_bytes(bytes: usize, edition: EsVersion) -> Result<usize, String> {
    namespace_bytes(object_bytes(&[
        ("schema", json_string_bytes("1")?),
        ("role", json_string_bytes("input")?),
        ("buffer_domain", json_string_bytes("read_to_string_utf8")?),
        ("encoding", json_string_bytes("utf8")?),
        ("grammar", json_string_bytes("javascript_standard")?),
        ("edition", json_string_bytes(edition.as_str())?),
        ("sha256", json_string_bytes(HASH)?),
        ("byte_length", number_bytes(bytes)?),
    ])?)
}
fn coordinate_bytes(p: SourcePosition) -> Result<usize, String> {
    object_bytes(&[
        ("byte_offset", number_bytes(p.byte_offset)?),
        ("scalar_offset", number_bytes(p.scalar_offset)?),
        ("utf16_offset", number_bytes(p.utf16_offset)?),
        ("line", number_bytes(p.line)?),
        ("scalar_column", number_bytes(p.scalar_column)?),
        ("utf16_column", number_bytes(p.utf16_column)?),
    ])
}
fn piece_bytes(
    index: usize,
    piece: &LexicalPiece,
    ordinal: Option<usize>,
    name: &str,
) -> Result<usize, String> {
    let fields = [
        ("schema", json_string_bytes("1")?),
        ("role", json_string_bytes("piece")?),
        ("input_id", json_string_bytes(ID)?),
        ("piece_index", number_bytes(index)?),
        ("kind", json_string_bytes(piece_kind(piece))?),
        ("start", coordinate_bytes(piece.range.start)?),
        ("end", coordinate_bytes(piece.range.end)?),
        ("token_index", number_bytes(ordinal.unwrap_or(0))?),
        ("token_type", json_string_bytes(name)?),
    ];
    namespace_bytes(object_bytes(
        &fields[..if ordinal.is_some() { 9 } else { 7 }],
    )?)
}
fn inventory_bytes(n: usize, t: usize, available: bool) -> Result<usize, String> {
    let ids = add(
        add(2, n.saturating_sub(1))?,
        multiply(n, json_string_bytes(ID)?)?,
    )?;
    let fields = [
        ("schema", json_string_bytes("1")?),
        ("role", json_string_bytes("inventory")?),
        (
            "status",
            json_string_bytes(if available {
                "available"
            } else {
                "unavailable"
            })?,
        ),
        ("piece_count", number_bytes(n)?),
        ("token_count", number_bytes(t)?),
        ("piece_ids", ids),
        ("reason", json_string_bytes("lexical_error")?),
    ];
    namespace_bytes(object_bytes(&fields[..if available { 6 } else { 7 }])?)
}
fn piece_kind(piece: &LexicalPiece) -> &'static str {
    match piece.kind {
        LexicalPieceKind::Token(_) => "token",
        LexicalPieceKind::Trivia => "trivia",
        LexicalPieceKind::Eof(_) => "eof",
    }
}
fn piece_token<'a>(
    piece: &LexicalPiece,
    tokens: &'a [Token],
) -> Result<(Option<usize>, &'a str), String> {
    match piece.kind {
        LexicalPieceKind::Trivia => Ok((None, "")),
        LexicalPieceKind::Token(i) | LexicalPieceKind::Eof(i) => {
            let token = tokens
                .get(i)
                .ok_or("source inventory invalid token ordinal")?;
            Ok((Some(i), token_name(token.type_)))
        }
    }
}
// Exhaustive enum matching freezes wire spelling independently of Debug/Display.
pub(super) fn token_name(kind: TokenType) -> &'static str {
    match kind {
        TokenType::Name => "Name",
        TokenType::Number => "Number",
        TokenType::String => "String",
        TokenType::Keyword => "Keyword",
        TokenType::Plus => "Plus",
        TokenType::Minus => "Minus",
        TokenType::Star => "Star",
        TokenType::Slash => "Slash",
        TokenType::Equals => "Equals",
        TokenType::EqualsEquals => "EqualsEquals",
        TokenType::LParen => "LParen",
        TokenType::RParen => "RParen",
        TokenType::Comma => "Comma",
        TokenType::Colon => "Colon",
        TokenType::Semicolon => "Semicolon",
        TokenType::LBrace => "LBrace",
        TokenType::RBrace => "RBrace",
        TokenType::LBracket => "LBracket",
        TokenType::RBracket => "RBracket",
        TokenType::Dot => "Dot",
        TokenType::Bang => "Bang",
        TokenType::Newline => "Newline",
        TokenType::Indent => "Indent",
        TokenType::Dedent => "Dedent",
        TokenType::Eof => "EOF",
    }
}
fn compact_id(id: &str) -> Result<(), String> {
    if id.len() != 20
        || !id.starts_with("cv1.")
        || !id.as_bytes()[4..]
            .iter()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b))
    {
        return Err("source inventory noncompact identity".into());
    }
    Ok(())
}
fn check_capacity(actual: usize, admitted: usize) -> Result<(), String> {
    if actual > admitted {
        Err("source inventory allocator capacity exceeds admitted storage".into())
    } else {
        Ok(())
    }
}
fn reference_copy(id: &str) -> Result<String, String> {
    compact_id(id)?;
    let mut owned = String::new();
    owned
        .try_reserve_exact(20)
        .map_err(|_| "source inventory reference allocation")?;
    check_capacity(owned.capacity(), 20)?;
    owned.push_str(id);
    Ok(owned)
}
fn reference_vector<T>(n: usize) -> Result<Vec<T>, String> {
    let mut owned = Vec::new();
    owned
        .try_reserve_exact(n)
        .map_err(|_| "source inventory vector allocation")?;
    check_capacity(owned.capacity(), n)?;
    Ok(owned)
}
pub(super) fn position(p: SourcePosition) -> Value {
    json!({"byte_offset":decimal(p.byte_offset),"scalar_offset":decimal(p.scalar_offset),"utf16_offset":decimal(p.utf16_offset),"line":decimal(p.line),"scalar_column":decimal(p.scalar_column),"utf16_column":decimal(p.utf16_column)})
}
fn input_meta(source: &str, edition: EsVersion) -> HashMap<String, Value> {
    let hash = coding_adventures_sha256::sha256_hex(source.as_bytes());
    HashMap::from([
        ("byte_len".into(), json!(source.len())),
        ("content_sha256".into(), json!(hash)),
        (
            NS.into(),
            json!({"schema":"1","role":"input","buffer_domain":"read_to_string_utf8","encoding":"utf8","grammar":"javascript_standard","edition":edition.as_str(),"sha256":hash,"byte_length":decimal(source.len())}),
        ),
    ])
}
fn piece_meta(
    root: &str,
    index: usize,
    piece: &LexicalPiece,
    ordinal: Option<usize>,
    name: &str,
) -> Result<Value, String> {
    let mut value = json!({"schema":"1","role":"piece","input_id":null,"piece_index":decimal(index),"kind":piece_kind(piece),"start":position(piece.range.start),"end":position(piece.range.end)});
    value["input_id"] = Value::String(reference_copy(root)?);
    if let Some(i) = ordinal {
        value["token_index"] = json!(decimal(i));
        value["token_type"] = json!(name);
    }
    Ok(value)
}
pub(super) struct RecordedInput {
    pub(super) id: String,
    pub(super) tokens: Result<Vec<Token>, String>,
    pub(super) token_ids: Vec<String>,
}

/// D2 transfers the already-precharged ID buffers into the actual captured
/// Tokens. Check the complete borrowed stream before changing its first slot:
/// a private invariant refusal must not leave a half-stamped parser input.
/// This is a linear trusted ingestion adapter, not serialized-domain admission.
pub(super) fn stamp_ingestion_ids(tokens: &mut [Token], ids: Vec<String>) -> Result<(), String> {
    if tokens.len() != ids.len() {
        return Err("captured ingestion token/identity length mismatch".into());
    }
    if tokens.iter().any(|token| token.cv.is_some()) {
        return Err("captured ingestion token already has an identity".into());
    }
    for id in &ids {
        compact_id(id)?;
    }
    for (token, id) in tokens.iter_mut().zip(ids) {
        token.cv = Some(id);
    }
    Ok(())
}

pub(super) fn record_file(
    log: &mut CVLog,
    path: &Path,
    source: &str,
    edition: EsVersion,
    budget: &mut SourceInventoryBudget,
) -> Result<RecordedInput, String> {
    let capture = coding_adventures_javascript_lexer::tokenize_javascript_typed_with_ranges(
        source,
        edition,
        LexicalLimits::default(),
    );
    record_capture(log, path, source, edition, budget, capture)
}
fn record_capture(
    log: &mut CVLog,
    path: &Path,
    source: &str,
    edition: EsVersion,
    budget: &mut SourceInventoryBudget,
    capture: Result<LexicalSnapshot<'_>, LexicalError>,
) -> Result<RecordedInput, String> {
    let (snapshot, error) = match capture {
        Ok(snapshot) => (Some(snapshot), None),
        Err(LexicalError::Lex(error)) => (None, Some(error)),
        Err(error) => return Err(error.to_string()),
    };
    budget.precharge(snapshot.as_ref(), source.len(), edition)?;
    let n = snapshot.as_ref().map_or(0, |s| s.pieces().len());
    let t = snapshot.as_ref().map_or(0, |s| s.tokens().len());
    let mut piece_ids = reference_vector::<Value>(n)?;
    let mut token_ids = reference_vector::<String>(t)?;
    let root = log.try_create(Some(Origin {
        source: "input_file".into(),
        location: path.to_string_lossy().into_owned(),
        timestamp: None,
        meta: input_meta(source, edition),
    }))?;
    compact_id(&root)?;
    if let Some(snapshot) = &snapshot {
        for (index, piece) in snapshot.pieces().iter().enumerate() {
            let (ordinal, name) = piece_token(piece, snapshot.tokens())?;
            let reserved = piece_meta(&root, index, piece, ordinal, name)?;
            let mut meta = HashMap::from([(NS.into(), reserved)]);
            let (origin_source, location) = if let Some(i) = ordinal {
                let token = &snapshot.tokens()[i];
                meta.insert("kind".into(), json!(name.to_ascii_lowercase()));
                meta.insert("lexeme_byte_len".into(), json!(token.value.len()));
                meta.insert("token_index".into(), json!(i));
                (
                    "lexer_token",
                    format!("{}:{}:{}", path.to_string_lossy(), token.line, token.column),
                )
            } else {
                (
                    "lexer_trivia",
                    format!(
                        "{}:{}:{}",
                        path.to_string_lossy(),
                        add(piece.range.start.line, 1)?,
                        add(piece.range.start.scalar_column, 1)?
                    ),
                )
            };
            let id = log.try_derive(
                &root,
                Some(Origin {
                    source: origin_source.into(),
                    location,
                    timestamp: None,
                    meta,
                }),
            )?;
            compact_id(&id)?;
            check_capacity(id.capacity(), 64)?;
            piece_ids.push(Value::String(reference_copy(&id)?));
            if ordinal.is_some() {
                token_ids.push(reference_copy(&id)?);
            }
        }
    }
    let mut inventory = json!({"schema":"1","role":"inventory","status":if snapshot.is_some(){"available"}else{"unavailable"},"piece_count":decimal(n),"token_count":decimal(t),"piece_ids":[]});
    inventory["piece_ids"] = Value::Array(piece_ids);
    if snapshot.is_none() {
        inventory["reason"] = json!("lexical_error");
    }
    log.contribute(
        &root,
        "source.inventory",
        "finalized",
        HashMap::from([(NS.into(), inventory)]),
    )?;
    let tokens = match snapshot {
        Some(snapshot) => Ok(snapshot.into_tokens()),
        None => Err(format!(
            "JavaScript tokenization failed: {}",
            error.ok_or("missing ordinary lexical error")?
        )),
    };
    Ok(RecordedInput {
        id: root,
        tokens,
        token_ids,
    })
}

#[cfg(test)]
#[path = "source_inventory_tests.rs"]
mod tests;
