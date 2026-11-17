#!/usr/bin/env python3
"""An independent recomputation of claim_sha256 (VOS02 10.1, D-GD7), stdlib only.

Not part of `cargo test`: it is the cross-check the pinned digest table in
`tests/warrant.rs` was produced with, so the pins do not merely echo the Rust code.

    /usr/bin/python3 tests/claim_sha256_reference.py ../vos-sched-contract/warrant/obligations-*.part

prints `<name> <sha256>` for every claim.  The definition: the claim's tokens, taken from
the claim's `claim` keyword to its last token, comments dropped, layout ignored, each token
as its raw source text (a string literal keeps its quotes), joined by single spaces,
hashed with SHA-256.  A claim ends where the next declaration begins: a line indented as
the `claim` line that is not a continuation (these parts are written with one indent).
"""
import hashlib
import re
import sys

TOKEN = re.compile(
    r'"[^"\n]*"'                       # string literal, quotes kept
    r"|[A-Za-z_][A-Za-z0-9_]*"         # word
    r"|[0-9][A-Za-z0-9_]*"             # number with an optional suffix
    r"|==|!=|<=|>=|=>"                 # two-character symbols
    r"|[(),:.<>=+\-*|]"                # one-character symbols
)


def strip_comment(line):
    out, in_str = [], False
    i = 0
    while i < len(line):
        c = line[i]
        if c == '"':
            in_str = not in_str
        if not in_str and line.startswith("--", i):
            break
        out.append(c)
        i += 1
    return "".join(out)


def claims(text):
    cur = None
    for raw in text.split("\n"):
        line = strip_comment(raw)
        if not line.strip():
            continue
        indent = len(line) - len(line.lstrip(" "))
        if indent == 2 and line.lstrip().startswith("claim "):
            if cur:
                yield cur
            cur = [line]
        elif indent == 2:  # another declaration
            if cur:
                yield cur
            cur = None
        elif cur is not None:
            cur.append(line)
    if cur:
        yield cur


def main(paths):
    for path in paths:
        for lines in claims(open(path).read()):
            toks = TOKEN.findall("\n".join(lines))
            name = toks[1]
            print(name, hashlib.sha256(" ".join(toks).encode()).hexdigest())


if __name__ == "__main__":
    main(sys.argv[1:])
