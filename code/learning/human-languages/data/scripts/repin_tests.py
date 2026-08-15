#!/usr/bin/env python3
"""HL-C166 — re-pin the corpus counters after a content change.

WHY THIS FILE EXISTS
--------------------
Adding lessons moves roughly a dozen exact-value pins across the
`human-language-data` suite: lesson counts, atom counts, root-ledger totals,
chapter counts, the modality manifest, the metalanguage term table. None of them
is a judgement call -- the corpus grew, the numbers grow with it -- so re-pinning
is mechanical, and mechanical work done by hand at 3am is how the two bugs below
got shipped.

It had been retyped from scratch every session. Twice in one PR that reintroduced
a DIFFERENT bug each time, so it lives here now.

THE TWO BUGS THIS FILE IS WRITTEN AROUND
-----------------------------------------
1. **Modifying the lines and never writing the file.** A version that built a
   patched `src` list and forgot `open(path, "w").write(...)` in one branch
   re-ran the entire vitest suite, unchanged, until it was killed. It looked like
   a hang. It was a missing write.

   Guard: every branch here writes before it prints, and prints what it wrote.

2. **Putting the annotation BEFORE the trailing comma.** The pins carry long
   explanatory comments, so the natural edit is `<number> // note`. Applied to

       totalLessons: 2266,

   that yields

       totalLessons: 2266 // HL-C166,

   which comments the comma out and makes the file unparseable. The suite then
   reports FAILED SUITES rather than failed tests -- and a `Tests 900 passed`
   line still looks green if you read the count instead of the exit code.

   Guard: the substitution captures the optional comma and re-emits it BEFORE
   the comment. See `PIN_FIELD` below.

Both bugs share a shape worth naming: they are invisible to a test *count* and
visible only to an exit *code*. That is why the loop below branches on
`returncode` and never on parsed output.

THE THREE PIN SHAPES IN THIS CORPUS
------------------------------------
    expect(x).toBe(2266)                        -> a bare scalar
    expect(x).toMatchObject({ lessonCount: 419 })-> an object field
    { term: "verb", lessons: 948 },              -> a metalanguage term row

vitest truncates long objects in its message (`{ language: 'spanish', …(7) }`),
so object pins are read from the +/- DIFF block rather than the message, which is
why `object_fields()` parses `- "key": N` / `+ "key": N` lines.

WHAT IT WILL NOT DO
-------------------
It re-pins COUNTS. It will not touch a CEILING (`toBeLessThanOrEqual`) or a
RATIO, because those failing means something regressed and the answer is to fix
the content -- see HL-C167. If the loop stops on one of those it prints STUCK and
leaves it for a human, which is the correct outcome.

USAGE
-----
    cd code/packages/typescript/human-language-data
    python3 ../../../learning/human-languages/data/scripts/repin_tests.py \
        " // HL-C999: +4 -- what this tranche added"

The tag is appended to every line it edits, so the diff explains itself.
Add `--dry-run` to see the first round's edits without writing.
"""

import os
import re
import subprocess
import sys

MAX_ROUNDS = 60

# Every path this script writes comes from a regex match on vitest's own output,
# and that output CONTAINS CORPUS CONTENT: these tests assert over loaded
# curriculum objects, so lesson text is printed into the +/- diff. A path is
# therefore data, not a trusted fact, and `tests/` as a prefix is not
# containment -- `tests/../../../etc/hosts` starts with it too. Everything below
# is confined to real files under tests/ before a single byte is written.
TEST_ROOT = os.path.realpath("tests")

# `.ts` under tests/, no traversal expressible: no `..`, no absolute path.
# Anchored to the start of a line so a marker quoted inside a diff value cannot
# be mistaken for a stack frame.
FRAME = re.compile(r"(?m)^\s*❯ (tests/[\w./-]+\.test\.ts):(\d+)")

# A `key: <old>` pin. Group 2 is the OPTIONAL TRAILING COMMA, and it is re-emitted
# before the comment -- this is bug 2 above, and the comma is why this regex is
# not the obvious one.
PIN_FIELD = r'(\n\s*"?\'?{key}\'?"?:\s*){old}(,?)'

TERMS = ("verb", "noun", "tense", "pronoun", "regular", "article", "adjective")


def safe_test_path(path):
    """Resolve inside tests/ or refuse. Symlinks are resolved before the check."""
    full = os.path.realpath(path)
    if ".." in path.split("/"):
        return None
    if os.path.commonpath([full, TEST_ROOT]) != TEST_ROOT:
        return None
    return full if os.path.isfile(full) else None


def write_atomic(path, text):
    """Write via a temp file and os.replace, so a failed write cannot truncate
    a test file that was fine a moment ago."""
    tmp = path + ".repin.tmp"
    with open(tmp, "w", encoding="utf-8") as fh:
        fh.write(text)
    os.replace(tmp, path)


def find_frame(block):
    """The (path, line) of a failure, confined to tests/. None if unusable."""
    m = FRAME.search(block)
    if not m:
        return None
    path = safe_test_path(m.group(1))
    return (path, int(m.group(2))) if path else None


def object_fields(out):
    """Changed numeric fields, read from the +/- diff rather than the message.

    vitest truncates the inline object, so the message alone cannot be trusted:
    `expected { language: 'spanish', …(7) } to match object { … }` carries no
    numbers at all.
    """
    seg = out[:6000]
    expected = dict(re.findall(r'-\s+"?([\w-]+)"?:\s*(\d+)', seg))
    actual = dict(re.findall(r'\+\s+"?([\w-]+)"?:\s*(\d+)', seg))
    return {k: (expected[k], v) for k, v in actual.items()
            if k in expected and expected[k] != v}


def patch_object(path, fields, tag, dry):
    """Rewrite each changed field. Returns the list of edits made."""
    text = open(path, encoding="utf-8").read()
    edits = []
    for key, (old, new) in fields.items():
        pattern = PIN_FIELD.format(key=re.escape(key), old=old)
        # A CALLABLE replacement: a tag containing \1 or \g<1> would otherwise be
        # interpreted as a backreference and splice captured text into the file.
        after = re.sub(pattern,
                       lambda m: "%s%s%s%s" % (m.group(1), new, m.group(2), tag),
                       text, count=1)
        if after != text:
            text = after
            edits.append("%s %s->%s" % (key, old, new))
            continue
        # A metalanguage row: { term: "verb", lessons: N }. Its field name is
        # `lessons` for every term, so the number is what identifies the row.
        for term in TERMS:
            row = '{ term: "%s", lessons: %s }' % (term, old)
            if row in text:
                text = text.replace(row, '{ term: "%s", lessons: %s }' % (term, new), 1)
                edits.append("%s %s->%s" % (term, old, new))
                break
    if edits and not dry:
        write_atomic(path, text)
    return edits


def patch_scalar(path, line, want, got, tag, dry):
    """Rewrite a bare `toBe(N)` / `toHaveLength(N)`.

    The reported line is usually right, but a long chained assertion can report
    the closing line instead, so a bounded window around it is searched too.
    """
    src = open(path, encoding="utf-8").read().split("\n")
    window = [line - 1] + list(range(max(0, line - 40), min(len(src), line + 10)))
    for j in window:
        if "(%s)" % want in src[j]:
            src[j] = src[j].replace("(%s)" % want, "(%s)" % got, 1) + tag
            if not dry:
                write_atomic(path, "\n".join(src))
            return "%s:%d %s->%s" % (path.split("/")[-1], j + 1, want, got)
    return None


def main():
    dry = "--dry-run" in sys.argv
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    tag = args[0] if args else " // repin"

    for _ in range(MAX_ROUNDS):
        # --no-install: never let a missing local vitest become a registry
        # download that then executes.
        run = subprocess.run(["npx", "--no-install", "vitest", "run"],
                             capture_output=True, text=True)
        out = run.stdout + run.stderr
        # The exit code, never a parsed count: an unparseable suite runs zero
        # tests and still prints a green-looking total.
        if run.returncode == 0:
            print("ALL GREEN")
            return 0

        # Split into per-failure blocks FIRST, then search each one. The
        # previous shape -- `(?:.*?\n)*?.*?❯` scanning the whole blob -- rescanned
        # the tail once per candidate and went quadratic: measured 0.035s at 67KB
        # of output, 0.52s at 268KB, and this loop runs up to MAX_ROUNDS times.
        blocks = out.split("AssertionError")[1:]
        obj = next((b for b in blocks
                    if re.match(r": expected .*? to (?:deeply equal|match object)", b)
                    and find_frame(b)), None)
        if obj:
            path, line = find_frame(obj)
            fields = object_fields(obj)
            edits = patch_object(path, fields, tag, dry)
            if not edits:
                print("UNPATCHED %s:%d %s" % (path, line, fields))
                return 1
            print("  %s %s" % (path.split("/")[-1], ", ".join(edits)))
            if dry:
                return 0
            continue

        scalar, sblock = None, None
        for b in blocks:
            scalar = (re.match(r": expected (?:\[[^\n]*?\]|\d+) to "
                               r"(?:have a length of|be) (\d+)[^\n]*?but got (\d+)", b)
                      or re.match(r": expected (\d+) to be (\d+)\b", b))
            if scalar and find_frame(b):
                sblock = b
                break
            scalar = None
        if not scalar:
            # A ceiling or a ratio. Deliberately not automated -- see the header.
            print("STUCK -- not a count pin. Fix the content, not the number:")
            print("\n".join(l for l in out.split("\n")
                            if "FAIL " in l or "AssertionError" in l)[:600])
            return 1

        groups = scalar.groups()
        want, got = (groups[0], groups[1]) if "length" in scalar.group(0) \
            else (groups[1], groups[0])
        fpath, fline = find_frame(sblock)
        done = patch_scalar(fpath, fline, want, got, tag, dry)
        if not done:
            print("CANNOT PATCH %s:%d %s->%s" % (fpath, fline, want, got))
            return 1
        print("  %s" % done)
        if dry:
            return 0

    print("did not converge in %d rounds" % MAX_ROUNDS)
    return 1


if __name__ == "__main__":
    sys.exit(main())
