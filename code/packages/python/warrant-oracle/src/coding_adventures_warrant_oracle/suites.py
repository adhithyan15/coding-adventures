"""Suites: product enumeration, admission, chunking, counts and digests (VER04r 5, 10.3).

This is the heart of oracle check 2 ("Enumeration and evaluation").  For one
claim it reproduces, *independently of the generator*:

``cases``, ``discarded``, ``first_admitted``, ``calls``, ``inputs_sha256`` and (for
``reference`` suites) ``expected_sha256`` -- per chunk, plus the suite digests, the
``slim`` chunk subset and ``wraps``.

The vocabulary, in one paragraph
--------------------------------
A claim ``forall b1: T1, ..., bk: Tk | refinements. body`` has ``k`` *binders*, each
with a finite domain (:mod:`.domains`).  The **product ordinals** ``0 .. P-1`` number
the tuples in row-major order, the *last* binder varying fastest.  A tuple is
**admitted** if the conjunction of the refinements (``admit``) is true; the
**admitted-case ordinal** counts admitted tuples.  The product is cut into
**chunks** of ``chunk_size`` ordinals; chunk ``c`` is ordinals
``[c*chunk_size, (c+1)*chunk_size)``.  Boundaries are on *product* ordinals, so a
chunk can be enumerated alone: unrank ``c*chunk_size`` and walk forward.

Counting without walking
------------------------
``cases`` and ``first_admitted`` of every chunk are needed even when only some
chunks are hashed (``slim``).  They are computed by *arithmetic*: ``admit`` usually
depends only on a prefix of the binders, so it is evaluated once per prefix tuple
(not once per case) and the counts follow from block sums.  Walking a hashed chunk
re-counts its admitted cases as an internal cross-check.
"""

from __future__ import annotations

import hashlib
from array import array
from dataclasses import dataclass, field

from . import ast as A
from .domains import Desc, _Listed, check_budget, demote, descriptor_for, refine_counts
from .errors import OracleError
from .ir_eval import (
    OutsideAdmit,
    Program,
    WrapsInAdmit,
    admit_check_cost,
    compile_admit_check,
    compile_calls,
    compile_trace,
    fixed_arith_over_subject,
    may_wrap,
)
from .irtree import free_vars
from .lower import BOOL, LoweredSuite, Spec
from .stream import (
    chunk_digest_framing,
    context_prefix,
    make_encoder,
    slim_base_set,
    suite_header,
    u32,
)

#: Total-work bounds (VER04r sections 4.2 and 14).  "Work" counts product ordinals the
#: oracle visits (a hashed chunk, an admission walk, a counting walk); IR work is
#: ordinals times the suite's ``ir_cost``.
MAX_TOTAL_WORK_SLIM = 32_000_000
MAX_TOTAL_WORK_FULL = 1 << 32
MAX_IR_WORK_SLIM = 64_000_000_000
MAX_IR_WORK_FULL = 8_589_934_592_000


class Work:
    """A shared meter for one oracle run; exceeding a bound is a ``limit`` error."""

    def __init__(self, depth: str = "full", cases: int | None = None, ir_work: int | None = None):
        slim = depth == "slim"
        self.max_cases = (
            cases if cases is not None else (MAX_TOTAL_WORK_SLIM if slim else MAX_TOTAL_WORK_FULL)
        )
        self.max_ir = (
            ir_work if ir_work is not None else (MAX_IR_WORK_SLIM if slim else MAX_IR_WORK_FULL)
        )
        self.cases = 0
        self.ir = 0
        self.walked = 0

    def walk(self, ordinals: int) -> None:
        """Meter an *analysis* walk (admission, call counting, wraps).

        VER04r 10.4: the oracle verifies every chunk's ``cases``/``first_admitted``
        by an admission-only pass whatever the depth, so these walks are not part
        of the ``slim`` case budget (that budget bounds the chunks that are run and
        hashed).  They are still bounded: each is at most one product
        (``2^28``) and all of them together stay under the ``full`` limit.
        """
        self.walked += ordinals
        if self.walked > MAX_TOTAL_WORK_FULL:
            raise OracleError("limit", "total analysis work bound exceeded")

    def charge_ir(self, ir_work: int) -> None:
        """Charge IR work that is not a recorded ``ir_cost`` (the check of [D19])."""
        self.ir += ir_work
        if self.ir > self.max_ir:
            raise OracleError("limit", "total IR work bound exceeded")

    def charge(self, ordinals: int, ir_cost: int) -> None:
        self.cases += ordinals
        self.ir += ordinals * ir_cost
        if self.cases > self.max_cases:
            raise OracleError("limit", "total work bound exceeded")
        if self.ir > self.max_ir:
            raise OracleError("limit", "total IR work bound exceeded")


DEFAULT_CHUNK_SIZE = 1 << 16
MAX_CHUNKS = 4096
MAX_BINDERS = 8  # VER04r 5.3 and 14: a claim has 0 to 8 binders (R75)
DEFAULT_BUDGET = 1 << 24


@dataclass
class ChunkResult:
    index: int
    cases: int
    first_admitted: int
    calls: int
    inputs: bytes  # raw 32-byte digest
    expected: bytes | None

    def line(self) -> str:
        """The chunk-file line of VER04r section 5.5."""
        base = f"{self.index} {self.cases} {self.first_admitted} {self.calls} {self.inputs.hex()}"
        return base + (f" {self.expected.hex()}" if self.expected is not None else "")


@dataclass
class SuiteResult:
    suite_id: str
    claim: str
    kind: str
    binder_types: list
    descriptors: list
    demoted: list
    label: str
    product: int
    chunk_size: int
    chunks_total: int
    cases: int
    discarded: int
    expected_calls: int
    wraps: int
    header: str
    chunks: list = field(default_factory=list)  # executed chunk results
    slim_chunks: list = field(default_factory=list)
    inputs_sha256: str | None = None  # suite digest over *all* chunks (full run)
    expected_sha256: str | None = None
    slim_inputs_sha256: str | None = None
    slim_expected_sha256: str | None = None
    v1_inputs_sha256: str | None = None
    v1_expected_sha256: str | None = None
    result_type: str = "Bool"
    ir_cost: int = 0


class _Binder:
    """One binder's descriptor, encoder and (lazily) cached value/encoding lists."""

    def __init__(self, name: str, ty: A.Ty, desc: Desc, enc) -> None:
        self.name = name
        self.ty = ty
        self.desc = desc
        self.enc = enc
        self.size = desc.size()
        self._enc_list: list[bytes] | None = None

    def values(self, start: int, stop: int) -> list:
        if isinstance(self.desc, _Listed):
            return self.desc.values[start:stop]
        return [self.desc.nth(i) for i in range(start, stop)]

    def encodings(self, start: int, stop: int) -> list[bytes]:
        if isinstance(self.desc, _Listed):
            if self._enc_list is None:
                self._enc_list = [self.enc(v) for v in self.desc.values]
            return self._enc_list[start:stop]
        return [self.enc(v) for v in self.values(start, stop)]

    def one(self, i: int) -> object:
        return self.desc.values[i] if isinstance(self.desc, _Listed) else self.desc.nth(i)


def _free_vars(ir: object) -> set[str]:
    """The binders an IR tree reads (a ``match`` pattern's own binders are not free)."""
    return free_vars(ir)


class SuitePlan:
    """Everything needed to enumerate one suite: domains, closures and sizes."""

    def __init__(
        self,
        spec: Spec,
        prog: Program,
        suite: LoweredSuite,
        *,
        chunk_size: int = DEFAULT_CHUNK_SIZE,
        budget: int = DEFAULT_BUDGET,
        work: Work | None = None,
    ) -> None:
        self.work = work if work is not None else Work("full")
        if suite.unsupported is not None:
            raise suite.unsupported
        if suite.refusal is not None:
            raise suite.refusal
        if chunk_size & (chunk_size - 1) or not (1 << 10) <= chunk_size <= (1 << 16):
            raise OracleError("generation", "chunk_size is a power of two in 2^10..2^16")
        self.spec = spec
        self.prog = prog
        self.suite = suite
        self.chunk_size = chunk_size
        if len(suite.binders) > MAX_BINDERS:  # VER04r 5.3, 14 "0 to 8", R75: before any descriptor
            raise OracleError("limit", "a claim has more than 8 binders (VER04r 5.3)")
        descs = [descriptor_for(spec, ty) for _, ty in suite.binders]
        # [D26], R10: a ``Nat`` binder that is directly a shift count has the 73-value ``nat_shift``
        # leaf; decided from the claim *body* (never ``admit``), before any budget demotion.
        refine_counts(descs, suite.binders, suite.body, spec.version)
        self.demoted = demote(descs, budget)
        self.product = check_budget(descs, budget)
        self.binders = [
            _Binder(name, ty, d, make_encoder(spec, ty))
            for (name, ty), d in zip(suite.binders, descs)
        ]
        self.chunks_total = max(1, -(-self.product // chunk_size))
        if self.chunks_total > MAX_CHUNKS:
            raise OracleError("generation", "more than 4096 chunks; raise chunk_size")
        tenv = dict(suite.binders)
        admit = prog.compile(suite.admit, tenv, allow_subject=False)
        costs = [admit.cost]
        if admit.ty != BOOL:
            raise OracleError("type", "admit must be Bool")
        self.admit_ir = suite.admit
        self.admit = admit.fn
        # Which binders does admit read?  Only the first ``admit_last + 1`` matter.
        names = [b.name for b in self.binders]
        used = _free_vars(suite.admit)
        self.admit_last = max((names.index(v) for v in used), default=-1)
        self.admit_const = False
        if self.admit_last == -1:
            # No variable at all: admission is one constant for the whole suite.
            if not self.admit({}):
                raise OracleError("generation", "a suite admits zero cases")
            self.admit_const = True
        body = prog.compile(suite.body, tenv, allow_subject=True)
        costs.append(body.cost)
        if body.ty != BOOL:
            raise OracleError("type", "a claim body must be Bool")
        if spec.version == "v0" and fixed_arith_over_subject(prog, suite.body, tenv):
            # Section 3.3 rule 3: the v0 harness evaluates U8 as Nat, so a fixed-width
            # node above a subject result may wrap in v2 and not there.  Static refusal.
            raise OracleError(
                "generation",
                "W0701: fixed-width arithmetic over a subject result in a v0 document",
            )
        self._trace = compile_trace(prog, suite.body, tenv)
        self._wraps_needed = may_wrap(prog, suite.admit) or self._trace is not None
        self._wraps: int | None = None
        self.calls = compile_calls(prog, suite.body, tenv)
        # [D19]: a call of an op with an ``admit`` is checked on every admitted case.
        self._admit_check = compile_admit_check(prog, suite.body, tenv, spec.version == "v0")
        # The check is IR work too, charged per product ordinal (conservatively), before any
        # case runs; the recorded ``ir_cost`` below does not include it.
        self.work.charge_ir(self.product * admit_check_cost(prog, suite.body, tenv))
        self._admits_checked = False
        self.expected = None
        self.enc_result = None
        if suite.kind == "reference":
            exp = prog.compile(suite.expected, tenv, allow_subject=False)
            costs.append(exp.cost)
            prog.compile(suite.subject, tenv, allow_subject=True)
            self.expected = exp.fn
            self.enc_result = make_encoder(spec, suite.result)
        #: VER04r 4.2: the sum of the worst-path costs of admit, body and expected.
        self.ir_cost = sum(costs)
        types = [t for _, t in suite.binders]
        self.header = suite_header(types, suite.result)
        self.prefix = context_prefix(suite.id, types, suite.result, self.header)
        self._prefix_counts: tuple | None = None

    # -- admission arithmetic ------------------------------------------------------

    def _admission(self) -> tuple[bytearray | None, array | None, int]:
        """``(adm, cumulative, R)`` for the prefix binders ``0..admit_last``.

        ``adm[q]`` says whether prefix tuple ``q`` is admitted, ``cumulative[q]``
        is the number of admitted prefix tuples before ``q`` and ``R`` is the
        product of the remaining (unread) binders, so every admitted prefix tuple
        stands for ``R`` product ordinals.  ``adm is None`` means every tuple is
        admitted (a constant ``true`` admission).
        """
        if self._prefix_counts is not None:
            return self._prefix_counts
        if self.admit_const:
            self._prefix_counts = (None, None, 1)
            return self._prefix_counts
        k = self.admit_last + 1
        sizes = [b.size for b in self.binders]
        r = 1
        for s in sizes[k:]:
            r *= s
        pre = 1
        for s in sizes[:k]:
            pre *= s
        self.work.walk(pre)
        adm = bytearray(pre)
        names = [b.name for b in self.binders[:k]]
        admit = self.admit
        cols = [b.values(0, b.size) for b in self.binders[:k]]
        idx = [0] * k
        env = {n: c[0] for n, c in zip(names, cols)}
        for q in range(pre):
            if admit(env):
                adm[q] = 1
            # advance the odometer, last prefix binder fastest
            j = k - 1
            while j >= 0:
                idx[j] += 1
                if idx[j] < sizes[j]:
                    env[names[j]] = cols[j][idx[j]]
                    break
                idx[j] = 0
                env[names[j]] = cols[j][0]
                j -= 1
        cum = array("Q", [0]) * (pre + 1)
        total = 0
        for q in range(pre):
            cum[q] = total
            total += adm[q]
        cum[pre] = total
        self._prefix_counts = (adm, cum, r)
        return self._prefix_counts

    def admitted_before(self, ordinal: int) -> int:
        """Admitted cases among product ordinals ``< ordinal``."""
        adm, cum, r = self._admission()
        if adm is None:  # everything admitted
            return ordinal
        q, rem = divmod(ordinal, r)
        if q >= len(adm):
            return cum[len(adm)] * r
        return cum[q] * r + adm[q] * rem

    def total_cases(self) -> int:
        return self.admitted_before(self.product)

    # -- one chunk -------------------------------------------------------------------

    def chunk_bounds(self, c: int) -> tuple[int, int]:
        if not 0 <= c < self.chunks_total:
            raise IndexError("chunk index out of range")
        return c * self.chunk_size, min(self.product, (c + 1) * self.chunk_size)

    def run_chunk(
        self, c: int, sinks_in: list | None = None, sinks_exp: list | None = None
    ) -> ChunkResult:
        """Enumerate chunk ``c`` by unranking its first ordinal; digest its streams.

        ``sinks_in`` / ``sinks_exp`` receive the same bytes as extra hash objects
        (used for the unchunked v1 whole-suite digest).
        """
        lo, hi = self.chunk_bounds(c)
        self.work.charge(hi - lo, self.ir_cost)
        first = self.admitted_before(lo)
        want = self.admitted_before(hi) - first
        h_in = hashlib.sha256(self.prefix + u32(c))
        h_ex = hashlib.sha256(self.prefix + u32(c)) if self.expected is not None else None
        ins = [h_in, *(sinks_in or [])]
        exs = [h_ex, *(sinks_exp or [])] if h_ex is not None else []
        cases = calls = 0
        bs = self.binders
        last = bs[-1] if bs else None
        n = len(bs)
        names = [b.name for b in bs]
        calls_fn = self.calls
        static_calls = calls_fn if not callable(calls_fn) else None
        pos = lo
        if n == 0:
            # A claim with no binders has one (empty) tuple.
            env: dict = {}
            if self.admit(env):
                cases = 1
                ins_bytes = b""
                for h in ins:
                    h.update(ins_bytes)
                if self.expected is not None:
                    value = self.enc_result(self.expected(env))
                    for h in exs:
                        h.update(value)
                calls = calls_fn(env) if callable(calls_fn) else calls_fn
            return self._finish(c, cases, first, want, calls, h_in, h_ex)
        length = last.size
        sizes = [b.size for b in bs]
        while pos < hi:
            row, s = divmod(pos, length)
            e = min(length, s + (hi - pos))
            # Unrank the prefix binders of this row (all but the last).
            idx = []
            r = row
            for size in reversed(sizes[:-1]):
                r, rem = divmod(r, size)
                idx.append(rem)
            idx.reverse()
            env = {names[j]: bs[j].one(idx[j]) for j in range(n - 1)}
            pre = b"".join(bs[j].enc(env[names[j]]) for j in range(n - 1))
            enc_last = last.encodings(s, e)
            per_case_admit = (not self.admit_const) and self.admit_last == n - 1
            row_admitted = True
            if not self.admit_const and not per_case_admit:
                row_admitted = bool(self.admit(env))
            if row_admitted:
                if self.expected is None and not callable(calls_fn) and not per_case_admit:
                    # Fast path: every case of the row is admitted and nothing
                    # depends on the value, so the row is a pure join.
                    blob = b"".join([pre + x for x in enc_last])
                    for h in ins:
                        h.update(blob)
                    cases += e - s
                    calls += (e - s) * static_calls
                else:
                    in_parts: list[bytes] = []
                    ex_parts: list[bytes] = []
                    for off, value in enumerate(last.values(s, e)):
                        env[last.name] = value
                        if per_case_admit and not self.admit(env):
                            continue
                        in_parts.append(pre + enc_last[off])
                        if self.expected is not None:
                            ex_parts.append(self.enc_result(self.expected(env)))
                        calls += calls_fn(env) if callable(calls_fn) else calls_fn
                    cases += len(in_parts)
                    blob = b"".join(in_parts)
                    for h in ins:
                        h.update(blob)
                    if ex_parts:
                        blob = b"".join(ex_parts)
                        for h in exs:
                            h.update(blob)
            pos += e - s
        if cases != want:
            raise OracleError("corpus", "internal: admission arithmetic disagrees with enumeration")
        return self._finish(c, cases, first, want, calls, h_in, h_ex)

    def _finish(
        self, c: int, cases: int, first: int, want: int, calls: int, h_in, h_ex
    ) -> ChunkResult:
        return ChunkResult(
            c, cases, first, calls, h_in.digest(), h_ex.digest() if h_ex is not None else None
        )

    # -- the whole suite ----------------------------------------------------------------

    def expected_calls_exact(self) -> int:
        """Total subject calls over all admitted cases (needs no digests)."""
        if not callable(self.calls):
            return self.total_cases() * self.calls
        total = 0
        self.work.walk(self.product)
        bs = self.binders
        names = [b.name for b in bs]
        cols = [b.values(0, b.size) for b in bs]
        sizes = [b.size for b in bs]
        idx = [0] * len(bs)
        env = {n: c[0] for n, c in zip(names, cols)}
        for _ in range(self.product):
            if self.admit(env):
                total += self.calls(env)
            j = len(bs) - 1
            while j >= 0:
                idx[j] += 1
                if idx[j] < sizes[j]:
                    env[names[j]] = cols[j][idx[j]]
                    break
                idx[j] = 0
                env[names[j]] = cols[j][0]
                j -= 1
        return total

    def wraps_exact(self) -> int:
        """Wrapped fixed-width node evaluations over the whole product (section 3.3).

        One dedicated counting pass: ``admit`` for every product tuple and the pure
        trace of the body (expected side, pure subject arguments, guards) for every
        admitted one.  Independent of memoisation, suite order and run depth.
        """
        if self._wraps is not None:
            return self._wraps
        if not self._wraps_needed:
            self._wraps = 0
            return 0
        self.work.walk(self.product)
        stats = self.prog.stats
        bs = self.binders
        names = [b.name for b in bs]
        cols = [b.values(0, b.size) for b in bs]
        sizes = [b.size for b in bs]
        idx = [0] * len(bs)
        env = {n: c[0] for n, c in zip(names, cols)}
        trace = self._trace
        before = stats.wraps
        stats.counting = True
        try:
            for _ in range(self.product):
                if self.admit(env) and trace is not None:
                    trace(env)
                j = len(bs) - 1
                while j >= 0:
                    idx[j] += 1
                    if idx[j] < sizes[j]:
                        env[names[j]] = cols[j][idx[j]]
                        break
                    idx[j] = 0
                    env[names[j]] = cols[j][0]
                    j -= 1
        finally:
            stats.counting = False
        self._wraps = stats.wraps - before
        return self._wraps

    def check_admits(self) -> None:
        """[D19] (VER04r section 6): every admitted case's subject calls are inside the
        callee's ``admit``.

        One dedicated pass over the admitted cases in order, so the first offender is the
        lowest admitted-case ordinal (the address ``<suite id>@<i>``).  It always covers the
        whole product, whatever depth a caller runs: a hole at case 2^20 is a defect of the
        document, not of the slim set.  Not metered (it is part of enumeration, which the
        generator does not charge twice) and skipped when no call is of an op with an
        ``admit``.
        """
        check = self._admit_check
        if check is None or self._admits_checked:
            return
        for i, (_ordinal, env) in enumerate(self.iter_admitted(meter=False)):
            try:
                check(env)
            except WrapsInAdmit as exc:
                raise OracleError(
                    "generation",
                    f"W0701: claim `{self.suite.claim}`: U8 arithmetic wraps while evaluating "
                    f"the admit of op `{exc.op}`; v0 harness evaluates U8 as Nat "
                    f"(VER04r 3.3 rule 2), at admitted case {i}",
                ) from None
            except OutsideAdmit as exc:
                raise OracleError(
                    "generation",
                    f"W0701: claim `{self.suite.claim}` calls op `{exc.op}` with arguments "
                    f"outside the op's admit, at admitted case {i}",
                ) from None
        self._admits_checked = True

    def iter_admitted(self, meter: bool = True):
        """Yield ``(product ordinal, env)`` for every admitted tuple, in product order.

        ``env`` is one dict mutated in place: a consumer evaluates immediately and keeps
        nothing.  This is the walk the witness search of section 5.6 uses.
        """
        if meter:
            self.work.walk(self.product)
        bs = self.binders
        names = [b.name for b in bs]
        cols = [b.values(0, b.size) for b in bs]
        sizes = [b.size for b in bs]
        idx = [0] * len(bs)
        env = {n: c[0] for n, c in zip(names, cols)}
        for ordinal in range(self.product):
            if self.admit(env):
                yield ordinal, env
            j = len(bs) - 1
            while j >= 0:
                idx[j] += 1
                if idx[j] < sizes[j]:
                    env[names[j]] = cols[j][idx[j]]
                    break
                idx[j] = 0
                env[names[j]] = cols[j][0]
                j -= 1

    def run(
        self,
        depth: str = "slim",
        chunks: list[int] | None = None,
        want_v1: bool = True,
        slim_chunks: list[int] | None = None,
        stride: int | str = "auto",
    ) -> SuiteResult:
        """Run the suite at ``depth`` (``"full"`` / ``"slim"``) or on explicit chunks.

        ``slim_chunks`` is the final slim set (base set, witness chunks and profile
        ``slim_chunk`` lines, decided by the caller); without it the set is the base set
        under ``stride`` (a profile ``slim_stride``, clamped per suite).
        """
        m = self.chunks_total
        slim_set = (
            list(slim_chunks) if slim_chunks is not None else slim_base_set(self.suite.id, m, stride)
        )
        if chunks is None:
            chunks = list(range(m)) if depth == "full" else list(slim_set)
        full = chunks == list(range(m))
        whole_in = whole_ex = None
        if full and want_v1 and self.header.startswith(b"warrant-stream-v1"):
            whole_in = hashlib.sha256(self.prefix)
            whole_ex = hashlib.sha256(self.prefix) if self.expected is not None else None
        results = []
        for c in chunks:
            if full and whole_in is not None:
                results.append(
                    self.run_chunk(c, [whole_in], [whole_ex] if whole_ex is not None else None)
                )
            else:
                results.append(self.run_chunk(c))
        cases = self.total_cases()
        out = SuiteResult(
            suite_id=self.suite.id,
            claim=self.suite.claim,
            kind=self.suite.kind,
            binder_types=[str(t) for _, t in self.suite.binders],
            descriptors=[b.desc.to_json() for b in self.binders],
            demoted=list(self.demoted),
            label=max(
                (b.desc.label for b in self.binders),
                key=("full", "bounded-set", "sample").index,
                default="full",
            ),
            product=self.product,
            chunk_size=self.chunk_size,
            chunks_total=m,
            cases=cases,
            discarded=self.product - cases,
            expected_calls=self.expected_calls_exact(),
            wraps=self.wraps_exact(),
            ir_cost=self.ir_cost,
            header=self.header[:-1].decode(),
            chunks=results,
            slim_chunks=list(slim_set),
            result_type=str(self.suite.result),
        )
        if cases == 0:
            raise OracleError("generation", "a suite admits zero cases")
        self.check_admits()
        if self.spec.version == "v0" and out.wraps > 0:
            # Section 3.3 rule 2: the v0 harness evaluates U8 as Nat.
            raise OracleError(
                "generation", "W0701: U8 arithmetic wraps; v0 harness evaluates U8 as Nat"
            )
        if full:
            out.inputs_sha256 = chunk_digest_framing([r.inputs for r in results])
            if self.expected is not None:
                out.expected_sha256 = chunk_digest_framing([r.expected for r in results])
            if whole_in is not None:
                out.v1_inputs_sha256 = whole_in.hexdigest()
                out.v1_expected_sha256 = whole_ex.hexdigest() if whole_ex is not None else None
        by_index = {r.index: r for r in results}
        if all(i in by_index for i in out.slim_chunks):
            chosen = [by_index[i] for i in out.slim_chunks]
        else:
            chosen = [self.run_chunk(i) for i in out.slim_chunks]
        out.slim_inputs_sha256 = chunk_digest_framing([r.inputs for r in chosen])
        if self.expected is not None:
            out.slim_expected_sha256 = chunk_digest_framing([r.expected for r in chosen])
        return out
