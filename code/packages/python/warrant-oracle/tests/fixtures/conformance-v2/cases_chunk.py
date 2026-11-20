"""``v2-chunk``: validate a chunk file, then enumerate one chunk alone by unranking (slice S5a table).

A case is self-contained: ``corpus`` (the parts of a ``corpus.json`` a suite needs), ``suite`` (one
complete suite row exactly as the oracle writes it), ``chunk_file_hex`` (the suite's chunk file) and
``chunk``.  The suites come from two small ``ir.core`` documents whose profiles declare **no fault**
(so ``slim_chunks`` is the base set and nothing depends on a witness search):

``kit`` (``kit.warrant``, default chunk size 65,536)
    ``big``        ``forall a b c: U8`` against a model: 2^24 ordinals, 256 chunks, every case admitted
    ``short``      a predicate suite (no ``expected_sha256``) with a refinement and a conditional call
    ``negs``       ``forall a: I8``: the ``warrant-stream-v2`` header at ``ir.core`` and a wrap
    ``res``        a binder of type ``Result<Bool,E>`` (the ``result`` descriptor)
    ``two_bytes``  two ``bytes_d1`` binders: 120,409 ordinals, so the last chunk is short
``kit-small`` (``kit-small.warrant``, ``chunk_size 1024``: a lane uses the recorded value)
    ``grid``    ``forall a b: U8``: 64 chunks;  ``ragged``  a ``Nat`` and a ``U8`` binder: 17 chunks, the last
    short, and ``m > 16`` so the slim set is a stride;  ``gap``  the refinement ``a < 2 or a > 253``
    discards a whole run of chunks, so chunk boundaries fall inside a discarded run

The tampers write the *negative* cases: each changes one thing in a valid case (and re-seals the file
digest where the point is another check), and the oracle's own validator must name the code.  The
intents are the numbers the layouts above imply, written by hand: chunk ``c`` of an all-admitted
suite has ``chunk_size`` cases and ``first_admitted = c * chunk_size``; in ``gap`` an ``a``-value is a
quarter chunk (256 ordinals), so chunk 0 holds ``a = 0, 1`` (512 cases), chunks 1..62 hold nothing and
chunk 63 holds ``a = 254, 255`` after 512 earlier cases.
"""

from __future__ import annotations

import copy
import hashlib

from kit_common import row

from coding_adventures_warrant_oracle.suites import chunk_digest_framing

FULL = 65536


def _parts(kit, claim, chunk):
    return {
        "corpus": copy.deepcopy(kit.corpus_part()),
        "suite": copy.deepcopy(kit.row(claim)),
        "file": kit.chunk_file(claim),
        "chunk": chunk,
    }


def _lines(parts):
    return parts["file"].decode("ascii").split("\n")[:-1]


def _set_lines(parts, lines, *, seal=True, final_lf=True):
    parts["file"] = ("\n".join(lines) + ("\n" if final_lf else "")).encode("ascii")
    if seal:
        parts["suite"]["chunks_sha256"] = hashlib.sha256(parts["file"]).hexdigest()


def _edit_line(parts, index, column, fn):
    lines = _lines(parts)
    cols = lines[index].split(" ")
    cols[column] = fn(cols[column])
    lines[index] = " ".join(cols)
    _set_lines(parts, lines)


def _flip_hex(text):
    return ("0" if text[0] != "0" else "1") + text[1:]


# -- tampers: each takes the parts of a valid case and changes one thing -------------------------------------------
def first_admitted_plus_one(p):
    _edit_line(p, 5, 2, lambda t: str(int(t) + 1))


def first_admitted_of_chunk_zero(p):
    _edit_line(p, 0, 2, lambda t: "1")


def drop_last_line(p):
    _set_lines(p, _lines(p)[:-1])


def add_a_line(p):
    lines = _lines(p)
    last = lines[-1].split(" ")
    last[0] = str(len(lines))
    _set_lines(p, lines + [" ".join(last)])


def cases_plus_one(p):
    _edit_line(p, 3, 1, lambda t: str(int(t) + 1))


def swap_two_indices(p):
    lines = _lines(p)
    a, b = lines[3].split(" "), lines[4].split(" ")
    a[0], b[0] = b[0], a[0]
    lines[3], lines[4] = " ".join(a), " ".join(b)
    _set_lines(p, lines)


def inputs_digest_flipped(p):
    _edit_line(p, 7, 4, _flip_hex)


def expected_digest_flipped(p):
    _edit_line(p, 7, 5, _flip_hex)


def drop_expected_column(p):
    lines = _lines(p)
    lines[2] = " ".join(lines[2].split(" ")[:5])
    _set_lines(p, lines)


def add_expected_column(p):
    lines = _lines(p)
    lines[0] = lines[0] + " " + "ab" * 32
    _set_lines(p, lines)


def crlf(p):
    _set_lines(p, [line + "\r" for line in _lines(p)])


def no_final_lf(p):
    _set_lines(p, _lines(p), final_lf=False)


def leading_zero_count(p):
    _edit_line(p, 2, 1, lambda t: "0" + t)


def double_space(p):
    lines = _lines(p)
    lines[2] = lines[2].replace(" ", "  ", 1)
    _set_lines(p, lines)


def uppercase_hex(p):
    _edit_line(p, 2, 4, lambda t: t.upper() if t.upper() != t else t)


def short_hex(p):
    _edit_line(p, 2, 4, lambda t: t[:-1])


def empty_file(p):
    p["file"] = b""
    p["suite"]["chunks_sha256"] = hashlib.sha256(b"").hexdigest()


def not_ascii(p):
    p["file"] = p["file"][:-1] + "é\n".encode()
    p["suite"]["chunks_sha256"] = hashlib.sha256(p["file"]).hexdigest()


def chunks_sha256_flipped(p):
    p["suite"]["chunks_sha256"] = _flip_hex(p["suite"]["chunks_sha256"])


def row_cases_plus_one(p):
    p["suite"]["cases"] += 1


def row_inputs_sha256_flipped(p):
    p["suite"]["inputs_sha256"] = _flip_hex(p["suite"]["inputs_sha256"])


def row_expected_sha256_flipped(p):
    p["suite"]["expected_sha256"] = _flip_hex(p["suite"]["expected_sha256"])


def corpus_drop(key):
    def tamper(p):
        del p["corpus"][key]

    return tamper


def corpus_add_key(p):
    p["corpus"]["fuel"] = 1


def corpus_set(key, value):
    def tamper(p):
        p["corpus"][key] = value

    return tamper


def row_drop(key):
    def tamper(p):
        del p["suite"][key]

    return tamper


def row_add_key(p):
    p["suite"]["note"] = "x"


def descriptor_size_wrong(p):
    p["suite"]["binders"][0][1]["domain"]["size"] += 1


def product_plus_one(p):
    # discarded is bumped too (it is product - cases), so the product is the only thing that is wrong
    p["suite"]["product"] += 1
    p["suite"]["discarded"] += 1


def chunk_is(value):
    def tamper(p):
        p["chunk"] = value

    return tamper


def _reseal_slim(p):
    """Recompute the row's slim digests for its (tampered) ``slim_chunks``, so only the list itself is wrong.

    Each is the framing of the per-chunk digests of the listed chunks, read from the chunk file; an index
    past the end of the file has no digest and contributes none.
    """
    cols = [line.split(" ") for line in _lines(p)]
    listed = [cols[i] for i in p["suite"]["slim_chunks"] if i < len(cols)]
    p["suite"]["slim_inputs_sha256"] = chunk_digest_framing([bytes.fromhex(c[4]) for c in listed])
    if "slim_expected_sha256" in p["suite"]:
        p["suite"]["slim_expected_sha256"] = chunk_digest_framing([bytes.fromhex(c[5]) for c in listed])


def slim(value):
    def tamper(p):
        p["suite"]["slim_chunks"] = value
        _reseal_slim(p)

    return tamper


def _slim_drop(p, fn):
    p["suite"]["slim_chunks"] = fn(list(p["suite"]["slim_chunks"]))
    _reseal_slim(p)


def _slim_with_extras(n):
    """The base slim set plus extra chunks (the lowest not already in it) until it has ``n`` entries."""

    def tamper(p):
        chosen = set(p["suite"]["slim_chunks"])
        extra = (i for i in range(len(_lines(p))) if i not in chosen)
        while len(chosen) < n:
            chosen.add(next(extra))
        slim(sorted(chosen))(p)

    return tamper


def build_rows(kits):
    """The ``v2-chunk`` rows; ``kits`` maps ``"kit"`` and ``"small"`` to the derived :class:`Kit` objects."""
    rows = []

    def add(slug, summary, kit, claim, chunk, intent, tamper=None, also_sequential=False):
        parts = _parts(kits[kit], claim, chunk)
        if tamper is not None:
            tamper(parts)
        fields = {
            "corpus": parts["corpus"],
            "suite": parts["suite"],
            "chunk_file_hex": parts["file"].hex(),
            "chunk": parts["chunk"],
            "_kit": kit,
        }
        if also_sequential:
            fields["also_sequential"] = True
        rows.append(row(slug, summary, fields, intent))

    def ok(cases, first, calls=None):
        return {"cases": cases, "first_admitted": first, "calls": cases if calls is None else calls}

    # -- valid cases: the first, middle and last chunk of a 2^24 suite --------------------------------------------
    for slug, c, what in (("first", 0, "first"), ("middle", 128, "middle"), ("last", 255, "last")):
        add(f"big-{slug}-chunk", f"the {what} chunk of a 2^24-ordinal suite, enumerated alone", "kit", "big", c,
            ok(FULL, c * FULL), also_sequential=True)
    add("short-predicate-chunk", "a predicate suite has no expected_sha256 and a calls count that is not cases", "kit",
        "short", 0, ok(65280, 0, 64770))
    add("negs-signed-chunk", "a signed binder under the warrant-stream-v2 header", "kit", "negs", 0, ok(256, 0))
    add("res-result-binder-chunk", "a binder of type Result<Bool,E>: the result descriptor", "kit", "res", 0, ok(4, 0))
    add("two-bytes-first-chunk", "two bytes_d1 binders: a full first chunk", "kit", "two_bytes", 0, ok(FULL, 0))
    add("two-bytes-short-last-chunk", "the last chunk of a product that is not a multiple of the chunk size", "kit",
        "two_bytes", 1, ok(120409 - FULL, FULL), also_sequential=True)

    # -- a recorded chunk_size of 1024 is used, never the default ---------------------------------------------------
    add("grid-first-chunk-1024", "chunk_size 1024: 64 chunks of 1,024 cases", "small", "grid", 0, ok(1024, 0))
    add("grid-last-chunk-1024", "the last of 64 chunks starts at 63 * 1,024", "small", "grid", 63, ok(1024, 64512),
        also_sequential=True)
    add("ragged-first-chunk", "a Nat and a U8 binder at chunk_size 1024: 17 chunks", "small", "ragged", 0, ok(1024, 0))
    add("ragged-short-last-chunk", "16,640 ordinals leave a last chunk of 256", "small", "ragged", 16, ok(256, 16384),
        also_sequential=True)

    # -- zero admitted cases, and boundaries inside a refinement-discarded run -----------------------------------------
    add("gap-first-chunk-is-half-admitted", "a = 0 and 1 are admitted, a = 2 and 3 are discarded: half a chunk", "small",
        "gap", 0, ok(512, 0), also_sequential=True)
    add("gap-chunk-with-zero-admitted-cases", "a chunk whose every ordinal is discarded: cases 0, first_admitted 512",
        "small", "gap", 1, ok(0, 512), also_sequential=True)
    add("gap-chunk-inside-the-discarded-run", "a chunk in the middle of the discarded run", "small", "gap", 30, ok(0, 512))
    add("gap-last-discarded-chunk", "the last chunk of the discarded run still reports first_admitted 512", "small", "gap",
        62, ok(0, 512))
    add("gap-chunk-after-the-discarded-run", "the run ends: a = 254 and 255 resume at first_admitted 512", "small", "gap",
        63, ok(512, 512), also_sequential=True)

    # -- chunk-file: the file fails the grammar of section 5.5 or a cross-check ----------------------------------------
    S, R = "small", "ragged"
    for slug, summary, tamper in (
        ("first-admitted-off-by-one", "first_admitted of one chunk is one too large", first_admitted_plus_one),
        ("first-admitted-of-the-first-chunk", "the first chunk's first_admitted must be 0", first_admitted_of_chunk_zero),
        ("missing-line", "one chunk line is missing (m - 1 lines)", drop_last_line),
        ("extra-line", "one line too many (m + 1 lines)", add_a_line),
        ("cases-sum-wrong", "one chunk's cases is one too large, so the running sum and the total are off", cases_plus_one),
        ("index-out-of-sequence", "two chunk indices are swapped", swap_two_indices),
        ("inputs-digest-wrong", "one chunk's inputs digest is altered (the file digest is re-sealed)", inputs_digest_flipped),
        ("expected-digest-wrong", "one chunk's expected digest is altered (re-sealed)", expected_digest_flipped),
        ("expected-column-missing", "a reference suite line without its expected digest", drop_expected_column),
        ("crlf-line-endings", "CR LF line endings", crlf),
        ("no-final-lf", "the last line has no LF", no_final_lf),
        ("leading-zero-count", "a count with a leading zero", leading_zero_count),
        ("double-space", "two spaces between fields", double_space),
        ("short-digest", "a 63-digit digest", short_hex),
        ("empty-file", "an empty file", empty_file),
        ("not-ascii", "a non-ASCII byte", not_ascii),
        ("chunks-sha256-wrong", "chunks_sha256 is not the digest of the file", chunks_sha256_flipped),
        ("row-cases-wrong", "the row's cases is not the sum of the lines", row_cases_plus_one),
        ("row-inputs-sha256-wrong", "the suite digest is not the framing of the chunk digests", row_inputs_sha256_flipped),
        ("row-expected-sha256-wrong", "the expected suite digest is not the framing of the chunk digests",
         row_expected_sha256_flipped),
    ):
        add(f"chunk-file-{slug}", summary, S, R, 3, {"error": "chunk-file"}, tamper)
    add("chunk-file-uppercase-hex", "upper-case hex in a digest", S, R, 3, {"error": "chunk-file"}, uppercase_hex)
    add("chunk-file-predicate-with-expected", "a predicate suite line carries an expected digest", "kit", "short", 0,
        {"error": "chunk-file"}, add_expected_column)
    add("chunk-file-off-by-one-two-chunks", "first_admitted of the last chunk one too small", "kit", "two_bytes", 1,
        {"error": "chunk-file"}, lambda p: _edit_line(p, 1, 2, lambda t: str(int(t) - 1)))
    add("chunk-file-gap-counted-on-product-ordinals", "first_admitted counted on product ordinals, not admitted cases",
        S, "gap", 63, {"error": "chunk-file"}, lambda p: _edit_line(p, 63, 2, lambda t: str(63 * 1024)))

    # -- slim-chunks: the row's slim_chunks list ---------------------------------------------------------------------------
    for slug, summary, tamper in (
        ("omits-the-first-chunk", "slim_chunks omits chunk 0", lambda p: _slim_drop(p, lambda s: s[1:])),
        ("omits-the-last-chunk", "slim_chunks omits chunk m-1", lambda p: _slim_drop(p, lambda s: s[:-1])),
        ("not-ascending", "slim_chunks is not ascending", slim([0, 8, 4, 16])),
        ("duplicate", "slim_chunks repeats an index", slim([0, 4, 4, 16])),
        ("index-past-m", "slim_chunks holds an index >= m", slim([0, 4, 16, 17])),
        ("empty", "slim_chunks is empty", slim([])),
        ("only-the-first", "slim_chunks omits the last chunk of a suite with m > 1", slim([0])),
    ):
        add(f"slim-chunks-{slug}", summary, S, R, 3, {"error": "slim-chunks"}, tamper)
    add("slim-chunks-more-than-32-entries", "a 256-chunk suite with 33 slim chunks", "kit", "big", 0,
        {"error": "slim-chunks"}, _slim_with_extras(33))
    add("slim-chunks-exactly-32-entries-is-valid", "32 slim chunks are allowed", "kit", "big", 0, ok(FULL, 0),
        _slim_with_extras(32))

    # -- corpus: corpus or suite is refused for another reason -----------------------------------------------------
    for slug, summary, tamper in (
        ("missing-key", "the corpus part lacks a key", corpus_drop("models")),
        ("unknown-key", "the corpus part has an unknown key", corpus_add_key),
        ("chunk-size-not-a-power-of-two", "chunk_size 1000", corpus_set("chunk_size", 1000)),
        ("chunk-size-below-2-10", "chunk_size 512 is below 2^10", corpus_set("chunk_size", 512)),
        ("chunk-size-above-2-16", "chunk_size 131072 is above 2^16", corpus_set("chunk_size", 131072)),
        ("max-ir-cost-above-the-ceiling", "max_ir_cost 65537", corpus_set("max_ir_cost", 65537)),
        ("row-unknown-key", "the suite row has an unknown key", row_add_key),
        ("row-missing-key", "the suite row lacks a key", row_drop("wraps")),
        ("row-missing-expected-sha256", "a reference suite row without expected_sha256", row_drop("expected_sha256")),
        ("wrong-descriptor-size", "a binder descriptor whose size is not the recomputed one", descriptor_size_wrong),
        ("wrong-product", "product is not the product of the descriptor sizes", product_plus_one),
        ("chunk-equals-m", "chunk >= m: the first chunk past the end", chunk_is(17)),
        ("chunk-far-past-m", "chunk far past m", chunk_is(4096)),
    ):
        add(f"corpus-{slug}", summary, S, R, 3, {"error": "corpus"}, tamper)
    return rows
