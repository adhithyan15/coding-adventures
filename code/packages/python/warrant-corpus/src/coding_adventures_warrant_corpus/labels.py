"""Honest coverage labels for exhaustive suites (backlog LP-012).

The corpus calls every enumerated suite ``kind: "exhaustive"``, and VER04i's
manifest has no room for a finer word.  But "exhaustive" over ``Bytes[347]``
is a statement about 347 chosen byte strings, not about every byte string, and
adding such totals together hides that (LP-012: the same conflation inflates
the Rust figures).  So each suite also gets a label *derived from its domain
descriptors*, never typed by hand:

``full``         the descriptor enumerates **every** value of its type
                 (``bool``, ``u8_all``, ``Bytes<0>``, ``Bytes<1>``);
``bounded-set``  a fixed, deterministic set of boundary and structured values
                 that is a strict subset of the type (``u64_edge``,
                 ``bytes_d1``, ``bytes_n(N)`` for ``N >= 2``, ``nat_0_64``);
``sample``       pseudo-random or otherwise unstructured draws.  No portable
                 descriptor produces one yet; the label is reserved so a
                 future sampled suite cannot be mistaken for a bounded set.

A suite is ``full`` only if *every* binder is ``full``; otherwise it is as weak
as its weakest binder (``sample`` over ``bounded-set`` over ``full``).

The labels live in a sidecar, ``evidence/domain-labels.json``, because the
``warrant-lane-evidence-v1`` rows are a closed key set that the Rust validator
(WD-004a) rejects additions to.
"""

from __future__ import annotations

import re

from .corpus import Corpus
from .errors import CorpusError

SCHEMA = "warrant-lane-domain-labels-v1"
NOTE = (
    "Suite-level 'exhaustive' counts conflate full, bounded-set and sample "
    "domains until backlog LP-012 lands; use cases_by_label, not the "
    "manifest's exhaustive total, as the headline."
)
_RANK = {"full": 0, "bounded-set": 1, "sample": 2}
_BYTES_N = re.compile(r"bytes_n\(([0-9]+)\)")


def domain_label(descriptor: str) -> str:
    """The label of one domain descriptor (see the module docstring)."""
    if descriptor in ("bool", "u8_all"):
        return "full"
    if descriptor in ("u64_edge", "bytes_d1", "nat_0_64"):
        return "bounded-set"
    match = _BYTES_N.fullmatch(descriptor)
    if match:
        # Bytes<0> has one value and Bytes<1> has 256: both enumerate the type.
        return "full" if int(match.group(1)) <= 1 else "bounded-set"
    raise CorpusError("unsupported-domain", "no coverage label for this descriptor")


def derive_labels(
    corpus: Corpus, package: str, module: str, manifest_sha256: str
) -> dict[str, object]:
    """The label document for ``corpus``, bound to one manifest by digest."""
    suites = []
    by_label: dict[str, int] = {}
    for suite in corpus.suites:
        binders = [
            {"name": name, "domain": descriptor, "label": domain_label(descriptor)}
            for name, descriptor in suite.binders
        ]
        label = max((b["label"] for b in binders), key=lambda value: _RANK[str(value)])
        by_label[str(label)] = by_label.get(str(label), 0) + suite.cases
        suites.append(
            {
                "id": suite.id,
                "label": label,
                "domain": suite.domain_text,
                "cases": suite.cases,
                "binders": binders,
            }
        )
    return {
        "schema": SCHEMA,
        "package": package,
        "module": module,
        "manifest_sha256": manifest_sha256,
        "note": NOTE,
        "cases_by_label": dict(sorted(by_label.items())),
        "suites": sorted(suites, key=lambda row: str(row["id"])),
    }
