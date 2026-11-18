#!/usr/bin/env python3
"""Write (or ``--check``) ``corpus/`` of the synthetic ``omit_example`` agreement fixture (CV2-S10-1).

``omit.warrant`` + ``corpus.profile`` omit three examples, one for each way the oracle decides
one (VER04r 8.4, [D34]): ``echo_big_value`` completes but its argument is 8,192 bytes
(``value-over-limit``, step 5), ``echo_size_error`` hits ``MAX_IR_VALUE`` while evaluating its
argument (``value-over-limit``, step 3) and ``echo_two_sided`` is not ``op(args) == pure``
(``not-op-eq-pure``).  ``echo_small`` stays, so ``cases.tfv`` is not empty.

The oracle has no writer of its own for a corpus directory (the generator writes the other
fixtures), so this script writes one from :func:`derive` and the canonical writer.  The recorded
generator is the generator's current name and version: the generator's slice S10-1 parity PR
regenerates this directory byte for byte from its own output, and this script's ``--check`` (and
``verify-v2`` in ``BUILD``) is what that PR is compared against.

    python3 make_fixture.py            # (re)write corpus/
    python3 make_fixture.py --check    # fail if the committed corpus differs
"""

from __future__ import annotations

import pathlib
import sys

from coding_adventures_warrant_oracle.corpus_v2 import canonical, derive

HERE = pathlib.Path(__file__).resolve().parent
SPEC_REF = "code/packages/python/warrant-oracle/tests/fixtures/omit-example-v2/omit.warrant"
GENERATOR = {"name": "warrant-corpus-gen", "version": "0.5.0"}


def build() -> dict[pathlib.Path, bytes]:
    derived = derive(
        (HERE / "omit.warrant").read_bytes(),
        (HERE / "corpus.profile").read_bytes(),
        spec_ref=SPEC_REF,
        generator=GENERATOR,
    )
    out = {
        HERE / "corpus" / "corpus.json": canonical(derived.corpus),
        HERE / "corpus" / "cases.tfv": derived.tfv,
    }
    for name, text in derived.chunk_files.items():
        out[HERE / "corpus" / "chunks" / name] = text
    return out


def main(argv: list[str]) -> int:
    files = build()
    if argv == ["--check"]:
        stale = [p.name for p, data in files.items() if not p.exists() or p.read_bytes() != data]
        for name in stale:
            print(f"make_fixture: {name} differs from the derived bytes", file=sys.stderr)
        return 1 if stale else 0
    for path, data in files.items():
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
