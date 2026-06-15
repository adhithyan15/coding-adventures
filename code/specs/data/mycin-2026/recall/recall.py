#!/usr/bin/env python3
"""recall.py — relational recall as a binding query (MYCIN-2026 REL-1 prototype).

This is the executable heart of REL-1 (see REL-1-RELATIONAL-RECALL.md): it proves
the *semantics* of fact recall — "which enzyme is deficient in Tay-Sachs?" — as a
single-hop binding query over a grounded knowledge graph, BEFORE we extend the Rust
grammar/engine (staged to REL-2/REL-3). It answers deterministically, on the CPU,
with **zero model calls at answer time** (the warm-path thesis), and every answer
carries a PROOF: the byte-provenanced edge that justifies it and its citation.

THE BIG IDEA (why recall is the same engine as the differential)
----------------------------------------------------------------
A board question "Tay-Sachs is a deficiency of which enzyme?" and a vignette
"Ashkenazi infant, cherry-red macula, regression — which enzyme?" both terminate in
the SAME edge:  deficient_in(tay_sachs, hexosaminidase_a).  The only difference is how
the first argument got bound — STATED (forward recall) vs INFERRED by the differential
(reverse). Recall is the deterministic, single-hop special case of the same continuum
the likelihood-ratio differential lives on. One store, two query tactics.

   FORWARD   ? deficient_in(tay_sachs, $Enzyme)        ⇒ $Enzyme = hexosaminidase_a
   REVERSE   ? disease  ⇒ tay_sachs (differential),  then bind into the SAME goal above

A LOGIC VARIABLE is written with a `$` sigil (`$Enzyme`) — unambiguous against ground
lowercase atoms (`tay_sachs`) and the `?` query lead. A goal argument is either a
ground atom (must match exactly) or a variable (binds to whatever the edge holds).

ABSTENTION is a feature, not a bug. Ask about a disease with no grounded edge and the
store returns NO bindings — UNKNOWN — rather than fabricating an enzyme. That honest
"I don't have a grounded fact for this" is the discriminator vs a hallucinating recall
LLM, and it is what makes the system safe as decision *support* (never replacement).

Usage:
    python3 recall.py                       # run the two worked vignettes (demo)
    python3 recall.py --edges iem-edges.adj # explicit edge file
"""

from __future__ import annotations

import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

HERE = Path(__file__).resolve().parent
DEFAULT_EDGES = HERE / "iem-edges.adj"

# A logic variable is a `$`-prefixed name:  $Enzyme, $Disease, $Pattern.
VAR_RE = re.compile(r"^\$[A-Za-z_][A-Za-z0-9_]*$")

# A `relate <rel>(<arg>, <arg>, ...)` clause header. Arguments are ground atoms
# (lowercase idents) in the stored edges; the parser is intentionally small — it
# reads the SAME surface the REL-2 grammar will lower, so the prototype and the
# native engine never diverge on what an edge looks like.
RELATE_RE = re.compile(r"^\s*relate\s+([a-z_][a-z0-9_]*)\s*\(([^)]*)\)\s*$")
ANNOT_RE = re.compile(r'^\s*(source|locator|trust)\s+(.*)$')


def is_var(tok: str) -> bool:
    """A token is a logic variable iff it carries the `$` sigil."""
    return bool(VAR_RE.match(tok))


@dataclass(frozen=True)
class Edge:
    """One ground relational fact — a typed edge in the knowledge graph, plus the
    provenance that makes it auditable and correctable (the whole point: a recalled
    fact is never a free-floating assertion, it is a citation you can follow)."""

    relation: str
    args: tuple[str, ...]
    source: str | None = None
    trust: str | None = None

    def cite(self) -> str:
        tier = f" [trust {self.trust}]" if self.trust else ""
        return (self.source or "(no citation)") + tier


@dataclass
class Binding:
    """The result of a successful single-hop unification: which variables bound to
    what, and the edge that proves it (a one-node proof DAG for slice-1)."""

    bindings: dict[str, str]
    proof: Edge

    def explain(self) -> str:
        b = ", ".join(f"{k} = {v}" for k, v in self.bindings.items()) or "(ground match)"
        return f"{b}\n      proof: {self.proof.relation}{self.proof.args} — {self.proof.cite()}"


@dataclass
class RelationStore:
    """A set of ground edges + a single-hop binding-query resolver (Datalog over
    facts). REL-3 generalizes this to multi-hop SLD resolution; slice-1 is one hop,
    which is all "which enzyme is deficient" needs."""

    edges: list[Edge] = field(default_factory=list)

    def query(self, relation: str, args: list[str]) -> list[Binding]:
        """Match `relation(args)` against every stored edge. A ground argument must
        match exactly; a `$variable` argument binds to the edge's value. Returns one
        Binding per matching edge (could be several — e.g. a reverse lookup), or an
        EMPTY list = abstention (no grounded fact → UNKNOWN, never a guess)."""
        results: list[Binding] = []
        for e in self.edges:
            if e.relation != relation or len(e.args) != len(args):
                continue
            unified: dict[str, str] = {}
            ok = True
            for goal_arg, edge_arg in zip(args, e.args):
                if is_var(goal_arg):
                    # A variable binds; if it appears twice it must bind consistently.
                    if goal_arg in unified and unified[goal_arg] != edge_arg:
                        ok = False
                        break
                    unified[goal_arg] = edge_arg
                elif goal_arg != edge_arg:
                    ok = False  # ground atom mismatch — this edge does not apply
                    break
            if ok:
                results.append(Binding(bindings=unified, proof=e))
        return results

    def ask(self, relation: str, args: list[str]) -> str:
        """Human-readable answer to a binding query — including honest abstention."""
        hits = self.query(relation, args)
        if not hits:
            return f"  ? {relation}({', '.join(args)})\n  ⇒ UNKNOWN (no grounded edge — abstaining)"
        lines = [f"  ? {relation}({', '.join(args)})"]
        for h in hits:
            lines.append(f"  ⇒ {h.explain()}")
        return "\n".join(lines)


def parse_edges(path: Path) -> RelationStore:
    """Read the `relate` clauses from an `.adj` edge file. The prototype reads the
    same surface syntax REL-2 will lower natively, so there is one source of truth for
    what an edge is. Annotation lines (`source`/`trust`) following a `relate` attach
    to it until the next clause."""
    store = RelationStore()
    pending: Edge | None = None
    src: str | None = None
    trust: str | None = None

    def flush() -> None:
        nonlocal pending, src, trust
        if pending is not None:
            store.edges.append(
                Edge(relation=pending.relation, args=pending.args, source=src, trust=trust)
            )
        pending, src, trust = None, None, None

    for raw in path.read_text().splitlines():
        line = raw.rstrip()
        if not line.strip() or line.lstrip().startswith("%"):
            continue
        m = RELATE_RE.match(line)
        if m:
            flush()  # commit the previous edge before starting a new one
            rel = m.group(1)
            args = tuple(a.strip() for a in m.group(2).split(",") if a.strip())
            pending = Edge(relation=rel, args=args)
            continue
        a = ANNOT_RE.match(line)
        if a and pending is not None:
            kind, rest = a.group(1), a.group(2).strip()
            if kind == "source":
                src = rest.strip().strip('"')
            elif kind == "trust":
                trust = rest.split()[0] if rest else None
            continue
        # Any other line (dictionary/rulebook/use/braces) ends the current edge block.
        flush()
    flush()
    return store


# ---------------------------------------------------------------------------
# Demo: the two worked vignettes from the spec (§4). Both 0 answer-time calls.
# ---------------------------------------------------------------------------

def _demo(store: RelationStore) -> None:
    print("REL-1 relational recall — worked vignettes (0 answer-time model calls)\n")

    print("[1] FORWARD recall — 'Tay-Sachs is a deficiency of which enzyme?'")
    print(store.ask("deficient_in", ["tay_sachs", "$Enzyme"]))
    print()

    print("[2] REVERSE diagnostic→recall — the differential bound `tay_sachs`,")
    print("    now recall its enzyme over the SAME edge:")
    # (In the full pipeline the disease comes from `? disease` over the LR differential;
    #  here we bind the MAP hypothesis the differential would have produced.)
    map_disease = "tay_sachs"
    print(store.ask("deficient_in", [map_disease, "$Enzyme"]))
    print(store.ask("accumulates", [map_disease, "$Substrate"]))
    print(store.ask("inherited_as", [map_disease, "$Pattern"]))
    print()

    print("[3] REVERSE lookup is free — 'which disease lacks hexosaminidase A?'")
    print(store.ask("deficient_in", ["$Disease", "hexosaminidase_a"]))
    print()

    print("[4] ABSTENTION — ask about an ungrounded disease:")
    print(store.ask("deficient_in", ["niemann_pick", "$Enzyme"]))


def main(argv: list[str]) -> int:
    edges = DEFAULT_EDGES
    if "--edges" in argv:
        edges = Path(argv[argv.index("--edges") + 1])
    store = parse_edges(edges)
    _demo(store)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
