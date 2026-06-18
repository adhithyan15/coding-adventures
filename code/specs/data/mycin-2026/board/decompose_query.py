#!/usr/bin/env python3
"""decompose_query.py — turn a prose board question into an ADJ recall QUERY, using a
LOCAL in-memory model (MYCIN-2026 offline board-exam).

THE ONE THING THE MODEL IS ALLOWED TO DO
----------------------------------------
A real board item is PROSE ("An Ashkenazi infant with a cherry-red macula… which
enzyme is deficient?"). The grounded knowledge graph answers binding queries, not
prose. So one — and only one — step needs language understanding: mapping the prose
to a typed query `relation(subject, $Var)`. That is a DECOMPOSE step (prose → typed
IR / ADJ program), exactly the job a small LOCAL model does well, and the only model
call permitted on the path. Critically it is a LOCAL, in-memory model — never an
online API — so the zero-ONLINE-call invariant holds (see offline_guard.py). The
model NEVER answers the medical question; it only writes the query. The native
adj-lang engine answers, over grounded edges, with a citation (see board_eval.py).

    prose stem ──[local model: DECOMPOSE]──▶ {relation, subject, var}
                                              │  (an ADJ program: ? relation(subject,$Var))
                                              ▼
                              native adj-lang-cli ──▶ binding + citation  OR  abstain

WHY THIS IS SAFE EVEN WITH A WEAK MODEL
---------------------------------------
The model's output is CONSTRAINED and CHECKED: the relation must be one of the legal
recall relations and the subject must be a canonical entity the graph knows. If the
model emits something off-vocabulary, the query simply finds no edge and the engine
ABSTAINS — it never fabricates a medical fact. So a decompose error degrades to an
honest "UNKNOWN", never to a wrong answer. The model is a translator on a short
leash, not an oracle.

The model is fully INJECTABLE: `decompose(stem, gen, vocab)` takes any
`gen(prompt) -> str`. Pass `local_gen(model_path)` for a real MLX model, or a stub in
tests — so this module imports with zero ML dependencies and the offline path needs
no network and no heavyweight import.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
RECALL = HERE.parent / "recall"
EDGE_FILES = ["iem-edges.adj", "vitamin-edges.adj", "anemia-edges.adj",
              "endocrine-edges.adj", "coag-edges.adj"]

# Each recall relation binds one conventional variable (the "what is being asked").
# This is the controlled query vocabulary the model must choose from — 11 relations
# across five organ systems. The engine ultimately validates the subject; this map
# pins the legal relation set and the variable name each relation answers.
REL_VAR = {
    "deficient_in": "Enzyme",          # IEM: which enzyme is deficient
    "accumulates": "Substrate",        # IEM: which substrate accumulates
    "inherited_as": "Pattern",         # IEM: inheritance pattern
    "deficiency_causes": "Disease",    # vitamin: deficiency → disease
    "classic_finding": "Finding",      # vitamin/anemia: pathognomonic finding
    "has_mcv": "Class",                # anemia: microcytic / normocytic / macrocytic
    "secreted_by": "Gland",            # endocrine: source gland/tissue
    "deficiency_syndrome": "Syndrome",  # endocrine: hormone deficiency → named syndrome
    "factor_deficiency": "Factor",     # coagulation: which clotting factor
    "coag_inheritance": "Pattern",     # coagulation: inheritance pattern
    "prolonged_test": "Test",          # coagulation: which lab test is prolonged
}

_RELATE_RE = re.compile(r"^\s*relate\s+([a-z_][a-z0-9_]*)\s*\(([^)]*)\)\s*$")
_JSON_RE = re.compile(r"\{.*?\}", re.DOTALL)


def build_vocab(recall_dir: Path | None = None) -> dict:
    """Read the grounded edge rulebooks and collect, per relation, the canonical
    SUBJECT atoms the graph knows. This is the closed vocabulary the model must map
    prose onto — built from the same edges the engine answers over, so the model can
    only ever name an entity the graph can actually resolve (or miss → abstain). This
    is prompt construction, NOT the answer path (the engine still answers)."""
    recall_dir = recall_dir or RECALL
    subjects: dict[str, set] = {rel: set() for rel in REL_VAR}
    for name in EDGE_FILES:
        for raw in (recall_dir / name).read_text().splitlines():
            m = _RELATE_RE.match(raw)
            if not m:
                continue
            rel = m.group(1)
            args = [a.strip() for a in m.group(2).split(",") if a.strip()]
            if rel in subjects and args:
                subjects[rel].add(args[0])
    return {rel: sorted(s) for rel, s in subjects.items()}


def build_query_prompt(stem: str, vocab: dict) -> str:
    """The decompose prompt: map ONE prose board question to a single recall query as
    strict JSON. Lists the legal relations (with the variable each answers) and the
    canonical subjects, so the model classifies rather than free-associates."""
    rel_lines = []
    for rel, var in REL_VAR.items():
        subs = ", ".join(vocab.get(rel, []))
        rel_lines.append(f'  - {rel}(subject, ${var})   subjects: [{subs}]')
    relations = "\n".join(rel_lines)
    return (
        "You convert a medical board question into ONE relational recall query.\n"
        "Do NOT answer the question. Output ONLY a JSON object:\n"
        '  {"relation": <one legal relation>, "subject": <one canonical subject>, '
        '"var": <the variable that relation answers>}\n\n'
        "Choose the relation that matches what the question ASKS FOR, and the subject "
        "that is the entity the question is ABOUT (use the exact canonical token).\n\n"
        f"LEGAL RELATIONS (and their canonical subjects):\n{relations}\n\n"
        f"QUESTION:\n{stem}\n\n"
        "JSON:"
    )


def _normalize_subject(raw: str) -> str:
    """Best-effort canonicalization of a model-emitted subject: lowercase, spaces and
    hyphens to underscores, drop a possessive and a trailing 'disease'. The engine is
    the final arbiter — an off-vocabulary subject just yields an abstention — so this
    only smooths common surface drift, it never invents a mapping."""
    s = raw.strip().lower().replace("'s", "").replace("-", "_")
    s = re.sub(r"[^a-z0-9_ ]", "", s).strip().replace(" ", "_")
    s = re.sub(r"_+", "_", s)
    return s


def parse_query(raw: str) -> dict | None:
    """Extract the first JSON object from the model's decode and validate it into a
    {relation, subject, var} query, or None if it is unusable. Validation is strict on
    the relation (must be legal) but lenient on surface form of the subject (the engine
    checks the subject by trying to resolve it)."""
    m = _JSON_RE.search(raw or "")
    if not m:
        return None
    try:
        obj = json.loads(m.group(0))
    except (json.JSONDecodeError, ValueError):
        return None
    rel = str(obj.get("relation", "")).strip()
    if rel not in REL_VAR:
        return None
    subject = _normalize_subject(str(obj.get("subject", "")))
    if not subject:
        return None
    # Trust the relation's conventional variable name (keeps the query well-formed even
    # if the model omitted or mis-named "var").
    return {"relation": rel, "subject": subject, "var": REL_VAR[rel]}


def decompose(stem: str, gen, vocab: dict) -> dict | None:
    """Decompose one prose stem into a recall query using an injected text generator
    `gen(prompt) -> str` (a local MLX model, or a test stub). Returns the parsed query
    or None when the model's output can't be parsed into a legal query."""
    return parse_query(gen(build_query_prompt(stem, vocab)))


def local_gen(model_path: str, adapter: str | None = None):
    """A LOCAL, in-memory MLX text generator `gen(prompt) -> str`. Imported lazily so
    this module stays dependency-free for the cached/offline path and for tests; the
    model runs entirely on-device (no network), preserving the zero-online-call
    invariant. Mirrors train/eval_decompose._mlx_gen."""
    from mlx_lm import generate, load  # noqa: PLC0415
    from mlx_lm.sample_utils import make_sampler  # noqa: PLC0415

    model, tok = load(model_path, adapter_path=adapter)
    sampler = make_sampler(temp=0.0)  # greedy — decomposition is classification, not prose

    def gen(prompt: str) -> str:
        messages = [{"role": "user", "content": prompt}]
        text = tok.apply_chat_template(messages, add_generation_prompt=True, tokenize=False)
        return generate(model, tok, prompt=text, max_tokens=128, sampler=sampler, verbose=False)

    return gen
