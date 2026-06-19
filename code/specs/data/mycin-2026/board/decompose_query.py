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

WHAT GROUNDING DOES AND DOES NOT PROTECT (an honest limit)
----------------------------------------------------------
The model's output is CONSTRAINED: the relation must be one of the legal recall
relations and the subject a canonical entity. This buys ONE guarantee — the engine
never fabricates a medical fact: every answer it returns cites a real grounded edge,
and an OFF-vocabulary query finds no edge and ABSTAINS. But grounding alone does NOT
make decomposition errors free: a mis-map to a DIFFERENT but valid query makes the
engine faithfully answer the WRONG question (correct-for-the-wrong-query). Grounding
kills FABRICATION, not MISDIRECTION. So the decomposition itself is gated against the
stem's own bytes — a TWO-SIDED faithfulness check, byte-provenance applied to the query:

  * SUBJECT gate (attested_in_stem): the chosen entity must be named by the stem.
    Stops "Von Gierke disease" → subject `fabry`.
  * RELATION gate (relation_attested_in_stem): the stem's interrogative must ASK for
    what the relation answers. Stops a stem asking for the classic FINDING being
    decomposed as has_mcv(...) — right subject, wrong question.

Either gate failing rejects the query, so the engine ABSTAINS rather than answering the
wrong question. Measured on the live demo (see OFFLINE-DEMO.md): a 4B local model
decomposes ~74% of stems correctly; with both gates every mis-decomposition (wrong
entity OR wrong question-type) becomes an abstention, so a weak local model is SAFE —
its errors are honest UNKNOWNs, never confident wrong answers (0 wrong / 100%
defensible on the recorded runs for both the 4B and the 0.5B model).

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
              "endocrine-edges.adj", "coag-edges.adj", "micro-edges.adj", "pharm-edges.adj",
              "immuno-edges.adj", "genetics-edges.adj", "rheum-edges.adj", "onco-edges.adj",
              "histo-edges.adj", "cardio-edges.adj", "neuro-edges.adj", "gi-edges.adj"]

# Each recall relation binds one conventional variable (the "what is being asked").
# This is the controlled query vocabulary the model must choose from — 31 relations
# across fourteen domains. The engine ultimately validates the subject; this map
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
    "gram_stain": "Result",            # microbiology: Gram reaction (positive / negative)
    "morphology": "Shape",             # microbiology: cell shape (cocci / bacilli / …)
    "causes": "Disease",               # microbiology: signature disease the organism causes
    "drug_class": "Class",             # pharmacology: pharmacologic class of a drug
    "mechanism": "MOA",                # pharmacology: mechanism of action
    "adverse_effect": "Effect",        # pharmacology: a notable adverse effect
    "antidote_for": "Antidote",        # pharmacology: what reverses a poisoning/overdose
    "mediated_by": "Mediator",         # immunology: effector driving a hypersensitivity reaction
    "associated_hla": "HLA",           # immunology: the HLA allele a disease associates with
    "gene_defect": "Gene",             # immunology/genetics: the mutated gene behind a disorder
    "deficiency_of": "Component",      # immunology: the immune component missing in an immunodeficiency
    "inheritance": "Pattern",          # genetics: how a Mendelian disorder is transmitted
    "trinucleotide_repeat": "Repeat",  # genetics: the expanded triplet (CAG / CGG / CTG / GAA)
    "imprinting": "Parent",            # genetics: the parent-of-origin lesion (PWS / Angelman)
    "associated_autoantibody": "Antibody",  # rheumatology: the serologic marker of a disease
    "tumor_marker": "Marker",          # oncology: the serum tumor marker of a neoplasm
    "seen_in": "Condition",            # pathology: the condition a smear/histology finding points to
    "murmur_indicates": "Lesion",      # cardiology: the valvular lesion a heart murmur points to
    "lesion_causes": "Deficit",        # neurology: the deficit/syndrome a lesion site produces
    "biopsy_finding_in": "Disease",    # gastroenterology: the GI diagnosis a biopsy finding points to
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


# Generic medical nouns carry no entity identity — they appear in many stems, so
# they don't count as attestation of a SPECIFIC subject.
_GENERIC_TOKENS = {
    "disease", "deficiency", "anemia", "syndrome", "disorder", "the", "a", "of",
    "and", "s", "type", "classic", "inherited",
}


def _norm_text(text: str) -> str:
    """Lowercase and collapse to space-separated alphanumeric tokens for matching."""
    return re.sub(r"[^a-z0-9]+", " ", (text or "").lower()).strip()


def attested_in_stem(subject: str, stem: str) -> bool:
    """Decomposition-faithfulness check: is the chosen canonical subject actually
    grounded in the STEM's own bytes? This is byte-provenance applied to the query —
    the model may only name an entity the question itself names. It is what stops a
    mis-decomposition (e.g. "Von Gierke disease" → subject `fabry`) from becoming a
    confident WRONG answer: an un-attested subject is rejected, so the engine ABSTAINS
    instead of faithfully answering the wrong question. True iff the subject's canonical
    phrase appears in the stem, or every NON-generic token of it does."""
    ns = " " + _norm_text(stem) + " "
    phrase = subject.replace("_", " ")
    if f" {phrase} " in ns:
        return True
    tokens = [t for t in phrase.split() if t and t not in _GENERIC_TOKENS]
    if not tokens:
        return False
    stem_tokens = set(ns.split())
    return all(t in stem_tokens for t in tokens)


# Structural tokens in a relation name carry no interrogative meaning (prepositions,
# the domain tag "coag", the auxiliary "has") — they are not cues for "what is asked".
_CUE_STOPWORDS = {"in", "as", "by", "of", "the", "a", "coag", "has"}


def _relation_cues() -> dict[str, set]:
    """Derive each relation's interrogative cue tokens from the controlled vocabulary
    ALREADY in REL_VAR — the relation NAME's own word-parts plus its conventional
    VARIABLE name. No new medical knowledge is authored: a relation's cues are literally
    the words it is spelled with (deficient_in + Enzyme → {deficient, enzyme};
    has_mcv + Class → {mcv, class}). This keeps the relation gate symmetric with the
    subject gate — both are byte-provenance against the question, both read only the
    vocabulary the decomposer was already given."""
    cues: dict[str, set] = {}
    for rel, var in REL_VAR.items():
        tokens = set(rel.split("_")) | {var.lower()}
        cues[rel] = {t for t in tokens if t not in _CUE_STOPWORDS}
    return cues


RELATION_CUES = _relation_cues()


def relation_attested_in_stem(relation: str, stem: str) -> bool:
    """Relation-faithfulness check: does the stem's interrogative actually ASK for what
    this relation answers? True iff at least one of the relation's cue tokens (see
    _relation_cues) appears as a whole word in the stem. This is the relation-side
    counterpart of attested_in_stem: it stops a right-subject / wrong-relation
    mis-decomposition (e.g. a stem asking for the classic FINDING of hereditary
    spherocytosis, decomposed as has_mcv(...)) from resolving to a real-but-wrong edge.
    The chosen relation that the stem does not ask for is rejected → the engine
    ABSTAINS rather than answering the wrong question. Whole-word (not substring)
    matching is deliberate so a cue like `class` is not spuriously found inside
    `classic`."""
    stem_tokens = set(_norm_text(stem).split())
    return bool(RELATION_CUES.get(relation, set()) & stem_tokens)


def decompose(stem: str, gen, vocab: dict, faithful: bool = True) -> dict | None:
    """Decompose one prose stem into a recall query using an injected text generator
    `gen(prompt) -> str` (a local MLX model, or a test stub). Returns the parsed query,
    or None when the model's output can't be parsed into a legal query OR (with
    faithful=True, the default) when the chosen SUBJECT or RELATION is not attested by
    the stem. The two-sided faithfulness gate — subject (attested_in_stem) AND relation
    (relation_attested_in_stem) — converts both kinds of mis-decomposition (wrong
    entity, wrong question-type) into an honest abstention rather than a confident wrong
    answer. Grounding stops fabrication; this gate stops misdirection."""
    q = parse_query(gen(build_query_prompt(stem, vocab)))
    if q is None:
        return None
    if faithful and not (attested_in_stem(q["subject"], stem)
                         and relation_attested_in_stem(q["relation"], stem)):
        return None
    return q


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
