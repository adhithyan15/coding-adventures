//! Original array/member occurrences are provisional until D3b certifies physical facts.
//! The callback records actual construction; final admission joins those private
//! receipts to replay-admitted source and exact Merge chronology before publish.
use super::source_replay::SourceReplayAdmitted;
use coding_adventures_correlation_vector::{CVLog, JournalEvent, Origin};
use coding_adventures_javascript_parser::bridge::{
    ArrayOccurrenceObserver, CompositeOccurrenceObserver,
};
use lexer::lexical::{SourcePosition, SourceRange};
use lexer::token::{Token, TokenType};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::mem::size_of;
const NS: &str = "closurec_node_v1";
const SOURCE: &str = "closurec_source_v1";
const META_BYTES: usize = 4096;
#[derive(Clone, Copy)]
pub(super) struct ArrayLimits {
    pub(super) work: usize,
    pub(super) buffer_bytes: usize,
    pub(super) ledger_bytes: usize,
    pub(super) metadata_bytes: usize,
}
impl Default for ArrayLimits {
    fn default() -> Self {
        Self {
            work: 64_000_000,
            buffer_bytes: 32 * 1024 * 1024,
            ledger_bytes: 16 * 1024 * 1024,
            metadata_bytes: 16 * 1024 * 1024,
        }
    }
}
#[derive(Clone, Copy)]
enum NodeKind {
    Array,
    Member,
    OptionalMember,
}
impl NodeKind {
    fn name(self) -> &'static str {
        match self {
            Self::Array => "ArrayExpression",
            Self::Member => "MemberExpression",
            Self::OptionalMember => "OptionalMemberExpression",
        }
    }
    fn source(self) -> &'static str {
        match self {
            Self::Array => "typed_array",
            Self::Member | Self::OptionalMember => "typed_member",
        }
    }
    fn location(self) -> &'static str {
        match self {
            Self::Array => "array_literal",
            Self::Member | Self::OptionalMember => "member_prefix",
        }
    }
}
#[derive(Clone, Copy)]
struct Receipt {
    kind: NodeKind,
    first_kind: TokenType,
    last_kind: TokenType,
    id: u64,
    root: u64,
    first: u64,
    last: u64,
    first_index: usize,
    last_index: usize,
    merge_sequence: u64,
}
pub(super) struct ArrayLedger {
    limits: ArrayLimits,
    used_work: usize,
    metadata: usize,
    receipts: Vec<Receipt>,
}
struct Work {
    used: usize,
    limit: usize,
}
fn add(a: usize, b: usize) -> Result<usize, String> {
    a.checked_add(b)
        .ok_or_else(|| "array source arithmetic overflow".into())
}
fn mul(a: usize, b: usize) -> Result<usize, String> {
    a.checked_mul(b)
        .ok_or_else(|| "array source arithmetic overflow".into())
}
impl Work {
    fn take(&mut self, n: usize) -> Result<(), String> {
        let next = add(self.used, n)?;
        if next > self.limit {
            return Err("array source work limit".into());
        }
        self.used = next;
        Ok(())
    }
    fn text(&mut self, s: &str) -> Result<(), String> {
        self.take(add(1, s.len())?)
    }
}
fn compact(s: &str, w: &mut Work) -> Result<u64, String> {
    w.text(s)?;
    w.take(mul(2, s.len())?)?;
    if s.len() != 20
        || !s.starts_with("cv1.")
        || !s.as_bytes()[4..]
            .iter()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(c))
    {
        return Err("array source compact identity".into());
    }
    let n = u64::from_str_radix(&s[4..], 16).map_err(|_| "array source compact identity")?;
    if n == 0 {
        return Err("array source zero identity".into());
    }
    Ok(n)
}
/// The final consumer needs canonical lookup keys, not four heap allocations.
/// Its caller buys all encoding/UTF8 scans and this fixed stack space first.
fn id(mut n: u64) -> [u8; 20] {
    let mut out = *b"cv1.0000000000000000";
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for slot in out[4..].iter_mut().rev() {
        *slot = HEX[(n & 15) as usize];
        n >>= 4;
    }
    out
}
fn number(v: &Value, w: &mut Work) -> Result<usize, String> {
    let s = v.as_str().ok_or("array source decimal type")?;
    w.text(s)?;
    w.take(mul(2, s.len())?)?;
    if s.is_empty()
        || s.len() > 20
        || !s.bytes().all(|c| c.is_ascii_digit())
        || (s.len() > 1 && s.starts_with('0'))
    {
        return Err("array source decimal spelling".into());
    }
    s.parse().map_err(|_| "array source decimal range".into())
}
fn object<'a>(v: &'a Value, keys: &[&str], w: &mut Work) -> Result<&'a Map<String, Value>, String> {
    let m = v.as_object().ok_or("array source object")?;
    w.take(add(1, m.len())?)?;
    if m.len() != keys.len() {
        return Err("array source closed fields".into());
    }
    for k in m.keys() {
        w.text(k)?;
        if !keys.contains(&k.as_str()) {
            return Err("array source unknown field".into());
        }
    }
    Ok(m)
}
fn position(v: &Value, w: &mut Work) -> Result<SourcePosition, String> {
    let m = object(
        v,
        &[
            "byte_offset",
            "scalar_offset",
            "utf16_offset",
            "line",
            "scalar_column",
            "utf16_column",
        ],
        w,
    )?;
    Ok(SourcePosition {
        byte_offset: number(&m["byte_offset"], w)?,
        scalar_offset: number(&m["scalar_offset"], w)?,
        utf16_offset: number(&m["utf16_offset"], w)?,
        line: number(&m["line"], w)?,
        scalar_column: number(&m["scalar_column"], w)?,
        utf16_column: number(&m["utf16_column"], w)?,
    })
}
fn entry<'a>(
    log: &'a CVLog,
    key: &str,
    w: &mut Work,
) -> Result<&'a coding_adventures_correlation_vector::CVEntry, String> {
    compact(key, w)?;
    log.get(key)
        .ok_or_else(|| "array source missing entry".into())
}
fn source_piece<'a>(
    log: &'a CVLog,
    key: &str,
    root: &str,
    w: &mut Work,
) -> Result<&'a Map<String, Value>, String> {
    let e = entry(log, key, w)?;
    w.take(e.parent_ids.len())?;
    if e.parent_ids.len() != 1 {
        return Err("array source piece parent".into());
    }
    w.text(&e.parent_ids[0])?;
    if e.parent_ids[0] != root {
        return Err("array source foreign piece".into());
    }
    let o = e.origin.as_ref().ok_or("array source piece origin")?;
    w.text(&o.source)?;
    if o.source != "lexer_token" && o.source != "lexer_trivia" {
        return Err("array source physical origin".into());
    }
    w.take(add(1, o.meta.len())?)?;
    let m = o
        .meta
        .get(SOURCE)
        .and_then(Value::as_object)
        .ok_or("array source piece namespace")?;
    let owner = m
        .get("input_id")
        .and_then(Value::as_str)
        .ok_or("array source piece root")?;
    compact(owner, w)?;
    if owner != root {
        return Err("array source foreign piece root".into());
    }
    Ok(m)
}
fn inventory<'a>(log: &'a CVLog, root: &str, w: &mut Work) -> Result<&'a [Value], String> {
    let e = entry(log, root, w)?;
    let o = e.origin.as_ref().ok_or("array source input origin")?;
    w.text(&o.source)?;
    if o.source != "input_file" {
        return Err("array source input root".into());
    }
    let mut found = None;
    for c in &e.contributions {
        w.take(1)?;
        w.text(&c.source)?;
        w.text(&c.tag)?;
        if c.source == "source.inventory" && c.tag == "finalized" {
            if found.is_some() {
                return Err("array source duplicate inventory".into());
            }
            w.take(add(1, c.meta.len())?)?;
            let m = c
                .meta
                .get(SOURCE)
                .and_then(Value::as_object)
                .ok_or("array source inventory namespace")?;
            if m.get("status").and_then(Value::as_str) != Some("available") {
                return Err("array source unavailable inventory".into());
            }
            found = Some(
                m.get("piece_ids")
                    .and_then(Value::as_array)
                    .ok_or("array source inventory pieces")?
                    .as_slice(),
            );
        }
    }
    found.ok_or_else(|| "array source missing inventory".into())
}
fn boundary(
    log: &CVLog,
    t: &Token,
    root: &str,
    kind: &str,
    w: &mut Work,
) -> Result<(u64, usize, SourceRange), String> {
    let key =
        t.cv.as_deref()
            .ok_or("array source missing physical boundary")?;
    let seq = compact(key, w)?;
    let p = source_piece(log, key, root, w)?;
    for (field, expected) in [("role", "piece"), ("kind", "token"), ("token_type", kind)] {
        let actual = p
            .get(field)
            .and_then(Value::as_str)
            .ok_or("array source boundary kind")?;
        w.text(actual)?;
        if actual != expected {
            return Err("array source boundary kind".into());
        }
    }
    let index = number(
        p.get("piece_index").ok_or("array source boundary index")?,
        w,
    )?;
    let range = SourceRange {
        start: position(p.get("start").ok_or("array source boundary start")?, w)?,
        end: position(p.get("end").ok_or("array source boundary end")?, w)?,
    };
    Ok((seq, index, range))
}
fn copy_id(s: &str, w: &mut Work) -> Result<String, String> {
    compact(s, w)?;
    w.take(20)?;
    let mut out = String::new();
    out.try_reserve_exact(20)
        .map_err(|_| "array source identity allocation")?;
    if out.capacity() > 64 {
        return Err("array source identity allocator capacity".into());
    }
    out.push_str(s);
    Ok(out)
}
impl ArrayLedger {
    pub(super) fn new(limits: ArrayLimits) -> Result<Self, String> {
        if limits.work > 512_000_000
            || limits.buffer_bytes > 128 * 1024 * 1024
            || limits.ledger_bytes > 64 * 1024 * 1024
            || limits.metadata_bytes > 64 * 1024 * 1024
        {
            return Err("array source configuration ceiling".into());
        }
        Ok(Self {
            limits,
            used_work: 0,
            metadata: 0,
            receipts: Vec::new(),
        })
    }
    pub(super) fn observer<'a>(
        &'a mut self,
        log: &'a mut CVLog,
        root: &'a str,
    ) -> Result<Observer<'a>, String> {
        let mut w = Work {
            used: self.used_work,
            limit: self.limits.work,
        };
        let root_sequence = compact(root, &mut w)?;
        if log.journal().is_none() {
            return Err("array source chronology required".into());
        }
        self.used_work = w.used;
        Ok(Observer {
            ledger: self,
            log,
            root,
            root_sequence,
        })
    }
    fn reserve_receipt(&mut self, w: &mut Work) -> Result<(), String> {
        if self.receipts.len() < self.receipts.capacity() {
            return Ok(());
        }
        let old = self.receipts.capacity();
        let next = if old == 0 { 1 } else { mul(old, 2)? };
        let bytes = mul(next, size_of::<Receipt>())?;
        if bytes > self.limits.ledger_bytes {
            return Err("array source ledger storage limit".into());
        }
        if add(bytes, mul(old, size_of::<Receipt>())?)? > self.limits.buffer_bytes {
            return Err("array source buffer limit".into());
        }
        w.take(add(next, old)?)?;
        self.receipts
            .try_reserve_exact(next - self.receipts.len())
            .map_err(|_| "array source ledger allocation")?;
        if self.receipts.capacity() > next {
            return Err("array source ledger allocator capacity".into());
        }
        Ok(())
    }
    pub(super) fn validate(&self, source: &SourceReplayAdmitted<'_>) -> Result<(), String> {
        self.validate_core(source).map(|_| ())
    }
    fn validate_core(&self, source: &SourceReplayAdmitted<'_>) -> Result<usize, String> {
        let log = source.log();
        let mut w = Work {
            used: self.used_work,
            limit: self.limits.work,
        };
        let mut previous = (0, 0);
        for row in &self.receipts {
            w.take(1)?;
            if row.id <= previous.0 || row.merge_sequence <= previous.1 {
                return Err("array source receipt order".into());
            }
            previous = (row.id, row.merge_sequence);
            // Buy all four fixed encodings and UTF8 scans before preparing keys.
            // Retained receipts coexist with this 80-byte stack scratch only;
            // final admission owns no parent/metadata/ID heap buffers.
            w.take(160)?;
            if add(mul(self.receipts.capacity(), size_of::<Receipt>())?, 80)?
                > self.limits.buffer_bytes
            {
                return Err("array source final buffer limit".into());
            }
            let key_bytes = id(row.id);
            let root_bytes = id(row.root);
            let first_bytes = id(row.first);
            let last_bytes = id(row.last);
            let key = std::str::from_utf8(&key_bytes).map_err(|_| "array source key encoding")?;
            let root = std::str::from_utf8(&root_bytes).map_err(|_| "array source key encoding")?;
            let first =
                std::str::from_utf8(&first_bytes).map_err(|_| "array source key encoding")?;
            let last = std::str::from_utf8(&last_bytes).map_err(|_| "array source key encoding")?;
            let e = entry(log, key, &mut w)?;
            let o = e.origin.as_ref().ok_or("array source node origin")?;
            w.text(&o.source)?;
            w.text(&o.location)?;
            w.take(add(1, o.meta.len())?)?;
            if o.source != row.kind.source()
                || o.location != row.kind.location()
                || o.timestamp.is_some()
                || o.meta.len() != 1
            {
                return Err("array source owned origin".into());
            }
            let m = object(
                o.meta.get(NS).ok_or("array source node namespace")?,
                &[
                    "schema",
                    "kind",
                    "input_id",
                    "first_piece_id",
                    "last_piece_id",
                    "start",
                    "end",
                ],
                &mut w,
            )?;
            for (field, expected) in [
                ("schema", "1"),
                ("kind", row.kind.name()),
                ("input_id", root),
                ("first_piece_id", first),
                ("last_piece_id", last),
            ] {
                let actual = m[field].as_str().ok_or("array source node field type")?;
                w.text(actual)?;
                if actual != expected {
                    return Err("array source node field disagreement".into());
                }
            }
            let pieces = inventory(log, root, &mut w)?;
            if row.first_index >= row.last_index || row.last_index >= pieces.len() {
                return Err("array source receipt interval".into());
            }
            let count = add(row.last_index - row.first_index, 1)?;
            w.take(count)?;
            if e.parent_ids.len() != count
                || pieces[row.first_index].as_str() != Some(first)
                || pieces[row.last_index].as_str() != Some(last)
            {
                return Err("array source ordered boundary parents".into());
            }
            for (actual, p) in e
                .parent_ids
                .iter()
                .zip(&pieces[row.first_index..=row.last_index])
            {
                let expected = p.as_str().ok_or("array source inventory identity")?;
                compact(expected, &mut w)?;
                w.text(actual)?;
                if actual != expected {
                    return Err("array source parent order".into());
                }
            }
            let a = source_piece(log, first, root, &mut w)?;
            let b = source_piece(log, last, root, &mut w)?;
            if a.get("token_type").and_then(Value::as_str)
                != Some(super::source_inventory::token_name(row.first_kind))
                || b.get("token_type").and_then(Value::as_str)
                    != Some(super::source_inventory::token_name(row.last_kind))
            {
                return Err("array source replay boundary kind".into());
            }
            let expected = SourceRange {
                start: position(&a["start"], &mut w)?,
                end: position(&b["end"], &mut w)?,
            };
            let actual = SourceRange {
                start: position(&m["start"], &mut w)?,
                end: position(&m["end"], &mut w)?,
            };
            if actual != expected {
                return Err("array source shared range disagreement".into());
            }
            let journal = log.journal().ok_or("array source missing journal")?;
            let index = usize::try_from(
                row.merge_sequence
                    .checked_sub(1)
                    .ok_or("array source merge zero")?,
            )
            .map_err(|_| "array source merge index")?;
            let record = journal
                .events()
                .get(index)
                .ok_or("array source missing merge")?;
            w.take(1)?;
            if record.sequence() != row.merge_sequence
                || record.context().is_some()
                || !matches!(record.event(),JournalEvent::Merge{entity} if entity.sequence()==row.id)
            {
                return Err("array source merge chronology disagreement".into());
            }
        }
        let mut owned = 0usize;
        for (key, e) in log.entries() {
            w.take(1)?;
            let sequence = compact(key, &mut w)?;
            let mut reserved = false;
            let mut tagged = false;
            if let Some(o) = &e.origin {
                w.text(&o.source)?;
                w.take(add(1, o.meta.len())?)?;
                reserved = o.meta.contains_key(NS);
                tagged = matches!(o.source.as_str(), "typed_array" | "typed_member");
            }
            if reserved || tagged {
                // Charge each actual numeric comparison. A guessed floor-log
                // allowance underpays std's fixed-step search for e.g. 3 rows.
                let (mut low, mut high) = (0, self.receipts.len());
                let mut found = false;
                while low < high {
                    w.take(1)?;
                    let middle = low + (high - low) / 2;
                    match self.receipts[middle].id.cmp(&sequence) {
                        std::cmp::Ordering::Less => low = middle + 1,
                        std::cmp::Ordering::Greater => high = middle,
                        std::cmp::Ordering::Equal => {
                            found = true;
                            break;
                        }
                    }
                }
                if !found {
                    return Err("array source unreceipted node".into());
                }
                owned = add(owned, 1)?;
            }
            for c in &e.contributions {
                w.take(add(1, c.meta.len())?)?;
                if c.meta.contains_key(NS) {
                    return Err("array source reserved contribution placement".into());
                }
            }
            if let Some(d) = &e.deleted {
                w.take(add(1, d.meta.len())?)?;
                if d.meta.contains_key(NS) {
                    return Err("array source reserved deletion placement".into());
                }
            }
        }
        if owned != self.receipts.len() {
            return Err("array source receipt census".into());
        }
        Ok(w.used)
    }
}
pub(super) struct Observer<'a> {
    ledger: &'a mut ArrayLedger,
    log: &'a mut CVLog,
    root: &'a str,
    root_sequence: u64,
}
// The common creator buys the whole physical interval, not an AST frontier.
// Array and member policies choose boundaries; their storage/work/graph gates
// and final source replay remain one closed publication domain.
impl Observer<'_> {
    fn construct(
        &mut self,
        open: &Token,
        close: &Token,
        kind: NodeKind,
        w: &mut Work,
    ) -> Result<Option<String>, String> {
        for token in [open, close] {
            w.take(1)?;
            if matches!(
                token.type_,
                TokenType::Eof | TokenType::Newline | TokenType::Indent | TokenType::Dedent
            ) {
                return Err("member source nonphysical boundary kind".into());
            }
        }
        let (first, first_index, a) = boundary(
            self.log,
            open,
            self.root,
            super::source_inventory::token_name(open.type_),
            w,
        )?;
        let (last, last_index, b) = boundary(
            self.log,
            close,
            self.root,
            super::source_inventory::token_name(close.type_),
            w,
        )?;
        if first >= last || first_index >= last_index || a.end.byte_offset > b.start.byte_offset {
            return Err("array source boundary order".into());
        }
        let pieces = inventory(self.log, self.root, w)?;
        if last_index >= pieces.len()
            || pieces[first_index].as_str() != open.cv.as_deref()
            || pieces[last_index].as_str() != close.cv.as_deref()
        {
            return Err("array source boundary inventory disagreement".into());
        }
        let count = add(last_index - first_index, 1)?;
        w.take(count)?;
        let metadata = add(self.ledger.metadata, META_BYTES)?;
        if metadata > self.ledger.limits.metadata_bytes {
            return Err("array source metadata limit".into());
        }
        w.take(META_BYTES)?;
        self.ledger.reserve_receipt(w)?;
        // Simultaneous owned String rows (24), each ID allocation (<=64),
        // borrowed reference rows (8), fixed metadata and current receipt capacity.
        let per_parent = add(add(size_of::<String>(), 64)?, size_of::<&str>())?;
        let scratch = add(
            add(mul(count, per_parent)?, META_BYTES)?,
            mul(self.ledger.receipts.capacity(), size_of::<Receipt>())?,
        )?;
        if scratch > self.ledger.limits.buffer_bytes {
            return Err("array source buffer limit".into());
        }
        w.take(scratch)?;
        let mut parents = Vec::new();
        parents
            .try_reserve_exact(count)
            .map_err(|_| "array source parent allocation")?;
        if parents.capacity() > count {
            return Err("array source parent allocator capacity".into());
        }
        for p in &pieces[first_index..=last_index] {
            let key = p.as_str().ok_or("array source piece identity type")?;
            source_piece(self.log, key, self.root, w)?;
            parents.push(copy_id(key, w)?);
        }
        let mut refs = Vec::new();
        refs.try_reserve_exact(count)
            .map_err(|_| "array source reference allocation")?;
        if refs.capacity() > count {
            return Err("array source reference allocator capacity".into());
        }
        for p in &parents {
            refs.push(p.as_str());
        }
        let range = SourceRange {
            start: a.start,
            end: b.end,
        };
        let meta = json!({"schema":"1","kind":kind.name(),"input_id":self.root,"first_piece_id":open.cv.as_deref().unwrap(),"last_piece_id":close.cv.as_deref().unwrap(),"start":super::source_inventory::position(range.start),"end":super::source_inventory::position(range.end)});
        let key = self.log.try_merge(
            &refs,
            Some(Origin {
                source: kind.source().into(),
                location: kind.location().into(),
                timestamp: None,
                meta: HashMap::from([(NS.into(), meta)]),
            }),
        )?;
        let sequence = compact(&key, w)?;
        let merge_sequence = self
            .log
            .journal()
            .ok_or("array source missing construction journal")?
            .last_sequence();
        self.ledger.receipts.push(Receipt {
            kind,
            first_kind: open.type_,
            last_kind: close.type_,
            id: sequence,
            root: self.root_sequence,
            first,
            last,
            first_index,
            last_index,
            merge_sequence,
        });
        self.ledger.used_work = w.used;
        self.ledger.metadata = metadata;
        Ok(Some(key))
    }
}
impl ArrayOccurrenceObserver for Observer<'_> {
    fn array_constructed(&mut self, node: &GrammarASTNode) -> Result<Option<String>, String> {
        let mut w = Work {
            used: self.ledger.used_work,
            limit: self.ledger.limits.work,
        };
        w.take(add(1, node.children.len())?)?;
        w.text(&node.rule_name)?;
        if node.rule_name != "array_literal" || !(2..=3).contains(&node.children.len()) {
            return Err("array source direct grammar shape".into());
        }
        let (Some(ASTNodeOrToken::Token(open)), Some(ASTNodeOrToken::Token(close))) =
            (node.children.first(), node.children.last())
        else {
            return Err("array source direct brackets".into());
        };
        if open.type_ != TokenType::LBracket || close.type_ != TokenType::RBracket {
            return Err("array source direct bracket kinds".into());
        }
        self.construct(open, close, NodeKind::Array, &mut w)
    }
}
/// Only one indexed suffix and one leftmost spine are inspected; no source
/// spelling search, recursive traversal, registry or prefix-sized scratch exists.
fn direct_token<'a>(
    node: &'a GrammarASTNode,
    index: usize,
    w: &mut Work,
) -> Result<&'a Token, String> {
    w.take(1)?;
    match node.children.get(index) {
        Some(ASTNodeOrToken::Token(t)) => Ok(t),
        _ => Err("member source direct suffix token".into()),
    }
}
impl CompositeOccurrenceObserver for Observer<'_> {
    fn member_constructed(
        &mut self,
        node: &GrammarASTNode,
        end: usize,
        computed: bool,
        optional: bool,
    ) -> Result<Option<String>, String> {
        let mut w = Work {
            used: self.ledger.used_work,
            limit: self.ledger.limits.work,
        };
        w.take(add(1, node.children.len())?)?;
        w.text(&node.rule_name)?;
        if !matches!(
            node.rule_name.as_str(),
            "member_expression" | "call_expression" | "optional_chain_expression"
        ) {
            return Err("member source owner rule".into());
        }
        let close = direct_token(node, end, &mut w)?;
        let marker_index = if computed {
            w.text(&close.value)?;
            if close.type_ != TokenType::RBracket || close.value != "]" {
                return Err("member source closing bracket".into());
            }
            let key_index = end.checked_sub(1).ok_or("member source missing key")?;
            w.take(1)?;
            if !matches!(node.children.get(key_index), Some(ASTNodeOrToken::Node(_))) {
                return Err("member source direct key".into());
            }
            let bracket = end.checked_sub(2).ok_or("member source missing bracket")?;
            let open = direct_token(node, bracket, &mut w)?;
            w.text(&open.value)?;
            if open.type_ != TokenType::LBracket || open.value != "[" {
                return Err("member source opening bracket".into());
            }
            bracket.checked_sub(1)
        } else {
            if !matches!(close.type_, TokenType::Name | TokenType::Keyword) {
                return Err("member source property kind".into());
            }
            Some(end.checked_sub(1).ok_or("member source missing dot")?)
        };
        w.take(1)?;
        let marker = marker_index.and_then(|index| node.children.get(index));
        let mut is_optional = false;
        let mut is_dot = false;
        if let Some(ASTNodeOrToken::Token(t)) = marker {
            w.text(&t.value)?;
            is_optional = t.value == "?.";
            is_dot = t.type_ == TokenType::Dot && t.value == ".";
            if is_optional {
                // Standard grammar punctuation uses the generic Name fallback
                // plus its precise token name. Buy the scan before comparing.
                w.take(1)?;
                if let Some(name) = &t.type_name {
                    w.text(name)?;
                }
                if t.type_ != TokenType::Name || t.type_name.as_deref() != Some("OPTIONAL_CHAIN") {
                    return Err("member source optional marker kind/name".into());
                }
            }
        }
        if is_optional != optional || (!computed && !optional && !is_dot) {
            return Err("member source optional/dot suffix disagreement".into());
        }
        // The first source terminal is reached by descending ONLY child zero.
        // Pay before every inspection; an empty spine is a refusal, not a search.
        let mut children = node.children.as_slice();
        let open = loop {
            w.take(1)?;
            match children.first() {
                Some(ASTNodeOrToken::Token(t)) => break t,
                Some(ASTNodeOrToken::Node(n)) => children = n.children.as_slice(),
                None => return Err("member source empty first spine".into()),
            }
        };
        let kind = if optional {
            NodeKind::OptionalMember
        } else {
            NodeKind::Member
        };
        self.construct(open, close, kind, &mut w)
    }
}
#[cfg(test)]
#[path = "array_source_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "member_source_tests.rs"]
mod member_tests;
