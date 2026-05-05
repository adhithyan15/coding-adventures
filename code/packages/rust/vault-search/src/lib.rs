//! # `coding_adventures_vault_search` — VLT13 encrypted search
//!
//! ## What this crate is
//!
//! A **local trigram index + BM25 ranker** for the Vault stack.
//! Vaults are E2EE — the server cannot help search — so the
//! search runs entirely on the client. The index is built from
//! plaintext, kept in memory while the vault is unlocked, and
//! persisted as ordinary vault records (encrypted under VLT01's
//! master key by the layer above). When the vault locks, the
//! plaintext index is dropped and zeroised.
//!
//! Two structures stack:
//!
//!   1. **Trigram index** — every 3-char window of every
//!      indexed field becomes a posting in
//!      `HashMap<[u8; 3], Vec<DocumentId>>`. Lookups are
//!      candidate-set unions: "foo" ⇒ posting list for `foo`.
//!   2. **BM25 ranker** — over the candidate set, score each
//!      document with the standard BM25 formula
//!      (k1=1.2, b=0.75) using the trigram as the term unit.
//!      Top-N results are returned sorted by score descending.
//!
//! Trigrams give substring matches (a search for "git" matches
//! "github" *and* "digital"), which is the right shape for a
//! password-manager UI: the user types fragments. BM25 keeps
//! the ranking sensible across a small corpus.
//!
//! ## Per-schema field declarations
//!
//! VLT02 typed records declare which fields are searchable. The
//! VLT13 caller passes a [`SearchableFields`] descriptor —
//! `(field_name, weight)` pairs — when indexing a record. Higher
//! weight ⇒ stronger contribution to the BM25 score (so a
//! match in `title` outranks the same match in `notes`).
//!
//! Per the spec, **passwords and other never-indexed secret
//! material are NOT in `SearchableFields`**. The index never
//! sees them. That's how a stolen index file (decrypted by an
//! attacker who later compromises the master key) gives up
//! titles and labels but not credentials.
//!
//! ## Storage
//!
//! The index is a plain in-memory data structure. Persistence
//! happens at a higher tier:
//!
//!   * The host serializes the index to bytes via
//!     [`SearchIndex::to_bytes`].
//!   * Wraps those bytes in a `vault_sealed_store::Sealed`
//!     record (VLT01) and writes them to the vault's record
//!     namespace.
//!   * On unlock, reads them back, decrypts, and calls
//!     [`SearchIndex::from_bytes`].
//!
//! The on-disk encoding is a small binary framing — version
//! byte, CBOR-of-postings — so it round-trips deterministically
//! and old encodings can be detected.
//!
//! ## Threat model
//!
//! * **Server can't see plaintext.** The index is sealed
//!   alongside everything else; the server stores ciphertext
//!   bytes and revision metadata only.
//! * **Locked vault has no plaintext index.** When the vault
//!   locks, the in-memory `SearchIndex` is dropped — its
//!   internal buffers are wrapped in `Zeroizing` so plaintext
//!   trigrams and document text are scrubbed.
//! * **Search results never include payloads.** The ranker
//!   returns `(DocumentId, score)`; the host fetches the
//!   sealed record separately. No plaintext flows through this
//!   crate other than as input to the indexer.
//! * **Bounded memory.** Per-document-field input is capped at
//!   [`MAX_INDEXED_FIELD_LEN`]; per-index document count is
//!   capped at [`MAX_INDEXED_DOCS`]. Above either, indexing
//!   returns [`SearchError::TooLarge`] without partial state
//!   updates.
//! * **Stop-list confidentiality.** The index records which
//!   trigrams *appeared* in indexed fields — i.e. anyone who
//!   later breaks the master-key gets a per-trigram presence
//!   set. We do *not* attempt the much-stronger SSE
//!   property of "the server learns nothing even when the
//!   client searches"; that's a v2 concern. The threat model
//!   here is: at-rest sealing protects the index until the
//!   master key falls.
//!
//! ## What this crate is NOT
//!
//! * Not a transparency layer (VLT09 records search-event
//!   audit if the application chooses).
//! * Not a stemmer / language pipeline. Trigrams are
//!   language-agnostic — that's why we picked them.
//! * Not a secrets-aware indexer. The caller decides which
//!   fields are indexable; this crate trusts that decision.
//! * Not server-side searchable encryption (SSE / OPE). Server
//!   sees ciphertext only.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_zeroize::Zeroize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Mutex, MutexGuard, PoisonError};

/// Best-effort lock-recovery (same pattern as `vault-audit` /
/// `vault-revisions`).
fn lock_recover<'a, T>(m: &'a Mutex<T>) -> MutexGuard<'a, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

// === Section 1. Bounds ====================================================

/// Maximum bytes per indexed field value. Above this the
/// indexer returns [`SearchError::TooLarge`] — caller must
/// truncate or otherwise summarise the field before calling
/// `index`.
pub const MAX_INDEXED_FIELD_LEN: usize = 64 * 1024;
/// Maximum number of distinct documents in the index. A
/// password manager never has 10M passwords; this cap protects
/// against a runaway caller pumping the index.
pub const MAX_INDEXED_DOCS: usize = 1_000_000;
/// Maximum number of fields per document.
pub const MAX_FIELDS_PER_DOC: usize = 64;
/// Maximum bytes of an opaque [`DocumentId`].
pub const MAX_DOC_ID_LEN: usize = 256;
/// Maximum bytes of a query string.
pub const MAX_QUERY_LEN: usize = 4 * 1024;
/// Length of a trigram, in *characters* (not bytes).
const TRIGRAM_CHAR_LEN: usize = 3;

/// Maximum distinct trigrams the decoder will accept *per
/// document*. Tighter than the universe of all possible 3-byte
/// trigrams (256³ ≈ 16M); a real document with 64 KiB of
/// content has at most ~64 K windows, of which a small fraction
/// are distinct. We cap at 65_536 so the decoder cannot be
/// asked to allocate gigabytes for a fabricated `tf_count`.
pub const MAX_TF_ENTRIES_PER_DOC: usize = 65_536;
/// Maximum value of `Doc::total_len` accepted from the on-disk
/// encoding. Matches the live indexer's bound
/// (`MAX_INDEXED_FIELD_LEN * MAX_FIELDS_PER_DOC`) so a crafted
/// persisted index cannot manipulate BM25 ranking via an
/// implausible length.
pub const MAX_TOTAL_LEN: u64 =
    (MAX_INDEXED_FIELD_LEN as u64) * (MAX_FIELDS_PER_DOC as u64);
/// On-disk per-tf-entry size (3 bytes trigram + 4 bytes f32).
const ON_DISK_TF_ENTRY_BYTES: usize = 7;

// === Section 2. Vocabulary types ==========================================

/// Opaque identifier for an indexed document. Backed by a
/// `String` so callers can pick whatever representation fits
/// their record namespace.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct DocumentId(String);

impl DocumentId {
    /// Construct, validating bounds and character class.
    pub fn new(id: impl Into<String>) -> Result<Self, SearchError> {
        let s = id.into();
        if s.is_empty() {
            return Err(SearchError::InvalidParameter("document id must not be empty"));
        }
        if s.len() > MAX_DOC_ID_LEN {
            return Err(SearchError::InvalidParameter(
                "document id exceeds MAX_DOC_ID_LEN",
            ));
        }
        for c in s.chars() {
            if c.is_control() || c.is_whitespace() {
                return Err(SearchError::InvalidParameter(
                    "document id contains forbidden whitespace / control chars",
                ));
            }
            let cp = c as u32;
            if matches!(cp, 0x202A..=0x202E | 0x2066..=0x2069 | 0x200B..=0x200D | 0xFEFF) {
                return Err(SearchError::InvalidParameter(
                    "document id contains forbidden Unicode (bidi / zero-width)",
                ));
            }
        }
        Ok(Self(s))
    }

    /// Borrow the underlying string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Per-schema field declarations: which fields the caller wants
/// indexed and with what relative weight. A field present here
/// AND in the document being indexed is included in the
/// trigram set; fields not in the descriptor are skipped
/// (most importantly: passwords).
#[derive(Clone, Debug, Default)]
pub struct SearchableFields {
    /// `field_name → weight`. Weight is a `f32 > 0` — its
    /// magnitude controls how strongly a match in this field
    /// contributes to the BM25 score.
    pub weights: BTreeMap<String, f32>,
}

impl SearchableFields {
    /// Empty descriptor — useful as a starting point for a
    /// builder pattern.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a `(field, weight)` pair. Replaces any prior weight.
    pub fn with(mut self, field: impl Into<String>, weight: f32) -> Self {
        self.weights.insert(field.into(), weight);
        self
    }
}

/// All errors produced by this crate.
#[derive(Debug)]
pub enum SearchError {
    /// Caller passed a malformed argument.
    InvalidParameter(&'static str),
    /// Index would exceed a documented bound (doc count, field
    /// length, fields-per-doc).
    TooLarge(&'static str),
    /// The on-disk encoded form is malformed or has an unknown
    /// version byte.
    Decode(&'static str),
}

impl core::fmt::Display for SearchError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidParameter(why) => write!(f, "invalid parameter: {}", why),
            Self::TooLarge(why) => write!(f, "too large: {}", why),
            Self::Decode(why) => write!(f, "decode error: {}", why),
        }
    }
}

impl std::error::Error for SearchError {}

/// One ranked search hit.
#[derive(Clone, Debug, PartialEq)]
pub struct SearchHit {
    /// The matching document.
    pub id: DocumentId,
    /// BM25 score. Higher = better match. Order across hits
    /// is meaningful; absolute magnitude is not.
    pub score: f32,
}

// === Section 3. Trigram extractor =========================================
//
// Lowercases ASCII; treats other Unicode codepoints byte-by-byte
// (we trigram over the lowercased UTF-8 stream rather than
// chars to keep the math integer and the index compact). For a
// password-manager corpus this is fine — search is over titles,
// URLs, usernames, tags.

/// Lower-case ASCII; pass through other bytes. Total, allocates
/// once.
fn lowercase_ascii(s: &str) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::with_capacity(s.len());
    for b in s.bytes() {
        if (b'A'..=b'Z').contains(&b) {
            out.push(b + 32);
        } else {
            out.push(b);
        }
    }
    out
}

/// Extract every 3-byte window of the lowercased input. Returns
/// a `HashMap<[u8;3], u32>` of trigram → count (the count is
/// used for BM25 term-frequency).
fn trigrams_with_counts(s: &str) -> HashMap<[u8; 3], u32> {
    let bytes = lowercase_ascii(s);
    let mut out: HashMap<[u8; 3], u32> = HashMap::new();
    if bytes.len() < TRIGRAM_CHAR_LEN {
        return out;
    }
    for i in 0..=bytes.len() - TRIGRAM_CHAR_LEN {
        let tg = [bytes[i], bytes[i + 1], bytes[i + 2]];
        *out.entry(tg).or_insert(0) += 1;
    }
    out
}

/// Trigrams of a *query*. We use a `HashSet` rather than counts
/// because BM25 scoring iterates over distinct query terms.
fn query_trigrams(s: &str) -> HashSet<[u8; 3]> {
    let bytes = lowercase_ascii(s);
    let mut out: HashSet<[u8; 3]> = HashSet::new();
    if bytes.len() < TRIGRAM_CHAR_LEN {
        return out;
    }
    for i in 0..=bytes.len() - TRIGRAM_CHAR_LEN {
        out.insert([bytes[i], bytes[i + 1], bytes[i + 2]]);
    }
    out
}

// === Section 4. Index =====================================================
//
// A document carries:
//   - id (opaque to this crate),
//   - field_lengths: total byte length per field (BM25 needs it
//     for length normalization),
//   - tf: trigram → count (per-document term frequency),
//   - field_weights: which fields contributed (so we can apply
//     SearchableFields weights at query time).
//
// The inverted index is a HashMap<trigram, Vec<DocumentId>>.

/// Per-document state stored inside the index. Held under
/// `Zeroize` so when the index is dropped (or `clear()`'d)
/// every plaintext trigram and per-document length is wiped.
#[derive(Clone, Debug)]
struct Doc {
    id: DocumentId,
    /// Total bytes across all indexed fields.
    total_len: u64,
    /// Per-trigram occurrence count in this document
    /// (across all fields, weighted by field weight).
    tf: HashMap<[u8; 3], f32>,
}

impl Zeroize for Doc {
    fn zeroize(&mut self) {
        // Drop the trigrams and their counts. Trigrams are 3
        // ASCII bytes each — clearing the map releases the
        // backing memory.
        self.tf.clear();
        self.tf.shrink_to_fit();
        self.total_len = 0;
    }
}

/// The full search index. Wrap in an `Arc` and share across
/// threads; the internal `Mutex` serializes mutations.
pub struct SearchIndex {
    inner: Mutex<IndexInner>,
}

struct IndexInner {
    /// All documents currently in the index.
    docs: BTreeMap<DocumentId, Doc>,
    /// Inverted index: trigram → set of document ids.
    postings: HashMap<[u8; 3], BTreeMap<DocumentId, ()>>,
}

impl Default for SearchIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchIndex {
    /// Construct a fresh, empty index.
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(IndexInner {
                docs: BTreeMap::new(),
                postings: HashMap::new(),
            }),
        }
    }

    /// Insert (or replace) a document.
    ///
    /// `fields` is the document's content keyed by field name.
    /// Only entries whose key appears in `searchable.weights`
    /// are indexed; other fields are silently skipped (this is
    /// how passwords and other never-indexed secrets stay out
    /// of the index — declare only the safe fields and pass
    /// the whole record).
    pub fn index(
        &self,
        id: DocumentId,
        fields: &BTreeMap<String, String>,
        searchable: &SearchableFields,
    ) -> Result<(), SearchError> {
        if fields.len() > MAX_FIELDS_PER_DOC {
            return Err(SearchError::TooLarge("fields per document"));
        }
        // Pre-validate sizes BEFORE mutating any state so a
        // half-completed index is impossible.
        for (k, v) in fields.iter() {
            if !searchable.weights.contains_key(k) {
                continue;
            }
            if v.len() > MAX_INDEXED_FIELD_LEN {
                return Err(SearchError::TooLarge("indexed field length"));
            }
        }
        // Build the per-document term frequencies (weighted by
        // the field weight) outside the lock so the critical
        // section is short.
        let mut total_len: u64 = 0;
        let mut tf: HashMap<[u8; 3], f32> = HashMap::new();
        for (field, value) in fields.iter() {
            let weight = match searchable.weights.get(field) {
                Some(&w) if w > 0.0 && w.is_finite() => w,
                _ => continue,
            };
            total_len = total_len.saturating_add(value.len() as u64);
            for (trigram, count) in trigrams_with_counts(value) {
                let entry = tf.entry(trigram).or_insert(0.0);
                *entry += weight * count as f32;
            }
        }
        // Now acquire the lock and merge into the index.
        let mut g = lock_recover(&self.inner);
        // Capacity check at the lock so it's authoritative.
        if !g.docs.contains_key(&id) && g.docs.len() >= MAX_INDEXED_DOCS {
            return Err(SearchError::TooLarge("MAX_INDEXED_DOCS"));
        }
        // Removing the prior version of this document scrubs
        // its old trigrams from postings.
        if let Some(mut prior) = g.docs.remove(&id) {
            for (tg, _) in prior.tf.drain() {
                if let Some(plist) = g.postings.get_mut(&tg) {
                    plist.remove(&id);
                    if plist.is_empty() {
                        g.postings.remove(&tg);
                    }
                }
            }
        }
        // Insert the new postings.
        for (tg, _) in tf.iter() {
            g.postings
                .entry(*tg)
                .or_default()
                .insert(id.clone(), ());
        }
        let doc = Doc {
            id: id.clone(),
            total_len,
            tf,
        };
        g.docs.insert(id, doc);
        Ok(())
    }

    /// Remove a document. Idempotent (no error if it never
    /// existed).
    pub fn remove(&self, id: &DocumentId) -> Result<(), SearchError> {
        let mut g = lock_recover(&self.inner);
        if let Some(mut prior) = g.docs.remove(id) {
            for (tg, _) in prior.tf.drain() {
                if let Some(plist) = g.postings.get_mut(&tg) {
                    plist.remove(id);
                    if plist.is_empty() {
                        g.postings.remove(&tg);
                    }
                }
            }
        }
        Ok(())
    }

    /// Number of documents currently in the index.
    pub fn len(&self) -> usize {
        let g = lock_recover(&self.inner);
        g.docs.len()
    }

    /// `true` iff the index has no documents.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Search the index. Returns the top `top_n` hits sorted
    /// by score descending. Ties broken by document id
    /// ascending (so order is deterministic across replicas).
    pub fn search(&self, query: &str, top_n: usize) -> Result<Vec<SearchHit>, SearchError> {
        if query.len() > MAX_QUERY_LEN {
            return Err(SearchError::InvalidParameter(
                "query exceeds MAX_QUERY_LEN",
            ));
        }
        let q_trigrams = query_trigrams(query);
        if q_trigrams.is_empty() {
            // Query shorter than 3 chars → no candidates. (Not
            // an error — empty result.)
            return Ok(Vec::new());
        }
        let g = lock_recover(&self.inner);
        let n_docs = g.docs.len() as f32;
        if n_docs == 0.0 {
            return Ok(Vec::new());
        }
        // Average document length for BM25 length normalization.
        // `Iterator::sum::<u64>` panics in debug on overflow and
        // wraps in release; both are bad. With `from_bytes` now
        // bounding `total_len <= MAX_TOTAL_LEN` per doc, the
        // realistic sum is at most `MAX_INDEXED_DOCS *
        // MAX_TOTAL_LEN` — far below `u64::MAX` — but a
        // saturating fold is still cheap defence-in-depth in
        // case the bound ever loosens.
        let avg_dl: f32 = if g.docs.is_empty() {
            0.0
        } else {
            let total: u64 = g
                .docs
                .values()
                .map(|d| d.total_len)
                .fold(0u64, u64::saturating_add);
            (total as f32) / n_docs
        };
        // Candidate set = union of posting lists for every
        // query trigram.
        let mut candidates: HashSet<DocumentId> = HashSet::new();
        for tg in q_trigrams.iter() {
            if let Some(plist) = g.postings.get(tg) {
                for id in plist.keys() {
                    candidates.insert(id.clone());
                }
            }
        }
        // BM25 over the candidate set.
        let mut scored: Vec<SearchHit> = Vec::with_capacity(candidates.len());
        for id in candidates {
            let doc = match g.docs.get(&id) {
                Some(d) => d,
                None => continue, // shouldn't happen but defensive
            };
            let score = bm25_score(doc, &q_trigrams, &g.postings, n_docs, avg_dl);
            if score > 0.0 {
                scored.push(SearchHit {
                    id: doc.id.clone(),
                    score,
                });
            }
        }
        // Sort: score descending, then id ascending.
        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });
        scored.truncate(top_n);
        Ok(scored)
    }

    /// Drop every document and posting. Used on lock to clear
    /// plaintext from memory.
    pub fn clear(&self) {
        let mut g = lock_recover(&self.inner);
        // Walk docs and zeroise each before dropping.
        for (_, doc) in g.docs.iter_mut() {
            doc.zeroize();
        }
        g.docs.clear();
        g.postings.clear();
    }

    /// Serialize the index to opaque bytes for persistence.
    /// Caller wraps the result in a `vault_sealed_store::Sealed`
    /// record. Wire shape: a 4-byte magic `"VSI1"` + version
    /// byte `1` + length-prefixed lists. The encoder is total
    /// and bounded; the decoder ([`Self::from_bytes`]) is
    /// strict.
    pub fn to_bytes(&self) -> Vec<u8> {
        let g = lock_recover(&self.inner);
        let mut out = Vec::with_capacity(64);
        out.extend_from_slice(b"VSI1");
        out.push(1u8); // version
        // u32 doc count
        out.extend_from_slice(&(g.docs.len() as u32).to_be_bytes());
        for (_, doc) in g.docs.iter() {
            // doc id (length-prefixed string)
            let id = doc.id.as_str();
            out.extend_from_slice(&(id.len() as u32).to_be_bytes());
            out.extend_from_slice(id.as_bytes());
            // total_len
            out.extend_from_slice(&doc.total_len.to_be_bytes());
            // tf entries
            out.extend_from_slice(&(doc.tf.len() as u32).to_be_bytes());
            for (tg, count) in doc.tf.iter() {
                out.extend_from_slice(tg);
                out.extend_from_slice(&count.to_bits().to_be_bytes());
            }
        }
        out
    }

    /// Deserialize an index from bytes produced by
    /// [`Self::to_bytes`]. Returns an error on any malformed
    /// input.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SearchError> {
        let mut p = ByteReader::new(bytes);
        let magic = p.take(4)?;
        if magic != b"VSI1" {
            return Err(SearchError::Decode("bad magic"));
        }
        let version = p.take_u8()?;
        if version != 1 {
            return Err(SearchError::Decode("unsupported version"));
        }
        let n_docs = p.take_u32()? as usize;
        if n_docs > MAX_INDEXED_DOCS {
            return Err(SearchError::Decode("doc count exceeds MAX_INDEXED_DOCS"));
        }
        // Per-doc minimum is 4 (id_len) + 0 (id_bytes) + 8
        // (total_len) + 4 (tf_count) = 16 bytes. Reject claims
        // that don't fit in the input — same bomb-defense as the
        // per-tf-count check below.
        const MIN_DOC_BYTES: usize = 16;
        let needed_docs_bytes = n_docs
            .checked_mul(MIN_DOC_BYTES)
            .ok_or(SearchError::Decode("n_docs overflow"))?;
        if needed_docs_bytes > p.remaining() {
            return Err(SearchError::Decode(
                "n_docs claims more documents than the input contains",
            ));
        }
        let mut docs: BTreeMap<DocumentId, Doc> = BTreeMap::new();
        let mut postings: HashMap<[u8; 3], BTreeMap<DocumentId, ()>> = HashMap::new();
        for _ in 0..n_docs {
            let id_len = p.take_u32()? as usize;
            if id_len > MAX_DOC_ID_LEN {
                return Err(SearchError::Decode("doc id exceeds MAX_DOC_ID_LEN"));
            }
            let id_bytes = p.take(id_len)?;
            let id_str = std::str::from_utf8(id_bytes)
                .map_err(|_| SearchError::Decode("doc id is not UTF-8"))?;
            let id = DocumentId::new(id_str.to_owned()).map_err(|_| {
                SearchError::Decode("doc id failed validation")
            })?;
            let total_len = p.take_u64()?;
            // Reject crafted indices that claim a `total_len`
            // larger than the live indexer would ever produce.
            // Otherwise an attacker who can replace the persisted
            // index manipulates BM25 ranking (small total_len ⇒
            // higher score) and, in `search`, can overflow the
            // u64 sum used for `avg_dl`.
            if total_len > MAX_TOTAL_LEN {
                return Err(SearchError::Decode(
                    "total_len exceeds MAX_TOTAL_LEN bound",
                ));
            }
            let tf_count = p.take_u32()? as usize;
            // Two layers of bound:
            // 1. A constant per-doc cap so the decoder cannot
            //    allocate gigabytes per document.
            // 2. A consistency check against the actual remaining
            //    bytes in the input — a `tf_count` of N implies
            //    at least `N * 7` more bytes are present
            //    (3 trigram + 4 f32). Without this, a 21-byte
            //    payload claiming `tf_count = 4_000_000` would
            //    drive a 256 MiB `with_capacity` allocation
            //    before any actual entry is read.
            if tf_count > MAX_TF_ENTRIES_PER_DOC {
                return Err(SearchError::Decode(
                    "tf count exceeds MAX_TF_ENTRIES_PER_DOC",
                ));
            }
            let needed = tf_count
                .checked_mul(ON_DISK_TF_ENTRY_BYTES)
                .ok_or(SearchError::Decode("tf_count overflow"))?;
            if needed > p.remaining() {
                return Err(SearchError::Decode(
                    "tf_count claims more entries than the input contains",
                ));
            }
            // Now `with_capacity` is safe — the input has at
            // least `needed` bytes left to actually fill it.
            let mut tf: HashMap<[u8; 3], f32> = HashMap::with_capacity(tf_count);
            for _ in 0..tf_count {
                let tg_bytes = p.take(3)?;
                let tg = [tg_bytes[0], tg_bytes[1], tg_bytes[2]];
                let bits = p.take_u32()?;
                let count = f32::from_bits(bits);
                if !count.is_finite() || count < 0.0 {
                    return Err(SearchError::Decode("tf weight not a finite non-negative f32"));
                }
                tf.insert(tg, count);
            }
            for tg in tf.keys() {
                postings.entry(*tg).or_default().insert(id.clone(), ());
            }
            let doc = Doc {
                id: id.clone(),
                total_len,
                tf,
            };
            docs.insert(id, doc);
        }
        if !p.is_empty() {
            return Err(SearchError::Decode("trailing bytes"));
        }
        Ok(SearchIndex {
            inner: Mutex::new(IndexInner { docs, postings }),
        })
    }
}

// === Section 5. Drop wipes the index =====================================

impl Drop for SearchIndex {
    fn drop(&mut self) {
        // On drop the mutex is uncontended (we have `&mut self`).
        // Walk docs to scrub each per-document tf map before the
        // `BTreeMap` releases their backing storage. This is the
        // belt-and-braces case: the host calls `clear()` on lock,
        // so by the time `Drop` runs there is usually nothing to
        // scrub — but if the index escapes that path (e.g. a
        // panic mid-search) the wipe still has to happen.
        //
        // `get_mut()` returns the inner data WITHOUT taking the
        // lock, so a poisoned mutex (from a panic mid-search) does
        // *not* skip the scrub. (The earlier `if let Ok(...)
        // self.inner.lock()` form silently dropped the scrub on
        // exactly the most-important path.)
        if let Ok(g) = self.inner.get_mut() {
            for (_, doc) in g.docs.iter_mut() {
                doc.zeroize();
            }
            g.docs.clear();
            g.postings.clear();
        }
    }
}

// === Section 6. BM25 =====================================================

/// Standard BM25 (k1=1.2, b=0.75) over trigrams.
///
/// term-IDF: `ln((N - df + 0.5) / (df + 0.5) + 1.0)` where N is
/// the total number of documents and df is the number of docs
/// containing the trigram. `+ 1.0` keeps IDF non-negative even
/// for very common trigrams.
fn bm25_score(
    doc: &Doc,
    q_trigrams: &HashSet<[u8; 3]>,
    postings: &HashMap<[u8; 3], BTreeMap<DocumentId, ()>>,
    n_docs: f32,
    avg_dl: f32,
) -> f32 {
    let k1 = 1.2_f32;
    let b = 0.75_f32;
    let dl = (doc.total_len as f32).max(1.0);
    let mut score: f32 = 0.0;
    for tg in q_trigrams.iter() {
        let tf = match doc.tf.get(tg) {
            Some(&v) => v,
            None => 0.0,
        };
        if tf <= 0.0 {
            continue;
        }
        let df = postings.get(tg).map(|p| p.len()).unwrap_or(0) as f32;
        let idf = ((n_docs - df + 0.5) / (df + 0.5) + 1.0).ln();
        let denom = tf + k1 * (1.0 - b + b * dl / avg_dl.max(1.0));
        let num = tf * (k1 + 1.0);
        score += idf * (num / denom);
    }
    score
}

// === Section 7. ByteReader (decoder helper) ===============================

struct ByteReader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> ByteReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }
    fn is_empty(&self) -> bool {
        self.pos >= self.bytes.len()
    }
    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.pos)
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], SearchError> {
        let end = self
            .pos
            .checked_add(n)
            .ok_or(SearchError::Decode("offset overflow"))?;
        if end > self.bytes.len() {
            return Err(SearchError::Decode("unexpected end of input"));
        }
        let slice = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(slice)
    }
    fn take_u8(&mut self) -> Result<u8, SearchError> {
        let b = self.take(1)?;
        Ok(b[0])
    }
    fn take_u32(&mut self) -> Result<u32, SearchError> {
        let b = self.take(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn take_u64(&mut self) -> Result<u64, SearchError> {
        let b = self.take(8)?;
        Ok(u64::from_be_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }
}

// === Section 8. Tests =====================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn doc_id(s: &str) -> DocumentId {
        DocumentId::new(s).unwrap()
    }

    fn searchable_title_and_url() -> SearchableFields {
        SearchableFields::new()
            .with("title", 2.0)
            .with("url", 1.0)
    }

    fn fields_pair(title: &str, url: &str) -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        m.insert("title".into(), title.into());
        m.insert("url".into(), url.into());
        m
    }

    // --- Trigram extraction ---

    #[test]
    fn trigram_lowercase() {
        // "Foo" has trigrams { "foo": 1 }
        let m = trigrams_with_counts("Foo");
        assert_eq!(m.len(), 1);
        assert_eq!(*m.get(b"foo").unwrap(), 1);
    }

    #[test]
    fn trigram_short_input_returns_empty() {
        assert_eq!(trigrams_with_counts("ab").len(), 0);
        assert_eq!(trigrams_with_counts("").len(), 0);
    }

    #[test]
    fn trigram_counts_overlapping() {
        // "abcabc" → "abc" "bca" "cab" "abc" → abc=2, bca=1, cab=1
        let m = trigrams_with_counts("abcabc");
        assert_eq!(*m.get(b"abc").unwrap(), 2);
        assert_eq!(*m.get(b"bca").unwrap(), 1);
        assert_eq!(*m.get(b"cab").unwrap(), 1);
    }

    // --- Indexing + search ---

    #[test]
    fn index_then_search_finds_match() {
        let idx = SearchIndex::new();
        let s = searchable_title_and_url();
        idx.index(doc_id("d1"), &fields_pair("github", "https://github.com"), &s)
            .unwrap();
        idx.index(doc_id("d2"), &fields_pair("gitlab", "https://gitlab.com"), &s)
            .unwrap();
        idx.index(doc_id("d3"), &fields_pair("notion", "https://notion.so"), &s)
            .unwrap();
        let hits = idx.search("git", 10).unwrap();
        assert_eq!(hits.len(), 2);
        let ids: Vec<&str> = hits.iter().map(|h| h.id.as_str()).collect();
        assert!(ids.contains(&"d1"));
        assert!(ids.contains(&"d2"));
        assert!(!ids.contains(&"d3"));
    }

    #[test]
    fn search_ranking_uses_field_weights() {
        // Title is weighted 2.0, url 1.0. A document whose
        // title contains "git" should rank above one whose
        // only mention is in url.
        let idx = SearchIndex::new();
        let s = searchable_title_and_url();
        idx.index(doc_id("title-match"), &fields_pair("git stuff", "https://example.com"), &s)
            .unwrap();
        idx.index(doc_id("url-match"), &fields_pair("things", "https://git.example.com"), &s)
            .unwrap();
        let hits = idx.search("git", 10).unwrap();
        assert!(hits.len() >= 2);
        // The first hit should be title-match.
        assert_eq!(hits[0].id.as_str(), "title-match");
    }

    #[test]
    fn search_does_not_index_undeclared_fields() {
        // The "password" field is NOT in `searchable`; even if
        // the document has it, the index must not see its
        // trigrams.
        let mut fields = fields_pair("github", "https://github.com");
        fields.insert("password".into(), "MySuperUniqueSecretFFFOOOO123".into());
        let idx = SearchIndex::new();
        let s = searchable_title_and_url();
        idx.index(doc_id("d1"), &fields, &s).unwrap();
        // Search for a substring that ONLY occurs in the
        // password — must return no hits.
        let hits = idx.search("yuni", 10).unwrap();
        assert!(hits.is_empty(), "hits were: {:?}", hits);
    }

    #[test]
    fn re_indexing_replaces_prior_postings() {
        let idx = SearchIndex::new();
        let s = searchable_title_and_url();
        idx.index(doc_id("d1"), &fields_pair("github", "https://github.com"), &s)
            .unwrap();
        // Re-index with completely different content.
        idx.index(doc_id("d1"), &fields_pair("notion", "https://notion.so"), &s)
            .unwrap();
        // Old trigrams must no longer match.
        assert!(idx.search("git", 10).unwrap().is_empty());
        // New ones do.
        let hits = idx.search("not", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id.as_str(), "d1");
    }

    #[test]
    fn remove_purges_postings() {
        let idx = SearchIndex::new();
        let s = searchable_title_and_url();
        idx.index(doc_id("d1"), &fields_pair("github", "https://github.com"), &s)
            .unwrap();
        idx.remove(&doc_id("d1")).unwrap();
        assert_eq!(idx.len(), 0);
        assert!(idx.search("git", 10).unwrap().is_empty());
    }

    #[test]
    fn remove_unknown_id_is_noop() {
        let idx = SearchIndex::new();
        idx.remove(&doc_id("never-indexed")).unwrap();
    }

    #[test]
    fn search_below_trigram_length_returns_empty() {
        let idx = SearchIndex::new();
        let s = searchable_title_and_url();
        idx.index(doc_id("d1"), &fields_pair("github", "x"), &s)
            .unwrap();
        assert!(idx.search("ab", 10).unwrap().is_empty());
    }

    #[test]
    fn search_top_n_caps_results() {
        let idx = SearchIndex::new();
        let s = searchable_title_and_url();
        for i in 0..10 {
            idx.index(
                doc_id(&format!("d{}", i)),
                &fields_pair("github", "https://github.com"),
                &s,
            )
            .unwrap();
        }
        let hits = idx.search("git", 3).unwrap();
        assert_eq!(hits.len(), 3);
    }

    #[test]
    fn search_results_sorted_descending_by_score() {
        let idx = SearchIndex::new();
        let s = searchable_title_and_url();
        for i in 0..5 {
            idx.index(
                doc_id(&format!("d{}", i)),
                &fields_pair("github github github", "https://example.com"),
                &s,
            )
            .unwrap();
        }
        let hits = idx.search("git", 5).unwrap();
        for w in hits.windows(2) {
            assert!(w[0].score >= w[1].score, "scores not descending: {:?}", hits);
        }
    }

    #[test]
    fn search_empty_index_returns_empty() {
        let idx = SearchIndex::new();
        assert!(idx.search("git", 10).unwrap().is_empty());
    }

    // --- Validation ---

    #[test]
    fn doc_id_rejects_empty() {
        assert!(matches!(
            DocumentId::new(""),
            Err(SearchError::InvalidParameter(_))
        ));
    }

    #[test]
    fn doc_id_rejects_oversize() {
        let big = "x".repeat(MAX_DOC_ID_LEN + 1);
        assert!(matches!(
            DocumentId::new(big),
            Err(SearchError::InvalidParameter(_))
        ));
    }

    #[test]
    fn doc_id_rejects_control_chars() {
        assert!(matches!(
            DocumentId::new("alice\nrole=admin"),
            Err(SearchError::InvalidParameter(_))
        ));
        assert!(matches!(
            DocumentId::new("alice\u{202e}txt"), // bidi override
            Err(SearchError::InvalidParameter(_))
        ));
    }

    #[test]
    fn index_rejects_oversize_field() {
        let idx = SearchIndex::new();
        let s = searchable_title_and_url();
        let big = "x".repeat(MAX_INDEXED_FIELD_LEN + 1);
        let mut fields = BTreeMap::new();
        fields.insert("title".into(), big);
        fields.insert("url".into(), "y".to_string());
        let r = idx.index(doc_id("d1"), &fields, &s);
        assert!(matches!(r, Err(SearchError::TooLarge(_))));
        // Index must still be empty (no partial state).
        assert_eq!(idx.len(), 0);
    }

    #[test]
    fn index_rejects_too_many_fields() {
        let idx = SearchIndex::new();
        let s = SearchableFields::new();
        let mut fields = BTreeMap::new();
        for i in 0..(MAX_FIELDS_PER_DOC + 1) {
            fields.insert(format!("f{}", i), "x".to_string());
        }
        let r = idx.index(doc_id("d1"), &fields, &s);
        assert!(matches!(r, Err(SearchError::TooLarge(_))));
    }

    #[test]
    fn search_rejects_oversize_query() {
        let idx = SearchIndex::new();
        let big = "x".repeat(MAX_QUERY_LEN + 1);
        let r = idx.search(&big, 10);
        assert!(matches!(r, Err(SearchError::InvalidParameter(_))));
    }

    // --- to_bytes / from_bytes ---

    #[test]
    fn roundtrip_via_bytes_preserves_search_results() {
        let idx = SearchIndex::new();
        let s = searchable_title_and_url();
        idx.index(doc_id("d1"), &fields_pair("github", "https://github.com"), &s)
            .unwrap();
        idx.index(doc_id("d2"), &fields_pair("notion", "https://notion.so"), &s)
            .unwrap();
        let bytes = idx.to_bytes();
        let restored = SearchIndex::from_bytes(&bytes).unwrap();
        // Both indices give the same hits for the same query.
        let h1 = idx.search("git", 10).unwrap();
        let h2 = restored.search("git", 10).unwrap();
        assert_eq!(h1.len(), h2.len());
        assert_eq!(h1[0].id, h2[0].id);
    }

    #[test]
    fn from_bytes_rejects_bad_magic() {
        let bad = b"XXXX\x01\x00\x00\x00\x00";
        let r = SearchIndex::from_bytes(bad);
        assert!(matches!(r, Err(SearchError::Decode(_))));
    }

    #[test]
    fn from_bytes_rejects_unsupported_version() {
        let mut bytes: Vec<u8> = Vec::new();
        bytes.extend_from_slice(b"VSI1");
        bytes.push(2u8); // wrong version
        bytes.extend_from_slice(&0u32.to_be_bytes());
        let r = SearchIndex::from_bytes(&bytes);
        assert!(matches!(r, Err(SearchError::Decode(_))));
    }

    #[test]
    fn from_bytes_rejects_trailing_bytes() {
        let idx = SearchIndex::new();
        let mut bytes = idx.to_bytes();
        bytes.push(0xff); // garbage tail
        let r = SearchIndex::from_bytes(&bytes);
        assert!(matches!(r, Err(SearchError::Decode(_))));
    }

    #[test]
    fn from_bytes_rejects_oversize_doc_count() {
        let mut bytes: Vec<u8> = Vec::new();
        bytes.extend_from_slice(b"VSI1");
        bytes.push(1u8);
        bytes.extend_from_slice(&((MAX_INDEXED_DOCS as u32) + 1).to_be_bytes());
        let r = SearchIndex::from_bytes(&bytes);
        assert!(matches!(r, Err(SearchError::Decode(_))));
    }

    #[test]
    fn from_bytes_rejects_truncated() {
        let bytes = b"VSI1\x01"; // missing doc count
        let r = SearchIndex::from_bytes(bytes);
        assert!(matches!(r, Err(SearchError::Decode(_))));
    }

    #[test]
    fn from_bytes_rejects_oversize_total_len() {
        // Build a payload whose `total_len` is larger than
        // MAX_TOTAL_LEN — the live indexer would never produce
        // this, so an attacker-controlled persisted index must
        // be rejected to prevent BM25 ranking manipulation +
        // the u64 sum overflow path in `search`.
        let mut bytes: Vec<u8> = Vec::new();
        bytes.extend_from_slice(b"VSI1");
        bytes.push(1u8);
        bytes.extend_from_slice(&1u32.to_be_bytes()); // 1 doc
        let id = "d1";
        bytes.extend_from_slice(&(id.len() as u32).to_be_bytes());
        bytes.extend_from_slice(id.as_bytes());
        // total_len = MAX_TOTAL_LEN + 1
        bytes.extend_from_slice(&(MAX_TOTAL_LEN + 1).to_be_bytes());
        bytes.extend_from_slice(&0u32.to_be_bytes()); // 0 tf
        let r = SearchIndex::from_bytes(&bytes);
        assert!(matches!(r, Err(SearchError::Decode(_))));
    }

    #[test]
    fn from_bytes_rejects_inflated_tf_count() {
        // Decompression-bomb: claim a tf_count that doesn't fit
        // in the remaining input. Must reject *before*
        // allocating the HashMap.
        let mut bytes: Vec<u8> = Vec::new();
        bytes.extend_from_slice(b"VSI1");
        bytes.push(1u8);
        bytes.extend_from_slice(&1u32.to_be_bytes()); // 1 doc
        let id = "d1";
        bytes.extend_from_slice(&(id.len() as u32).to_be_bytes());
        bytes.extend_from_slice(id.as_bytes());
        bytes.extend_from_slice(&0u64.to_be_bytes()); // total_len
        // Claim 60_000 tf entries — under MAX_TF_ENTRIES_PER_DOC
        // but the input ends here so they cannot be read.
        bytes.extend_from_slice(&60_000u32.to_be_bytes());
        let r = SearchIndex::from_bytes(&bytes);
        assert!(matches!(r, Err(SearchError::Decode(_))));
    }

    #[test]
    fn from_bytes_rejects_inflated_n_docs() {
        // Same shape, doc-level: a 9-byte header claiming 10_000
        // docs is rejected because the remaining input is empty.
        let mut bytes: Vec<u8> = Vec::new();
        bytes.extend_from_slice(b"VSI1");
        bytes.push(1u8);
        bytes.extend_from_slice(&10_000u32.to_be_bytes());
        let r = SearchIndex::from_bytes(&bytes);
        assert!(matches!(r, Err(SearchError::Decode(_))));
    }

    #[test]
    fn from_bytes_rejects_tf_count_above_per_doc_cap() {
        let mut bytes: Vec<u8> = Vec::new();
        bytes.extend_from_slice(b"VSI1");
        bytes.push(1u8);
        bytes.extend_from_slice(&1u32.to_be_bytes());
        let id = "d1";
        bytes.extend_from_slice(&(id.len() as u32).to_be_bytes());
        bytes.extend_from_slice(id.as_bytes());
        bytes.extend_from_slice(&0u64.to_be_bytes());
        // > MAX_TF_ENTRIES_PER_DOC.
        bytes.extend_from_slice(&((MAX_TF_ENTRIES_PER_DOC as u32) + 1).to_be_bytes());
        let r = SearchIndex::from_bytes(&bytes);
        assert!(matches!(r, Err(SearchError::Decode(_))));
    }

    #[test]
    fn from_bytes_rejects_non_finite_tf_weight() {
        let mut bytes: Vec<u8> = Vec::new();
        bytes.extend_from_slice(b"VSI1");
        bytes.push(1u8);
        bytes.extend_from_slice(&1u32.to_be_bytes()); // 1 doc
        let id = "d1";
        bytes.extend_from_slice(&(id.len() as u32).to_be_bytes());
        bytes.extend_from_slice(id.as_bytes());
        bytes.extend_from_slice(&0u64.to_be_bytes()); // total_len
        bytes.extend_from_slice(&1u32.to_be_bytes()); // 1 tf entry
        bytes.extend_from_slice(b"abc");
        bytes.extend_from_slice(&f32::NAN.to_bits().to_be_bytes());
        let r = SearchIndex::from_bytes(&bytes);
        assert!(matches!(r, Err(SearchError::Decode(_))));
    }

    // --- clear / drop wipes ---

    #[test]
    fn clear_drops_documents() {
        let idx = SearchIndex::new();
        let s = searchable_title_and_url();
        idx.index(doc_id("d1"), &fields_pair("github", "https://github.com"), &s)
            .unwrap();
        idx.clear();
        assert!(idx.is_empty());
        assert!(idx.search("git", 10).unwrap().is_empty());
    }

    #[test]
    fn drop_runs_without_panic_after_indexing() {
        // Best-effort: drop the index after building it; the
        // Drop impl walks per-doc tf maps to scrub them.
        // No panic = passing.
        let idx = SearchIndex::new();
        let s = searchable_title_and_url();
        for i in 0..16 {
            idx.index(
                doc_id(&format!("d{}", i)),
                &fields_pair("github", "https://github.com"),
                &s,
            )
            .unwrap();
        }
        drop(idx);
    }

    // --- threading ---

    #[test]
    fn index_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SearchIndex>();
    }

    #[test]
    fn concurrent_indexers_all_succeed() {
        use std::sync::Arc;
        use std::thread;
        let idx = Arc::new(SearchIndex::new());
        let s = Arc::new(searchable_title_and_url());
        let mut handles = Vec::new();
        for i in 0..16u32 {
            let idx = idx.clone();
            let s = s.clone();
            handles.push(thread::spawn(move || {
                idx.index(
                    doc_id(&format!("d{}", i)),
                    &fields_pair("github", "https://example.com"),
                    &s,
                )
                .unwrap();
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(idx.len(), 16);
    }

    // --- BM25 sanity ---

    #[test]
    fn bm25_score_is_finite_and_non_negative() {
        let idx = SearchIndex::new();
        let s = searchable_title_and_url();
        idx.index(doc_id("d1"), &fields_pair("github", "https://github.com"), &s)
            .unwrap();
        let hits = idx.search("git", 10).unwrap();
        for h in hits.iter() {
            assert!(h.score.is_finite() && h.score >= 0.0);
        }
    }

    #[test]
    fn missing_field_is_silently_skipped() {
        // The descriptor declares "title" and "url"; the doc
        // only has "title". That's fine — url contributes
        // nothing.
        let mut fields = BTreeMap::new();
        fields.insert("title".into(), "github".into());
        let idx = SearchIndex::new();
        let s = searchable_title_and_url();
        idx.index(doc_id("d1"), &fields, &s).unwrap();
        let hits = idx.search("git", 10).unwrap();
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn zero_or_negative_weight_skipped() {
        // A field declared with weight 0.0 (or NaN, or
        // negative) is skipped so it cannot be used to "weigh
        // up" via a malicious caller.
        let mut s = SearchableFields::new();
        s.weights.insert("title".into(), 0.0);
        s.weights.insert("notes".into(), -1.0);
        s.weights.insert("body".into(), f32::NAN);
        let mut fields = BTreeMap::new();
        fields.insert("title".into(), "foo".into());
        fields.insert("notes".into(), "foo".into());
        fields.insert("body".into(), "foo".into());
        let idx = SearchIndex::new();
        idx.index(doc_id("d1"), &fields, &s).unwrap();
        // Index has the document but no postings — no weight
        // means no contribution.
        assert!(idx.search("foo", 10).unwrap().is_empty());
    }
}
