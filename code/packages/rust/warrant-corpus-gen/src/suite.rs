//! # One suite: product enumeration, admission, chunks, counts and digests (VER04r 5)
//!
//! A claim `forall b1: T1, ..., bk: Tk | refinements. body` has `k` binders, each with a
//! finite domain ([`crate::domains`]).  The **product ordinals** `0 .. P-1` number the
//! tuples row-major, the *last* binder varying fastest.  A tuple is **admitted** if the
//! conjunction of the refinements (`admit`) holds; the **admitted-case ordinal** counts
//! admitted tuples.  The product is cut into **chunks** of `chunk_size` ordinals, chunk `c`
//! being `[c*chunk_size, (c+1)*chunk_size)`; boundaries are on *product* ordinals, so a chunk
//! can be enumerated alone: unrank `c*chunk_size` (the S2 `unrank_product`) and walk forward.
//!
//! ```text
//!   ordinals   0 ........................ P-1
//!   chunks     [ chunk 0 ][ chunk 1 ][ ... ][ chunk m-1 ]      m = ceil(P / chunk_size)
//!   admission  1 1 0 1 1 1 0 0 1 ...                           admit(tuple), per ordinal
//!   per chunk  cases  first_admitted  calls  wraps  inputs digest  [expected digest]
//! ```
//!
//! ## Admission without a per-case evaluation
//!
//! `admit` usually reads only a *prefix* of the binders (`select_picks` reads `a` and `b`
//! but not `choice`).  Let `k` be one more than the position of the last binder it reads and
//! `r` the product of the remaining sizes.  Then admission is a function of the prefix tuple
//! `q = ordinal / r`, evaluated **once per prefix tuple** into a bit table, and
//!
//! ```text
//!   admitted_before(ordinal) = admitted_prefixes_before(q) * r  +  (bit(q) ? ordinal mod r : 0)
//! ```
//!
//! is O(1): `first_admitted` of any chunk needs no walk over the earlier chunks (VER04r 5.4:
//! "a lane can unrank `c*chunk_size`, enumerate from there, and never touch an earlier
//! chunk").  The `wraps` of `admit` are counted per product ordinal (VER04r 3.3 rule 1): a
//! prefix tuple's count is multiplied by `r`.
//!
//! ## What a chunk run computes
//!
//! For every admitted case, in order: the stream bytes of the binder values (cached per
//! binder value), the value of `expected` for a `reference` suite (evaluated by this crate's
//! own evaluator and encoded by the *subject side's* type), the subject calls and wraps on
//! the pure trace of the body.  Bytes go to the chunk's own hasher and, for a v1-header
//! suite, to the whole-suite hashers in the same pass.
//!
//! ## Slim
//!
//! [`SuiteResult::slim`] is the slim chunk set: the base set of VER04r 5.6 plus the
//! profile's `slim_chunk` lines.  **Witness chunks belong to slice S3c** (they need the fault
//! IR): [`SuiteResult::slim_set`] takes them as a parameter so S3c adds them without
//! re-hashing anything.

use coding_adventures_sha256::Sha256Hasher;
use coding_adventures_warrant_grammar::domain::unrank_product;

use crate::domains::{suite_label, BinderDomain, Label, SuiteDomains, MAX_BINDERS};
use crate::error::{code, GenError};
use crate::eval::{Program, Stats};
use crate::ir::Ir;
use crate::json::Json;
use crate::lower::{LoweredSuite, SuiteKind};
use crate::stream::{
    auto_stride, chunk_digest_framing, context_prefix, encode_in, put_u32, slim_base_set,
    suite_header, Header,
};
use crate::types::Ty;

/// Chunks per suite (`MAX_CHUNKS`, VER04r 5.4).
pub const MAX_CHUNKS: u64 = 4096;
/// Chunks in one slim set (`MAX_SLIM_CHUNKS`, VER04r 5.6).
pub const MAX_SLIM_CHUNKS: usize = 32;
/// Total product ordinals of a `full` run (`MAX_TOTAL_WORK_FULL`, VER04r 14).
pub const MAX_TOTAL_WORK_FULL: u64 = 1 << 32;

/// The facts about one chunk (a line of `chunks/<claim>.txt`, VER04r 5.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkResult {
    /// The chunk index `c`.
    pub index: u64,
    /// Admitted tuples in the chunk.
    pub cases: u64,
    /// The admitted-case ordinal of the chunk's first admitted case (the number of admitted
    /// cases in earlier chunks).
    pub first_admitted: u64,
    /// `expected_calls` contributed by this chunk.
    pub calls: u64,
    /// Wrapped fixed-width evaluations on the pure trace of this chunk's cases (without
    /// the `admit` wraps, which belong to the suite).
    pub wraps: u64,
    /// The raw digest of the chunk's input stream.
    pub inputs: [u8; 32],
    /// The raw digest of the chunk's expected stream (`reference` suites only).
    pub expected: Option<[u8; 32]>,
}

fn hex(d: &[u8; 32]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}

impl ChunkResult {
    /// The chunk-file line of VER04r 5.5, without the newline:
    /// `<c> <cases> <first_admitted> <calls> <inputs_hex> [<expected_hex>]`.
    pub fn line(&self) -> String {
        let mut s = format!(
            "{} {} {} {} {}",
            self.index,
            self.cases,
            self.first_admitted,
            self.calls,
            hex(&self.inputs)
        );
        if let Some(e) = &self.expected {
            s.push(' ');
            s.push_str(&hex(e));
        }
        s
    }
}

/// The slim chunk set of a suite and its two digests (VER04r 5.6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlimSet {
    /// Ascending, duplicate-free chunk indices.
    pub chunks: Vec<u64>,
    /// The `warrant-chunks-v1` framing over the chosen chunks' input digests.
    pub inputs_sha256: String,
    /// The same over the expected digests (`reference` suites only).
    pub expected_sha256: Option<String>,
}

/// Everything computed about one suite.
#[derive(Debug, Clone)]
pub struct SuiteResult {
    /// `warrant/<module>/exhaustive/<claim>`.
    pub id: String,
    /// The claim name.
    pub claim: String,
    /// `"reference"` or `"predicate"`.
    pub kind: &'static str,
    /// The binder names, in order.
    pub binder_names: Vec<String>,
    /// The canonical type name of each binder.
    pub binder_types: Vec<String>,
    /// The descriptor tree of each binder (every node has `size`).
    pub descriptors: Vec<Json>,
    /// The leaves demoted to fit the budget, in the order applied: the binder's index as decimal
    /// text, then `.k` for a tuple component, `.ok` for the payload of a `Result` (the oracle's
    /// `demote` paths, VER04j 2.6).  Each is a 16-bit `fx_full` demoted to the 16-bit `fx_edge`
    /// of [D25].
    pub demoted: Vec<String>,
    /// The weakest label of the binders (VER04r 9.3).
    pub label: Label,
    /// `P`.
    pub product: u64,
    /// The chunk size the suite was cut with.
    pub chunk_size: u64,
    /// `m = ceil(P / chunk_size)` (at least 1).
    pub chunks_total: u64,
    /// Admitted cases.
    pub cases: u64,
    /// `P - cases`.
    pub discarded: u64,
    /// The exact number of subject calls in an unfaulted complete run (VER04r 4.4).
    pub expected_calls: u64,
    /// Wrapped fixed-width evaluations over the whole suite (VER04r 3.3 rule 1): the
    /// `admit` wraps per product ordinal plus the trace wraps per admitted case.
    pub wraps: u64,
    /// The sum of the worst-path costs of `admit`, `body` and (reference) `expected`.
    pub ir_cost: u64,
    /// The stream header.
    pub header: Header,
    /// The type name of the result stream (`Bool` for a predicate).
    pub result_type: String,
    /// All `m` chunks, in order.
    pub chunks: Vec<ChunkResult>,
    /// The v2 suite digest over the chunk input digests.
    pub inputs_sha256: String,
    /// The v2 suite digest over the chunk expected digests (`reference` suites only).
    pub expected_sha256: Option<String>,
    /// The v1 whole-suite input digest (v1-header suites only): the SHA-256 of the
    /// unchunked stream.
    pub v1_inputs_sha256: Option<String>,
    /// The v1 whole-suite expected digest (v1-header `reference` suites only).
    pub v1_expected_sha256: Option<String>,
    /// The slim set (base set plus profile lines; witnesses are S3c).
    pub slim: SlimSet,
    slim_base: Vec<u64>,
}

impl SuiteResult {
    /// The chunk lines of `chunks/<claim>.txt`, one per chunk.
    pub fn chunk_lines(&self) -> Vec<String> {
        self.chunks.iter().map(ChunkResult::line).collect()
    }

    /// The slim set of this suite with `witnesses` added: the base set plus the profile's
    /// `slim_chunk` lines plus the witness chunks, ascending and duplicate-free.
    ///
    /// # Errors
    ///
    /// `limit` for more than [`MAX_SLIM_CHUNKS`] chunks; `W0701` for an index past `m`.
    pub fn slim_set(&self, witnesses: &[u64]) -> Result<SlimSet, GenError> {
        let mut chunks = self.slim_base.clone();
        chunks.extend_from_slice(witnesses);
        chunks.sort_unstable();
        chunks.dedup();
        if chunks.len() > MAX_SLIM_CHUNKS {
            return Err(GenError::new(
                code::LIMIT,
                &self.claim,
                "a slim set has more than 32 chunks (VER04r 5.6)",
            ));
        }
        let (mut inputs, mut expected) = (Vec::new(), Vec::new());
        for c in &chunks {
            let chunk = self.chunks.get(*c as usize).ok_or_else(|| {
                GenError::new(
                    code::W0701,
                    &self.claim,
                    "a slim chunk index is past the last chunk",
                )
            })?;
            inputs.push(chunk.inputs);
            expected.extend(chunk.expected);
        }
        // every chunk of a reference suite has an expected digest, so `expected` is as long
        // as `inputs` exactly when the suite is a reference suite
        let expected = (self.kind == "reference")
            .then(|| chunk_digest_framing(&expected))
            .transpose()?;
        Ok(SlimSet {
            inputs_sha256: chunk_digest_framing(&inputs)?,
            expected_sha256: expected,
            chunks,
        })
    }
}

/// How the profile shapes one suite's slim set.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SuiteOptions {
    /// `slim_stride N`: clamped per suite to the automatic stride `ceil(m/14)` (a larger value
    /// changes nothing).
    pub stride: Option<u64>,
    /// The `slim_chunk <claim> <index>` lines of this claim.
    pub slim_chunks: Vec<u64>,
}

/// How admission is decided for a tuple.
#[derive(Debug, Clone)]
enum Admission {
    /// `admit` reads no binder and is true: every tuple is admitted.
    All,
    /// `admit` reads the first `k` binders: a bit per prefix tuple.
    Table {
        bits: Vec<u64>,
        /// `cum[w]`: admitted prefix tuples in words before `w`.
        cum: Vec<u32>,
        /// The number of prefix tuples.
        pre: u64,
        /// The product of the remaining binders' sizes.
        r: u64,
    },
}

impl Admission {
    fn bit(bits: &[u64], q: u64) -> bool {
        bits[(q / 64) as usize] >> (q % 64) & 1 == 1
    }

    /// Admitted prefix tuples among those before `q` (`q < pre`).
    fn prefix_before(bits: &[u64], cum: &[u32], q: u64) -> u64 {
        let w = (q / 64) as usize;
        let mask = (1u64 << (q % 64)) - 1;
        u64::from(cum[w]) + u64::from((bits[w] & mask).count_ones())
    }
}

/// The hashers of the v1 whole-suite streams, fed in the same pass as the chunk hashers.
pub struct Whole {
    /// The input stream (prefix, then every admitted case).
    pub inputs: Sha256Hasher,
    /// The expected stream, for a `reference` suite.
    pub expected: Option<Sha256Hasher>,
}

/// Everything needed to enumerate one suite.
pub struct SuitePlan<'a> {
    suite: &'a LoweredSuite,
    program: &'a Program<'a>,
    binders: Vec<BinderDomain>,
    demoted: Vec<String>,
    product: u64,
    chunk_size: u64,
    chunks_total: u64,
    header: Header,
    prefix: Vec<u8>,
    result_ty: Ty,
    admission: Admission,
    admit_wraps: u64,
    total_cases: u64,
    /// The call count when it does not depend on the case.
    static_calls: Option<u64>,
    /// Must each case be traced (a dynamic count, or a body that can wrap)?
    trace: bool,
    ir_cost: u64,
}

impl<'a> SuitePlan<'a> {
    /// Plan a suite: check the limits, build the stream prefix and decide admission for
    /// every prefix tuple (the only place `admit` is evaluated).
    ///
    /// `ir_cost` is the suite's worst-path cost (S3a's `suite_cost`), recorded in the row.
    ///
    /// # Errors
    ///
    /// `limit` for more than 8 binders or more than 4,096 chunks; `W0706` for a suite that
    /// admits no case; the evaluator's errors for an `admit` that cannot be evaluated.
    pub fn new(
        program: &'a Program<'a>,
        suite: &'a LoweredSuite,
        domains: SuiteDomains,
        ir_cost: u64,
        chunk_size: u64,
    ) -> Result<SuitePlan<'a>, GenError> {
        let at = |e: GenError| e.at(&suite.claim);
        if domains.binders.len() > MAX_BINDERS {
            return Err(at(GenError::new(
                code::LIMIT,
                "",
                "a claim has more than 8 binders (VER04r 5.3)",
            )));
        }
        if !chunk_size.is_power_of_two() || !((1 << 10)..=(1 << 16)).contains(&chunk_size) {
            return Err(at(GenError::new(
                code::LIMIT,
                "",
                "chunk_size is a power of two in 2^10..2^16",
            )));
        }
        let SuiteDomains {
            binders,
            product,
            demoted,
        } = domains;
        let sizes: Vec<u64> = binders.iter().map(|b| b.size).collect();
        let checked: Option<u64> = sizes.iter().try_fold(1u64, |a, n| a.checked_mul(*n));
        if checked != Some(product) {
            return Err(at(GenError::new(
                code::W0706,
                "",
                "the product of the domain sizes does not fit 64 bits",
            )));
        }
        if product == 0 {
            return Err(at(GenError::new(
                code::W0706,
                "",
                "a binder domain is empty, so the product has no ordinals",
            )));
        }
        let chunks_total = product.div_ceil(chunk_size).max(1);
        if chunks_total > MAX_CHUNKS {
            return Err(at(GenError::new(
                code::LIMIT,
                "",
                "a suite has more than 4096 chunks; raise chunk_size (VER04r 5.4)",
            )));
        }
        let types: Vec<Ty> = binders.iter().map(|b| b.ty.clone()).collect();
        let result_ty = match &suite.kind {
            SuiteKind::Reference { result, .. } => result.clone(),
            SuiteKind::Predicate => Ty::Bool,
        };
        let header = suite_header(&types, &result_ty);
        let prefix = context_prefix(&suite.id, &types, &result_ty, header).map_err(at)?;

        // Which binders does `admit` read?
        let names: Vec<&str> = binders.iter().map(|b| b.name.as_str()).collect();
        let mut last: Option<usize> = None;
        suite.admit.walk(&mut |n| {
            if let Ir::Var(v) = n {
                if let Some(p) = names.iter().position(|x| x == v) {
                    last = last.max(Some(p));
                }
            }
        });
        let k = last.map_or(0, |p| p + 1);
        let (admission, admit_wraps, total_cases) =
            build_admission(program, suite, &binders, &sizes, k, product).map_err(at)?;
        if total_cases == 0 {
            return Err(at(GenError::new(
                code::W0706,
                "",
                "a suite admits zero cases (VER04r 4.5)",
            )));
        }
        let static_calls = program.static_calls(&suite.body);
        // [D19]: a call of an op that has an `admit` is checked on every admitted case, which
        // is a trace of the body; an argument that depends on a subject result is refused.
        program.check_admit_calls(&suite.body).map_err(at)?;
        let trace = static_calls.is_none()
            || program.may_fail_or_wrap(&suite.body)
            || program.calls_admitted_op(&suite.body);
        Ok(SuitePlan {
            suite,
            program,
            binders,
            demoted,
            product,
            chunk_size,
            chunks_total,
            header,
            prefix,
            result_ty,
            admission,
            admit_wraps,
            total_cases,
            static_calls,
            trace,
            ir_cost,
        })
    }

    /// `P`.
    pub fn product(&self) -> u64 {
        self.product
    }

    /// `m`, the number of chunks.
    pub fn chunks_total(&self) -> u64 {
        self.chunks_total
    }

    /// Admitted cases in the whole suite.
    pub fn total_cases(&self) -> u64 {
        self.total_cases
    }

    /// Admitted cases among product ordinals `< ordinal` (O(1); see the module docs).
    pub fn admitted_before(&self, ordinal: u64) -> u64 {
        match &self.admission {
            Admission::All => ordinal,
            Admission::Table { bits, cum, pre, r } => {
                let (q, rem) = (ordinal / r, ordinal % r);
                if q >= *pre {
                    return u64::from(cum[cum.len() - 1]) * r;
                }
                Admission::prefix_before(bits, cum, q) * r
                    + if Admission::bit(bits, q) { rem } else { 0 }
            }
        }
    }

    fn admits(&self, ordinal: u64) -> bool {
        match &self.admission {
            Admission::All => true,
            Admission::Table { bits, r, .. } => Admission::bit(bits, ordinal / r),
        }
    }

    /// The product-ordinal range `[lo, hi)` of chunk `c`.
    ///
    /// # Errors
    ///
    /// `W0701` for a chunk past the last.
    pub fn chunk_bounds(&self, c: u64) -> Result<(u64, u64), GenError> {
        if c >= self.chunks_total {
            return Err(GenError::new(
                code::W0701,
                &self.suite.claim,
                "a chunk index is past the last chunk",
            ));
        }
        Ok((
            c * self.chunk_size,
            self.product.min((c + 1) * self.chunk_size),
        ))
    }

    fn is_reference(&self) -> bool {
        matches!(self.suite.kind, SuiteKind::Reference { .. })
    }

    /// A fresh pair of v1 whole-suite hashers seeded with the stream prefix.
    fn whole(&self) -> Whole {
        let mut inputs = Sha256Hasher::new();
        inputs.update(&self.prefix);
        let expected = self.is_reference().then(|| {
            let mut h = Sha256Hasher::new();
            h.update(&self.prefix);
            h
        });
        Whole { inputs, expected }
    }

    /// Walk every **admitted** case in product order, calling `f(ordinal, vars)` with the
    /// binder values; `f` returns `Ok(false)` to stop early.  This is the seam of the
    /// witness search (VER04r 5.6): it digests nothing and counts nothing.
    ///
    /// # Errors
    ///
    /// Whatever `f` returns, and `W0701` if a binder value cannot be produced.
    pub fn scan_admitted(
        &self,
        f: &mut dyn FnMut(u64, &crate::eval::Vars<'_>) -> Result<bool, GenError>,
    ) -> Result<(), GenError> {
        let at = |e: GenError| e.at(&self.suite.claim);
        let n = self.binders.len();
        let sizes: Vec<u64> = self.binders.iter().map(|b| b.size).collect();
        let mut idx = vec![0u64; n];
        let mut vars: Vec<(&str, crate::value::Value)> = self
            .binders
            .iter()
            .map(|b| (b.name.as_str(), crate::value::Value::Bool(false)))
            .collect();
        let mut dirty = 0usize;
        for ordinal in 0..self.product {
            if self.admits(ordinal) {
                for j in dirty..n {
                    vars[j].1 = self.binders[j].value(idx[j]).map_err(at)?;
                }
                dirty = n;
                if !f(ordinal, &vars)? {
                    return Ok(());
                }
            }
            let mut j = n;
            while j > 0 {
                j -= 1;
                dirty = dirty.min(j);
                idx[j] += 1;
                if idx[j] < sizes[j] {
                    break;
                }
                idx[j] = 0;
            }
        }
        Ok(())
    }

    /// Enumerate chunk `c` alone, by unranking its first ordinal, and digest its streams.
    /// `whole` receives the same bytes (the v1 whole-suite digest is fed in one pass).
    ///
    /// # Errors
    ///
    /// `W0701` for a bad chunk index or an evaluation error; every [`Program::eval`] error.
    pub fn run_chunk(
        &self,
        c: u64,
        mut whole: Option<&mut Whole>,
    ) -> Result<ChunkResult, GenError> {
        let at = |e: GenError| e.at(&self.suite.claim);
        let (lo, hi) = self.chunk_bounds(c)?;
        let first = self.admitted_before(lo);
        let want = self.admitted_before(hi) - first;
        let mut chunk_index = Vec::with_capacity(4);
        put_u32(&mut chunk_index, c as usize).map_err(at)?;
        let mut h_in = Sha256Hasher::new();
        h_in.update(&self.prefix);
        h_in.update(&chunk_index);
        let mut h_ex = self.is_reference().then(|| {
            let mut h = Sha256Hasher::new();
            h.update(&self.prefix);
            h.update(&chunk_index);
            h
        });

        let n = self.binders.len();
        let sizes: Vec<u64> = self.binders.iter().map(|b| b.size).collect();
        let mut idx = unrank_product(&sizes, lo)
            .ok_or_else(|| at(GenError::internal("a chunk start cannot be unranked")))?;
        let mut vars: Vec<(&str, crate::value::Value)> = self
            .binders
            .iter()
            .map(|b| (b.name.as_str(), crate::value::Value::Bool(false)))
            .collect();
        let mut dirty = 0usize;
        let (mut in_buf, mut ex_buf) = (Vec::with_capacity(1 << 17), Vec::with_capacity(1 << 15));
        let (mut stats, mut scratch) = (Stats::default(), Stats::default());
        let (mut cases, mut calls) = (0u64, 0u64);
        let expected_ir = match &self.suite.kind {
            SuiteKind::Reference { expected, .. } => Some(expected),
            SuiteKind::Predicate => None,
        };

        let mut flush = |in_buf: &mut Vec<u8>, ex_buf: &mut Vec<u8>| {
            h_in.update(in_buf);
            if let Some(w) = whole.as_deref_mut() {
                w.inputs.update(in_buf);
            }
            if let Some(h) = h_ex.as_mut() {
                h.update(ex_buf);
                if let Some(w) = whole.as_deref_mut() {
                    if let Some(he) = w.expected.as_mut() {
                        he.update(ex_buf);
                    }
                }
            }
            in_buf.clear();
            ex_buf.clear();
        };

        for ordinal in lo..hi {
            if self.admits(ordinal) {
                for j in dirty..n {
                    vars[j].1 = self.binders[j].value(idx[j]).map_err(at)?;
                }
                dirty = n;
                cases += 1;
                for (b, i) in self.binders.iter().zip(&idx) {
                    b.encode_into(*i, &mut in_buf).map_err(at)?;
                }
                if let Some(e) = expected_ir {
                    let v = self.program.eval(e, &vars, &mut scratch).map_err(at)?;
                    encode_in(self.program.decls(), &self.result_ty, &v, &mut ex_buf).map_err(at)?;
                }
                calls += match self.static_calls {
                    Some(k) if !self.trace => k,
                    _ => self
                        .program
                        .trace(&self.suite.body, &vars, &mut stats)
                        .map_err(|mut e| {
                            if e.during_case {
                                // The address of VER04i 2.4: the suite's admitted-case ordinal.
                                e.message
                                    .push_str(&format!(", at admitted case {}", first + cases - 1));
                            }
                            at(e)
                        })?,
                };
                if in_buf.len() >= 1 << 16 {
                    flush(&mut in_buf, &mut ex_buf);
                }
            }
            // Advance the odometer, last binder fastest.
            let mut j = n;
            while j > 0 {
                j -= 1;
                dirty = dirty.min(j);
                idx[j] += 1;
                if idx[j] < sizes[j] {
                    break;
                }
                idx[j] = 0;
            }
        }
        flush(&mut in_buf, &mut ex_buf);
        if cases != want {
            return Err(at(GenError::internal(
                "the admission arithmetic disagrees with the enumeration",
            )));
        }
        Ok(ChunkResult {
            index: c,
            cases,
            first_admitted: first,
            calls,
            wraps: stats.wraps,
            inputs: h_in.digest(),
            expected: h_ex.as_ref().map(Sha256Hasher::digest),
        })
    }

    /// Run every chunk and assemble the suite's row.
    ///
    /// # Errors
    ///
    /// As [`SuitePlan::run_chunk`]; `profile` for a `slim_chunk` past the last chunk.
    pub fn run(&self, opts: &SuiteOptions) -> Result<SuiteResult, GenError> {
        let at = |e: GenError| e.at(&self.suite.claim);
        let m = self.chunks_total;
        if opts.slim_chunks.iter().any(|c| *c >= m) {
            return Err(at(GenError::new(
                code::PROFILE,
                "",
                "a slim_chunk line names a chunk past the last chunk",
            )));
        }
        let v1 = self.header == Header::V1;
        let mut whole = v1.then(|| self.whole());
        let mut chunks = Vec::with_capacity(m as usize);
        for c in 0..m {
            chunks.push(self.run_chunk(c, whole.as_mut())?);
        }
        let inputs: Vec<[u8; 32]> = chunks.iter().map(|c| c.inputs).collect();
        let inputs_sha256 = chunk_digest_framing(&inputs).map_err(at)?;
        let expected_sha256 = if self.is_reference() {
            // every chunk of a reference suite carries an expected digest
            let ex: Vec<[u8; 32]> = chunks.iter().filter_map(|c| c.expected).collect();
            Some(chunk_digest_framing(&ex).map_err(at)?)
        } else {
            None
        };
        let cases: u64 = chunks.iter().map(|c| c.cases).sum();
        if cases != self.total_cases {
            return Err(at(GenError::internal(
                "the chunk cases do not sum to the suite cases",
            )));
        }
        // At most 2^28 ordinals times 65,536 node evaluations per case (VER04r 14): far below
        // 2^64, so plain addition cannot overflow.
        let wraps = self.admit_wraps + chunks.iter().map(|c| c.wraps).sum::<u64>();
        let mut slim_base = slim_base_set(
            &self.suite.id,
            m,
            // owner decision 2026-11-17: a profile stride is clamped per suite to the
            // automatic stride (never an error); for m <= 16 every chunk is slim anyway
            opts.stride.filter(|s| m > 16 && *s < auto_stride(m)),
        );
        slim_base.extend_from_slice(&opts.slim_chunks);
        slim_base.sort_unstable();
        slim_base.dedup();
        let mut result = SuiteResult {
            id: self.suite.id.clone(),
            claim: self.suite.claim.clone(),
            kind: if self.is_reference() {
                "reference"
            } else {
                "predicate"
            },
            binder_names: self.binders.iter().map(|b| b.name.clone()).collect(),
            binder_types: self.binders.iter().map(|b| b.ty.name()).collect(),
            descriptors: self.binders.iter().map(|b| b.descriptor.clone()).collect(),
            demoted: self.demoted.clone(),
            label: suite_label(&self.binders),
            product: self.product,
            chunk_size: self.chunk_size,
            chunks_total: m,
            cases,
            discarded: self.product - cases,
            expected_calls: chunks.iter().map(|c| c.calls).sum(),
            wraps,
            ir_cost: self.ir_cost,
            header: self.header,
            result_type: self.result_ty.name(),
            chunks,
            inputs_sha256,
            expected_sha256,
            v1_inputs_sha256: whole.as_ref().map(|w| w.inputs.hex_digest()),
            v1_expected_sha256: whole
                .as_ref()
                .and_then(|w| w.expected.as_ref().map(Sha256Hasher::hex_digest)),
            slim: SlimSet {
                chunks: Vec::new(),
                inputs_sha256: String::new(),
                expected_sha256: None,
            },
            slim_base,
        };
        result.slim = result.slim_set(&[]).map_err(at)?;
        Ok(result)
    }
}

/// Decide admission for every prefix tuple of the first `k` binders.
fn build_admission(
    program: &Program<'_>,
    suite: &LoweredSuite,
    binders: &[BinderDomain],
    sizes: &[u64],
    k: usize,
    product: u64,
) -> Result<(Admission, u64, u64), GenError> {
    // `wraps` is at most 2^28 ordinals * 65,536 node evaluations (VER04r 14): plain u64
    // arithmetic cannot overflow.
    if k == 0 {
        let mut st = Stats::default();
        let ok = program.eval(&suite.admit, &[], &mut st)?.as_bool()?;
        let wraps = st.wraps * product;
        // A constant-false admit admits nothing: the caller refuses the suite (VER04r 4.5).
        return Ok((Admission::All, wraps, if ok { product } else { 0 }));
    }
    let pre: u64 = sizes[..k].iter().product();
    let r: u64 = sizes[k..].iter().product();
    let words = pre.div_ceil(64) as usize;
    let mut bits = vec![0u64; words];
    let mut vars: Vec<(&str, crate::value::Value)> = Vec::with_capacity(k);
    for b in &binders[..k] {
        vars.push((b.name.as_str(), b.value(0)?));
    }
    let mut idx = vec![0u64; k];
    let mut wraps = 0u64;
    for q in 0..pre {
        let mut st = Stats::default();
        let ok = program.eval(&suite.admit, &vars, &mut st)?.as_bool()?;
        wraps += st.wraps * r;
        if ok {
            bits[(q / 64) as usize] |= 1 << (q % 64);
        }
        let mut j = k;
        while j > 0 {
            j -= 1;
            idx[j] += 1;
            if idx[j] < sizes[j] {
                vars[j].1 = binders[j].value(idx[j])?;
                break;
            }
            idx[j] = 0;
            vars[j].1 = binders[j].value(0)?;
        }
    }
    let mut cum = Vec::with_capacity(words + 1);
    let mut total = 0u32;
    for w in &bits {
        cum.push(total);
        total += w.count_ones();
    }
    cum.push(total);
    let cases = u64::from(total) * r;
    Ok((Admission::Table { bits, cum, pre, r }, wraps, cases))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chunk_line_is_the_section_5_5_format() {
        let c = ChunkResult {
            index: 2,
            cases: 100,
            first_admitted: 4000,
            calls: 300,
            wraps: 0,
            inputs: [0xab; 32],
            expected: Some([0x01; 32]),
        };
        assert_eq!(
            c.line(),
            format!("2 100 4000 300 {} {}", "ab".repeat(32), "01".repeat(32))
        );
        let p = ChunkResult {
            expected: None,
            ..c
        };
        assert_eq!(p.line(), format!("2 100 4000 300 {}", "ab".repeat(32)));
        assert!(!p.line().ends_with(' '));
    }

    #[test]
    fn admission_prefix_arithmetic_on_a_hand_built_table() {
        // 5 prefix tuples, admitted: 1 0 1 1 0 ; r = 3 (so 15 ordinals)
        let bits = vec![0b01101u64];
        let cum = vec![0u32, 3];
        let a = Admission::Table {
            bits: bits.clone(),
            cum: cum.clone(),
            pre: 5,
            r: 3,
        };
        // helper replicating `admitted_before` without a plan
        let before = |ordinal: u64| -> u64 {
            let Admission::Table { bits, cum, pre, r } = &a else {
                unreachable!()
            };
            let (q, rem) = (ordinal / r, ordinal % r);
            if q >= *pre {
                return u64::from(cum[cum.len() - 1]) * r;
            }
            Admission::prefix_before(bits, cum, q) * r
                + if Admission::bit(bits, q) { rem } else { 0 }
        };
        // by hand: ordinals 0..3 are prefix 0 (admitted): 0,1,2 ; 3..6 prefix 1 (not) ...
        let admitted = [1, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 1, 0, 0, 0];
        for o in 0..=15u64 {
            let want: u64 = admitted[..o as usize].iter().sum();
            assert_eq!(before(o), want, "ordinal {o}");
        }
    }
}
