//! Consumed-buffer authority for source inventory admission. D3a establishes
//! bounded shape; this separate read-only gate compares it with the real grammar.
//! Keeping original Strings by move makes a later filesystem change irrelevant.
use super::source_domain::ShapeAdmitted;
use super::source_inventory::token_name;
use coding_adventures_javascript_tokens::EsVersion;
use coding_adventures_sha256::Sha256Hasher;
use lexer::lexical::{LexicalError, LexicalLimits, LexicalPieceKind, SourcePosition};
use lexer::token::Token;
use serde_json::Value;
use std::mem::size_of;
use std::path::PathBuf;

const NS: &str = "closurec_source_v1";
const TOKEN_DOMAIN: &[u8] = b"closurec.lexical-token.v1\0";

#[derive(Clone, Copy)]
pub(super) struct ReplayLimits {
    source_capacity: usize,
    index_bytes: usize,
    fingerprint_bytes: usize,
    work: usize,
}
impl Default for ReplayLimits {
    fn default() -> Self {
        Self {
            source_capacity: 64 * 1024 * 1024,
            index_bytes: 16 * 1024 * 1024,
            fingerprint_bytes: 16 * 1024 * 1024,
            work: 64_000_000,
        }
    }
}
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
struct Cost {
    source_capacity: usize,
    index_bytes: usize,
    fingerprint_bytes: usize,
    work: usize,
}
struct TrustedInput {
    source: String,
    root: u64,
    fingerprints: Option<Vec<[u8; 32]>>,
}
pub(super) struct SourceCatalog {
    inputs: Vec<TrustedInput>,
    expected: usize,
    edition: EsVersion,
    limits: ReplayLimits,
    used: Cost,
}
struct Work {
    used: usize,
    limit: usize,
}
impl Work {
    fn take(&mut self, amount: usize) -> Result<(), String> {
        let next = add(self.used, amount)?;
        if next > self.limit {
            return Err("source replay work limit".into());
        }
        self.used = next;
        Ok(())
    }
}
fn add(a: usize, b: usize) -> Result<usize, String> {
    a.checked_add(b)
        .ok_or_else(|| "source replay arithmetic overflow".into())
}
fn mul(a: usize, b: usize) -> Result<usize, String> {
    a.checked_mul(b)
        .ok_or_else(|| "source replay arithmetic overflow".into())
}
fn catalog_storage(count: usize) -> Result<usize, String> {
    mul(count, size_of::<TrustedInput>())
}
fn fingerprint_storage(count: usize) -> Result<usize, String> {
    mul(count, size_of::<[u8; 32]>())
}
fn compact(id: &str, work: &mut Work) -> Result<u64, String> {
    // One visit plus two complete canonical-ID scans, before either scan.
    work.take(41)?;
    if id.len() != 20
        || !id.starts_with("cv1.")
        || !id.as_bytes()[4..]
            .iter()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b))
    {
        return Err("source replay root identity".into());
    }
    let number = u64::from_str_radix(&id[4..], 16).map_err(|_| "source replay root identity")?;
    if number == 0 {
        return Err("source replay root zero".into());
    }
    Ok(number)
}
fn feed(hasher: &mut Sha256Hasher, bytes: &[u8]) {
    // update buffers and drains internally: bounded chunks avoid quadratic
    // drains or a second source-sized allocation in the existing local hasher.
    for chunk in bytes.chunks(64) {
        hasher.update(chunk);
    }
}
fn framed_len(text: &str) -> Result<usize, String> {
    u64::try_from(text.len()).map_err(|_| "source replay string length exceeds u64")?;
    add(8, text.len())
}
fn frame(hasher: &mut Sha256Hasher, text: &str) -> Result<(), String> {
    let length =
        u64::try_from(text.len()).map_err(|_| "source replay string length exceeds u64")?;
    feed(hasher, &length.to_le_bytes());
    feed(hasher, text.as_bytes());
    Ok(())
}
fn fingerprint(token: &Token, work: &mut Work) -> Result<[u8; 32], String> {
    // Exhaustive destructuring and spelling prevent future fields/variants
    // from silently falling outside the full original-Token commitment.
    let Token {
        type_,
        value,
        line,
        column,
        type_name,
        flags,
        cv,
    } = token;
    work.take(1)?;
    if cv.is_some() {
        return Err("source replay stamped original Token".into());
    }
    let kind = token_name(*type_);
    let mut length = add(
        TOKEN_DOMAIN.len(),
        add(framed_len(kind)?, framed_len(value)?)?,
    )?;
    length = add(length, 19)?; // two u64 coordinates plus three option tags
    if let Some(name) = type_name {
        length = add(length, framed_len(name)?)?;
    }
    if flags.is_some() {
        length = add(length, 4)?;
    }
    work.take(add(mul(2, length)?, 128)?)?;
    let line = u64::try_from(*line).map_err(|_| "source replay line exceeds u64")?;
    let column = u64::try_from(*column).map_err(|_| "source replay column exceeds u64")?;
    let mut hasher = Sha256Hasher::new();
    feed(&mut hasher, TOKEN_DOMAIN);
    frame(&mut hasher, kind)?;
    frame(&mut hasher, value)?;
    feed(&mut hasher, &line.to_le_bytes());
    feed(&mut hasher, &column.to_le_bytes());
    match type_name {
        None => feed(&mut hasher, &[0]),
        Some(name) => {
            feed(&mut hasher, &[1]);
            frame(&mut hasher, name)?;
        }
    }
    match flags {
        None => feed(&mut hasher, &[0]),
        Some(flags) => {
            feed(&mut hasher, &[1]);
            feed(&mut hasher, &flags.to_le_bytes());
        }
    }
    feed(&mut hasher, &[0]);
    Ok(hasher.digest())
}
impl SourceCatalog {
    pub(super) fn new(
        expected: usize,
        edition: EsVersion,
        limits: ReplayLimits,
    ) -> Result<Self, String> {
        if limits.source_capacity > 256 * 1024 * 1024
            || limits.index_bytes > 64 * 1024 * 1024
            || limits.fingerprint_bytes > 64 * 1024 * 1024
            || limits.work > 512_000_000
        {
            return Err("source replay configuration ceiling".into());
        }
        let index_bytes = catalog_storage(expected)?;
        if index_bytes > limits.index_bytes {
            return Err("source replay catalog index limit".into());
        }
        let mut work = Work {
            used: 0,
            limit: limits.work,
        };
        work.take(expected)?;
        let mut inputs = Vec::new();
        inputs
            .try_reserve_exact(expected)
            .map_err(|_| "source replay catalog allocation")?;
        if catalog_storage(inputs.capacity())? > index_bytes {
            return Err("source replay catalog allocator capacity".into());
        }
        Ok(Self {
            inputs,
            expected,
            edition,
            limits,
            used: Cost {
                index_bytes,
                work: work.used,
                ..Cost::default()
            },
        })
    }
    pub(super) fn retain(
        &mut self,
        source: String,
        root: &str,
        tokens: Option<&[Token]>,
    ) -> Result<&str, String> {
        // Prospective ownership and work stay local until the entire new slot
        // has been admitted. An error never changes an already committed prefix.
        let mut next = self.used;
        let mut work = Work {
            used: next.work,
            limit: self.limits.work,
        };
        work.take(1)?;
        if self.inputs.len() >= self.expected {
            return Err("source replay catalog slot limit".into());
        }
        let root = compact(root, &mut work)?;
        if self.inputs.last().is_some_and(|last| last.root >= root) {
            return Err("source replay root order".into());
        }
        next.source_capacity = add(next.source_capacity, source.capacity())?;
        if next.source_capacity > self.limits.source_capacity {
            return Err("source replay source capacity limit".into());
        }
        let fingerprints = if let Some(tokens) = tokens {
            let bytes = fingerprint_storage(tokens.len())?;
            next.fingerprint_bytes = add(next.fingerprint_bytes, bytes)?;
            if next.fingerprint_bytes > self.limits.fingerprint_bytes {
                return Err("source replay fingerprint storage limit".into());
            }
            let mut fingerprints = Vec::new();
            fingerprints
                .try_reserve_exact(tokens.len())
                .map_err(|_| "source replay fingerprint allocation")?;
            if fingerprint_storage(fingerprints.capacity())? > bytes {
                return Err("source replay fingerprint allocator capacity".into());
            }
            for token in tokens {
                fingerprints.push(fingerprint(token, &mut work)?);
            }
            Some(fingerprints)
        } else {
            None
        };
        next.work = work.used;
        self.inputs.push(TrustedInput {
            source,
            root,
            fingerprints,
        });
        self.used = next;
        Ok(&self
            .inputs
            .last()
            .ok_or("source replay missing committed input")?
            .source)
    }
    pub(super) fn validate(
        &self,
        shape: &ShapeAdmitted<'_>,
        roots: &[String],
        paths: &[PathBuf],
    ) -> Result<(), String> {
        self.validate_core(shape, roots, paths).map(|_| ())
    }
    fn validate_core(
        &self,
        shape: &ShapeAdmitted<'_>,
        roots: &[String],
        paths: &[PathBuf],
    ) -> Result<usize, String> {
        let log = shape.log();
        let mut work = Work {
            used: self.used.work,
            limit: self.limits.work,
        };
        if self.inputs.len() != self.expected
            || roots.len() != self.expected
            || paths.len() != self.expected
        {
            return Err("source replay catalog cardinality".into());
        }
        let mut census = 0usize;
        for entry in log.entries().values() {
            work.take(1)?;
            if let Some(origin) = &entry.origin {
                work.take(origin.source.len())?;
                if origin.source == "input_file" {
                    census = add(census, 1)?;
                }
            }
        }
        if census != self.expected {
            return Err("source replay input census".into());
        }
        for ((input, root), path) in self.inputs.iter().zip(roots).zip(paths) {
            work.take(512)?;
            if compact(root, &mut work)? != input.root {
                return Err("source replay root order agreement".into());
            }
            work.take(mul(9, path.as_os_str().as_encoded_bytes().len())?)?;
            let entry = log.get(root).ok_or("source replay missing root")?;
            let origin = entry
                .origin
                .as_ref()
                .ok_or("source replay missing root origin")?;
            if origin.location != path.to_string_lossy() {
                return Err("source replay path disagreement".into());
            }
            let descriptor = origin
                .meta
                .get(NS)
                .ok_or("source replay input descriptor")?;
            if text(descriptor, "edition")? != self.edition.as_str() {
                return Err("source replay edition disagreement".into());
            }
            if number(descriptor, "byte_length")? != input.source.len() {
                return Err("source replay byte length disagreement".into());
            }
            let mut inventory = None;
            for contribution in &entry.contributions {
                work.take(add(
                    1,
                    add(contribution.source.len(), contribution.tag.len())?,
                )?)?;
                if contribution.source == "source.inventory" {
                    if inventory.is_some() {
                        return Err("source replay duplicate inventory".into());
                    }
                    inventory = contribution.meta.get(NS);
                }
            }
            let inventory = inventory.ok_or("source replay missing inventory")?;
            let blocks = add(
                input.source.len() / 64,
                usize::from(input.source.len() % 64 != 0),
            )?;
            work.take(add(add(input.source.len(), blocks)?, 128)?)?;
            let mut hasher = Sha256Hasher::new();
            feed(&mut hasher, input.source.as_bytes());
            if hasher.digest() != decode_digest(text(descriptor, "sha256")?)? {
                return Err("source replay digest disagreement".into());
            }
            work.take(1)?;
            let replay = coding_adventures_javascript_lexer::tokenize_javascript_typed_with_ranges(
                &input.source,
                self.edition,
                LexicalLimits::default(),
            );
            let available = text(inventory, "status")? == "available";
            let snapshot = match replay {
                Ok(snapshot) => {
                    if !available || input.fingerprints.is_none() {
                        return Err("source replay availability disagreement".into());
                    }
                    snapshot
                }
                Err(LexicalError::Lex(_)) => {
                    if available || input.fingerprints.is_some() {
                        return Err("source replay availability disagreement".into());
                    }
                    continue;
                }
                Err(_) => return Err("source replay capture refused".into()),
            };
            let fingerprints = input
                .fingerprints
                .as_ref()
                .ok_or("source replay availability missing original tokens")?;
            if fingerprints.len() != snapshot.tokens().len() {
                return Err("source replay fingerprint count disagreement".into());
            }
            let ids = inventory
                .get("piece_ids")
                .and_then(Value::as_array)
                .ok_or("source replay piece IDs")?;
            if ids.len() != snapshot.pieces().len()
                || number(inventory, "token_count")? != snapshot.tokens().len()
            {
                return Err("source replay kind counts disagreement".into());
            }
            for (index, (piece, id)) in snapshot.pieces().iter().zip(ids).enumerate() {
                work.take(1024)?;
                let entry = log
                    .get(id.as_str().ok_or("source replay piece ID")?)
                    .ok_or("source replay missing piece")?;
                let origin = entry
                    .origin
                    .as_ref()
                    .ok_or("source replay missing piece origin")?;
                let meta = origin
                    .meta
                    .get(NS)
                    .ok_or("source replay piece descriptor")?;
                let (kind, ordinal) = match piece.kind {
                    LexicalPieceKind::Trivia => ("trivia", None),
                    LexicalPieceKind::Token(i) => ("token", Some(i)),
                    LexicalPieceKind::Eof(i) => ("eof", Some(i)),
                };
                if text(meta, "kind")? != kind || number(meta, "piece_index")? != index {
                    return Err("source replay kind disagreement".into());
                }
                coordinates(
                    meta.get("start").ok_or("source replay start")?,
                    piece.range.start,
                )?;
                coordinates(meta.get("end").ok_or("source replay end")?, piece.range.end)?;
                let (line, column) = if let Some(i) = ordinal {
                    let token = snapshot
                        .tokens()
                        .get(i)
                        .ok_or("source replay token ordinal")?;
                    let name = token_name(token.type_);
                    if number(meta, "token_index")? != i || text(meta, "token_type")? != name {
                        return Err("source replay kind token disagreement".into());
                    }
                    if !origin
                        .meta
                        .get("kind")
                        .and_then(Value::as_str)
                        .is_some_and(|kind| kind.eq_ignore_ascii_case(name))
                        || origin.meta.get("token_index").and_then(Value::as_u64)
                            != Some(u64::try_from(i).map_err(|_| "source replay ordinal overflow")?)
                        || origin.meta.get("lexeme_byte_len").and_then(Value::as_u64)
                            != Some(
                                u64::try_from(token.value.len())
                                    .map_err(|_| "source replay length overflow")?,
                            )
                    {
                        return Err("source replay legacy token disagreement".into());
                    }
                    if fingerprint(token, &mut work)? != fingerprints[i] {
                        return Err("source replay fingerprint disagreement".into());
                    }
                    (token.line, token.column)
                } else {
                    (
                        add(piece.range.start.line, 1)?,
                        add(piece.range.start.scalar_column, 1)?,
                    )
                };
                work.take(mul(3, origin.location.len())?)?;
                let (prefix, actual_column) = origin
                    .location
                    .rsplit_once(':')
                    .ok_or("source replay legacy location")?;
                let (_, actual_line) = prefix
                    .rsplit_once(':')
                    .ok_or("source replay legacy location")?;
                if actual_line.parse::<usize>().ok() != Some(line)
                    || actual_column.parse::<usize>().ok() != Some(column)
                {
                    return Err("source replay legacy location disagreement".into());
                }
            }
        }
        Ok(work.used)
    }
}
fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| "source replay missing text field".into())
}
fn number(value: &Value, key: &str) -> Result<usize, String> {
    text(value, key)?
        .parse()
        .map_err(|_| "source replay numeric field".into())
}
fn coordinates(value: &Value, expected: SourcePosition) -> Result<(), String> {
    let SourcePosition {
        byte_offset,
        scalar_offset,
        utf16_offset,
        line,
        scalar_column,
        utf16_column,
    } = expected;
    for (key, actual) in [
        ("byte_offset", byte_offset),
        ("scalar_offset", scalar_offset),
        ("utf16_offset", utf16_offset),
        ("line", line),
        ("scalar_column", scalar_column),
        ("utf16_column", utf16_column),
    ] {
        if number(value, key)? != actual {
            return Err("source replay coordinates disagreement".into());
        }
    }
    Ok(())
}
fn decode_digest(text: &str) -> Result<[u8; 32], String> {
    if text.len() != 64 {
        return Err("source replay digest encoding".into());
    }
    let mut digest = [0; 32];
    for (slot, pair) in digest.iter_mut().zip(text.as_bytes().as_chunks::<2>().0) {
        let nibble = |b: u8| match b {
            b'0'..=b'9' => Ok(b - b'0'),
            b'a'..=b'f' => Ok(b - b'a' + 10),
            _ => Err("source replay digest encoding"),
        };
        *slot = nibble(pair[0])? * 16 + nibble(pair[1])?;
    }
    Ok(digest)
}
#[cfg(test)]
#[path = "source_replay_tests.rs"]
mod tests;
