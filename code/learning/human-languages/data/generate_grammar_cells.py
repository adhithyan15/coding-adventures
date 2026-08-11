#!/usr/bin/env python3
"""Generate the universal grammar-slot inventory and Spanish's filling of it.

HL10 section 5 -- THE UNIT IS THE CELL, NOT THE PARADIGM.

A *cell* is one filled slot in one paradigm: Spanish `hablo` is a cell.
`PRESENT-INDICATIVE-CONJ1` -- the whole six-form table -- is not a teachable
unit; it is six.

This distinction is the whole reason the file exists. Every language textbook
ever written opens a tense with its full six-form grid, which is the single
steepest step in language pedagogy: six new forms, one new concept, no
retrieval, and an implicit claim that the learner will absorb them by staring.
HL10 forbids it (`maxNewGrammarCellsPerLesson: 1`, and no paradigm table until
every cell in it has been taught individually), and a rule like that is only
enforceable if the cells are enumerated somewhere. They are enumerated here.

WHY TWO FILES

HL10 section 4 makes GRAMMAR a *universal slot inventory with local filling*,
because that is what lets the other 21 tracks reuse this work:

  core/grammar-slots.json      language-neutral. Names roles most languages
                               have some version of -- "perfective past, first
                               person singular, conjugation class 1". Never
                               names a Spanish form. A new track answers "do
                               you have this?" rather than designing a syllabus
                               from nothing.

  spanish/grammar-cells.json   Spanish's answers. CONJ1 is -ar, the perfective
                               past is the preterite, and here is the ordering
                               a learner climbs.

A track that lacks a slot declares an omission rather than leaving a hole, the
same contract `curriculum.json` already uses for spine nodes.

THE ORDERING IS THE POINT

`prerequisites` is what makes this a ramp rather than a list. The rules, each
one a pedagogical claim that can be argued with:

  1. Singular before plural, one person at a time. The chain runs
     1SG -> 2SG -> 3SG -> 1PL -> 2PL -> 3PL. This is why "the present tense"
     takes fourteen chapters instead of one.
  2. Conjugation 1 before 2 before 3, at the same person and tense. The learner
     meets -ar, then -er, then -ir, never two at once.
  3. Tenses in acquisition order, anchored at 1SG-CONJ1:
     present -> preterite -> imperfect -> future -> conditional.
     Preterite before imperfect because it is the one a beginner needs to tell
     a story; imperfect after it because the *contrast* is the real lesson and
     you cannot contrast with something you do not have.
  4. The present subjunctive hangs off the present indicative 1SG, because that
     is where its stem actually comes from (`tengo` -> `tenga`). This is why
     HL10 section 5.4 puts the -go verbs before the subjunctive: the rule is
     load-bearing, not decorative.
  5. Compounds require the past participle AND the auxiliary's own finite cell.
     A learner cannot say "I have spoken" before they can say "I have".
  6. Non-finite forms are the roots: the infinitive depends on nothing, and the
     gerund and participle depend on it.

Rule 3's anchoring deserves a note. Only the 1SG-CONJ1 cell of a tense carries
the cross-tense edge. Hanging every cell of the preterite off every cell of the
present would be true but useless -- it would say "learn all of the present
before any of the preterite", which forbids the interleaving that makes the
ramp gentle.

Run:  python3 code/learning/human-languages/data/generate_grammar_cells.py
Check that the committed files match:  ... --check
"""

from __future__ import annotations

import argparse
import collections
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent

# --- dimensions -------------------------------------------------------------

PERSONS = [
    ("1SG", "first person singular"),
    ("2SG", "second person singular"),
    ("3SG", "third person singular"),
    ("1PL", "first person plural"),
    ("2PL", "second person plural"),
    ("3PL", "third person plural"),
]
PERSON_ORDER = [p for p, _ in PERSONS]

# Acquisition order, not traditional grammar-book order. See rule 3 above.
FINITE = [
    ("IND", "PRES", "indicative", "present"),
    ("IND", "PRET", "indicative", "perfective past"),
    ("IND", "IMPF", "indicative", "imperfective past"),
    ("IND", "FUT", "indicative", "future"),
    ("IND", "COND", "indicative", "conditional"),
    ("SBJ", "PRES", "subjunctive", "present"),
    ("SBJ", "IMPF", "subjunctive", "imperfective past"),
    ("SBJ", "FUT", "subjunctive", "future"),
]
TENSE_CHAIN = ["PRES", "PRET", "IMPF", "FUT", "COND"]

# No first-person-singular imperative: you do not command yourself.
IMPERATIVE_PERSONS = ["2SG", "3SG", "1PL", "2PL", "3PL"]

COMPOUND = [
    ("PERF-PRES", "present perfect", "IND", "PRES"),
    ("PERF-PAST", "pluperfect", "IND", "IMPF"),
    ("PERF-PRET", "preterite perfect", "IND", "PRET"),
    ("PERF-FUT", "future perfect", "IND", "FUT"),
    ("PERF-COND", "conditional perfect", "IND", "COND"),
    ("PERF-SBJ-PRES", "present perfect subjunctive", "SBJ", "PRES"),
    ("PERF-SBJ-PAST", "pluperfect subjunctive", "SBJ", "IMPF"),
    ("PERF-SBJ-FUT", "future perfect subjunctive", "SBJ", "FUT"),
]
NONFINITE = [("INF", "infinitive"), ("GER", "gerund / present participle"), ("PART", "past participle")]
CONJ = ["CONJ1", "CONJ2", "CONJ3"]

# Spanish's filling of the universal classes.
ES_CONJ = {"CONJ1": "-ar", "CONJ2": "-er", "CONJ3": "-ir"}
ES_TENSE_NAME = {
    ("IND", "PRES"): "presente de indicativo",
    ("IND", "PRET"): "preterito indefinido",
    ("IND", "IMPF"): "preterito imperfecto",
    ("IND", "FUT"): "futuro simple",
    ("IND", "COND"): "condicional simple",
    ("SBJ", "PRES"): "presente de subjuntivo",
    ("SBJ", "IMPF"): "imperfecto de subjuntivo",
    ("SBJ", "FUT"): "futuro de subjuntivo",
}
# The one slot Spanish fills only receptively. Recorded as data so the gate can
# tell "not taught yet" from "deliberately never produced".
ES_RECEPTIVE_ONLY = {("SBJ", "FUT")}


def finite_slot(mood: str, tense: str, person: str, conj: str) -> str:
    return f"SLOT-{mood}-{tense}-{person}-{conj}"


def build_slots() -> list[dict]:
    """The universal inventory. Nothing here may name a Spanish form."""
    slots: list[dict] = []
    for mood, tense, mood_g, tense_g in FINITE:
        for person, person_g in PERSONS:
            for conj in CONJ:
                slots.append(
                    collections.OrderedDict(
                        id=finite_slot(mood, tense, person, conj),
                        kind="finite",
                        mood=mood,
                        tense=tense,
                        person=person,
                        conjugation=conj,
                        gloss=f"{mood_g} {tense_g}, {person_g}, conjugation class {conj[-1]}",
                    )
                )
    for person, person_g in PERSONS:
        if person not in IMPERATIVE_PERSONS:
            continue
        for polarity in ["AFF", "NEG"]:
            for conj in CONJ:
                word = "affirmative" if polarity == "AFF" else "negative"
                slots.append(
                    collections.OrderedDict(
                        id=f"SLOT-IMP-{polarity}-{person}-{conj}",
                        kind="imperative",
                        mood="IMP",
                        polarity=polarity,
                        person=person,
                        conjugation=conj,
                        gloss=f"{word} command, {person_g}, conjugation class {conj[-1]}",
                    )
                )
    for tense, tense_g, _aux_mood, _aux_tense in COMPOUND:
        for person, person_g in PERSONS:
            slots.append(
                collections.OrderedDict(
                    id=f"SLOT-{tense}-{person}",
                    kind="compound",
                    tense=tense,
                    person=person,
                    gloss=f"{tense_g}, {person_g} (auxiliary plus participle, so conjugation-independent)",
                )
            )
    for form, form_g in NONFINITE:
        for conj in CONJ:
            slots.append(
                collections.OrderedDict(
                    id=f"SLOT-{form}-{conj}",
                    kind="non-finite",
                    form=form,
                    conjugation=conj,
                    gloss=f"{form_g}, conjugation class {conj[-1]}",
                )
            )
    return slots


def prerequisites_for(slot: dict) -> list[str]:
    """The ordering rules, applied. See the module docstring for the reasoning."""
    kind = slot["kind"]

    if kind == "non-finite":
        # Rule 6: the infinitive is a root; the other two hang off it.
        if slot["form"] == "INF":
            return []
        return [f"SLOT-INF-{slot['conjugation']}"]

    if kind == "compound":
        # Rule 5: participle plus the auxiliary's own finite cell. Spanish's
        # auxiliary (haber) is CONJ2, which is why the edge names CONJ2.
        tense = slot["tense"]
        aux_mood, aux_tense = next((m, t) for k, _g, m, t in COMPOUND if k == tense)
        return [
            "SLOT-PART-CONJ1",
            finite_slot(aux_mood, aux_tense, slot["person"], "CONJ2"),
        ]

    if kind == "imperative":
        person, conj, polarity = slot["person"], slot["conjugation"], slot["polarity"]
        # The negative command is built on the present subjunctive -- which is
        # how the subjunctive first reaches a learner, before it is ever named.
        if polarity == "NEG":
            return [f"SLOT-IMP-AFF-{person}-{conj}", finite_slot("SBJ", "PRES", person, conj)]
        index = IMPERATIVE_PERSONS.index(person)
        if index > 0:
            return [f"SLOT-IMP-AFF-{IMPERATIVE_PERSONS[index - 1]}-{conj}"]
        # The first command form needs the indicative it is carved from.
        return [finite_slot("IND", "PRES", "3SG", conj)]

    # finite
    mood, tense, person, conj = slot["mood"], slot["tense"], slot["person"], slot["conjugation"]
    prereqs: list[str] = []

    index = PERSON_ORDER.index(person)
    if index > 0:
        # Rule 1: one person at a time.
        prereqs.append(finite_slot(mood, tense, PERSON_ORDER[index - 1], conj))
    else:
        # Rule 2: at 1SG, the earlier conjugation of the same tense.
        c = CONJ.index(conj)
        if c > 0:
            prereqs.append(finite_slot(mood, tense, person, CONJ[c - 1]))
        else:
            # Rule 3/4: the anchor cell carries the cross-tense edge, and only it.
            if mood == "IND":
                t = TENSE_CHAIN.index(tense)
                if t > 0:
                    prereqs.append(finite_slot("IND", TENSE_CHAIN[t - 1], "1SG", "CONJ1"))
            elif mood == "SBJ" and tense == "PRES":
                # Rule 4: the present subjunctive stem comes from the present
                # indicative 1SG. tengo -> tenga.
                prereqs.append(finite_slot("IND", "PRES", "1SG", "CONJ1"))
            elif mood == "SBJ" and tense == "IMPF":
                # Formed from the third person plural preterite.
                prereqs.append(finite_slot("IND", "PRET", "3PL", "CONJ1"))
            elif mood == "SBJ" and tense == "FUT":
                prereqs.append(finite_slot("SBJ", "IMPF", "1SG", "CONJ1"))
    return prereqs


def build_spanish_cells(slots: list[dict]) -> list[dict]:
    cells: list[dict] = []
    for slot in slots:
        conj = slot.get("conjugation")
        cell = collections.OrderedDict(
            id="ES-CELL-" + slot["id"][len("SLOT-") :],
            slot=slot["id"],
            prerequisites=["ES-CELL-" + p[len("SLOT-") :] for p in prerequisites_for(slot)],
        )
        if conj:
            cell["conjugationEnding"] = ES_CONJ[conj]
        key = (slot.get("mood"), slot.get("tense"))
        if key in ES_TENSE_NAME:
            cell["spanishName"] = ES_TENSE_NAME[key]
        if key in ES_RECEPTIVE_ONLY:
            cell["productive"] = False
            cell["receptiveOnlyBecause"] = (
                "survives in legal and proverbial register only; a learner must "
                "recognise it and is never asked to produce it"
            )
        cells.append(cell)
    return cells


def check_dag(cells: list[dict]) -> None:
    """No cell may depend on one that does not exist, and there may be no cycle."""
    ids = {c["id"] for c in cells}
    for cell in cells:
        for prereq in cell["prerequisites"]:
            if prereq not in ids:
                raise SystemExit(f"dangling prerequisite: {cell['id']} -> {prereq}")

    # Kahn's algorithm. A cycle here would mean a cell can never be reached,
    # which is a curriculum that cannot be taught in any order at all.
    incoming = {c["id"]: len(c["prerequisites"]) for c in cells}
    dependents: dict[str, list[str]] = collections.defaultdict(list)
    for cell in cells:
        for prereq in cell["prerequisites"]:
            dependents[prereq].append(cell["id"])
    queue = collections.deque(sorted(i for i, n in incoming.items() if n == 0))
    seen = 0
    while queue:
        node = queue.popleft()
        seen += 1
        for dep in dependents[node]:
            incoming[dep] -= 1
            if incoming[dep] == 0:
                queue.append(dep)
    if seen != len(cells):
        stuck = sorted(i for i, n in incoming.items() if n > 0)
        raise SystemExit(f"cycle in the cell DAG, {len(stuck)} cells unreachable, e.g. {stuck[:5]}")


# --- irregular and stem-changing overlays (HL-C91) ---------------------------
#
# An OVERLAY is one verb's one cell that deviates from the regular pattern:
# `tengo` overlays SLOT-IND-PRES-1SG-CONJ2. It is kept in a separate list from
# the regular cells for a reason that matters pedagogically -- the learner does
# not meet "the irregular verbs" as a category. They meet the regular row, and
# then meet the one verb that breaks it, in frequency order, one cell at a time.
#
# So every overlay's prerequisite is the REGULAR cell it deviates from. The DAG
# gains depth rather than breadth: nothing becomes reachable earlier, and the
# irregular is always taught against a pattern the learner already holds.

# The four stem-change patterns. Singular plus third-plural change; the two
# plural persons keep the regular stem -- the shape traditionally drawn as a
# boot, and the reason these are four cells per verb rather than six.
STEM_CHANGE_PERSONS = ["1SG", "2SG", "3SG", "3PL"]
STEM_CHANGERS = {
    "e-ie": ["pensar", "querer", "sentir", "empezar", "entender", "preferir",
             "cerrar", "perder", "mentir"],
    "o-ue": ["poder", "dormir", "contar", "volver", "morir", "encontrar",
             "recordar", "mover"],
    "e-i": ["pedir", "servir", "repetir", "seguir", "vestir"],
    "u-ue": ["jugar"],
}
STEM_CHANGE_CONJ = {
    "pensar": "CONJ1", "querer": "CONJ2", "sentir": "CONJ3", "empezar": "CONJ1",
    "entender": "CONJ2", "preferir": "CONJ3", "cerrar": "CONJ1", "perder": "CONJ2",
    "mentir": "CONJ3", "poder": "CONJ2", "dormir": "CONJ3", "contar": "CONJ1",
    "volver": "CONJ2", "morir": "CONJ3", "encontrar": "CONJ1", "recordar": "CONJ1",
    "mover": "CONJ2", "pedir": "CONJ3", "servir": "CONJ3", "repetir": "CONJ3",
    "seguir": "CONJ3", "vestir": "CONJ3", "jugar": "CONJ1",
}

# The -go club: irregular in the first person singular of the present ONLY.
# One cell each, and the club is why HL10 section 5.4 puts these before the
# subjunctive -- the subjunctive stem is built from exactly this form.
GO_VERBS = {"tener": "CONJ2", "poner": "CONJ2", "salir": "CONJ3", "venir": "CONJ3",
            "hacer": "CONJ2", "decir": "CONJ3", "traer": "CONJ2", "oir": "CONJ3",
            "caer": "CONJ2", "valer": "CONJ2"}

# Strong preterites: irregular across all six persons, and unstressed in the
# first and third singular, which is the audible signature of the whole set.
# Ordered by frequency, because that is the order they are taught in.
STRONG_PRETERITES = {"ser-ir": "CONJ2", "hacer": "CONJ2", "decir": "CONJ3",
                     "tener": "CONJ2", "estar": "CONJ1", "poder": "CONJ2",
                     "poner": "CONJ2", "saber": "CONJ2", "querer": "CONJ2",
                     "venir": "CONJ3", "traer": "CONJ2", "dar": "CONJ1",
                     "ver": "CONJ2", "andar": "CONJ1", "haber": "CONJ2"}

# The imperfect has exactly three irregular verbs in the whole language, which
# is why HL10 section 5.4 places it immediately after the preterite as a rest.
IMPERFECT_IRREGULARS = {"ser": "CONJ2", "ir": "CONJ3", "ver": "CONJ2"}

# Irregular past participles: one cell each, and each one is also a word the
# learner will meet as an adjective.
IRREGULAR_PARTICIPLES = {"abrir": "CONJ3", "decir": "CONJ3", "escribir": "CONJ3",
                         "hacer": "CONJ2", "morir": "CONJ3", "poner": "CONJ2",
                         "romper": "CONJ2", "ver": "CONJ2", "volver": "CONJ2",
                         "cubrir": "CONJ3", "resolver": "CONJ2", "freir": "CONJ3"}

# Shortened future/conditional stems. One stem serves BOTH tenses -- the single
# weld HL10 section 5.4 rung 22 calls "one weld, twice" -- so a verb here owns
# twelve cells but only one thing to learn.
SHORT_STEM_VERBS = {"tener": "CONJ2", "poner": "CONJ2", "salir": "CONJ3",
                    "venir": "CONJ3", "poder": "CONJ2", "saber": "CONJ2",
                    "hacer": "CONJ2", "decir": "CONJ3", "querer": "CONJ2",
                    "haber": "CONJ2", "valer": "CONJ2", "caber": "CONJ2"}

# Present subjunctive irregulars: the six whose stem is NOT the present-tense
# first person, so the rule that carries every other verb does not reach them.
SUBJUNCTIVE_IRREGULARS = {"ser": "CONJ2", "ir": "CONJ3", "saber": "CONJ2",
                          "haber": "CONJ2", "dar": "CONJ1", "estar": "CONJ1"}


def overlay(verb: str, kind: str, slot_suffix: str, conj: str, note: str) -> dict:
    """One verb's one deviating cell, hung off the regular cell it breaks."""
    regular = f"ES-CELL-{slot_suffix}"
    return collections.OrderedDict(
        id=f"ES-OVERLAY-{verb.upper().replace('-', '')}-{slot_suffix}",
        verb=verb,
        kind=kind,
        conjugation=conj,
        deviatesFrom=regular,
        prerequisites=[regular],
        note=note,
    )


def build_overlays() -> list[dict]:
    out: list[dict] = []
    for pattern, verbs in STEM_CHANGERS.items():
        for verb in verbs:
            conj = STEM_CHANGE_CONJ[verb]
            for person in STEM_CHANGE_PERSONS:
                out.append(overlay(
                    verb, f"stem-change-{pattern}", f"IND-PRES-{person}-{conj}", conj,
                    f"stem vowel {pattern.replace('-', ' to ')} under stress; "
                    f"the two plural persons keep the regular stem"))
    for verb, conj in GO_VERBS.items():
        out.append(overlay(verb, "go-club", f"IND-PRES-1SG-{conj}", conj,
                           "first person singular only; the form the present subjunctive is built from"))
    for verb, conj in STRONG_PRETERITES.items():
        for person in PERSON_ORDER:
            out.append(overlay(verb, "strong-preterite", f"IND-PRET-{person}-{conj}", conj,
                               "irregular stem across all six persons, unstressed in 1SG and 3SG"))
    for verb, conj in IMPERFECT_IRREGULARS.items():
        for person in PERSON_ORDER:
            out.append(overlay(verb, "irregular-imperfect", f"IND-IMPF-{person}-{conj}", conj,
                               "one of only three irregular imperfects in the language"))
    for verb, conj in IRREGULAR_PARTICIPLES.items():
        out.append(overlay(verb, "irregular-participle", f"PART-{conj}", conj,
                           "irregular past participle; also met as an adjective"))
    for verb, conj in SHORT_STEM_VERBS.items():
        for tense in ("FUT", "COND"):
            for person in PERSON_ORDER:
                out.append(overlay(verb, "short-stem", f"IND-{tense}-{person}-{conj}", conj,
                                   "shortened stem, shared by the future and the conditional"))
    for verb, conj in SUBJUNCTIVE_IRREGULARS.items():
        for person in PERSON_ORDER:
            out.append(overlay(verb, "irregular-subjunctive", f"SBJ-PRES-{person}-{conj}", conj,
                               "stem is not the present first person, so the general rule does not reach it"))
    return out


def check_overlays(overlays: list[dict], cells: list[dict]) -> None:
    """Every overlay must deviate from a regular cell that actually exists."""
    ids = {c["id"] for c in cells}
    seen: set[str] = set()
    for item in overlays:
        if item["deviatesFrom"] not in ids:
            raise SystemExit(
                f"overlay {item['id']} deviates from {item['deviatesFrom']}, which is not a regular cell")
        if item["id"] in seen:
            raise SystemExit(f"duplicate overlay id: {item['id']}")
        seen.add(item["id"])


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true", help="fail if the committed files differ")
    args = parser.parse_args()

    slots = build_slots()
    if len(slots) != len({s["id"] for s in slots}):
        raise SystemExit("duplicate slot id")
    cells = build_spanish_cells(slots)
    check_dag(cells)
    overlays = build_overlays()
    check_overlays(overlays, cells)

    kinds = collections.Counter(s["kind"] for s in slots)
    slot_doc = collections.OrderedDict(
        version=1,
        note=(
            "HL10 section 5.1. The universal grammar-slot inventory: one entry per "
            "individually teachable cell of a verb paradigm. Language-neutral by "
            "contract -- no id or gloss here may name a form from any particular "
            "language, because the other 21 tracks fill these same slots. A track "
            "without a slot declares an omission rather than leaving a hole. "
            "Generated by data/generate_grammar_cells.py; do not hand-edit."
        ),
        counts=collections.OrderedDict(sorted(kinds.items())) | {"total": len(slots)},
        dimensions=collections.OrderedDict(
            persons=PERSON_ORDER,
            conjugationClasses=CONJ,
            finiteMoodTense=[f"{m}-{t}" for m, t, _, _ in FINITE],
            imperativePersons=IMPERATIVE_PERSONS,
            compoundTenses=[t for t, _, _, _ in COMPOUND],
            nonFinite=[f for f, _ in NONFINITE],
        ),
        slots=slots,
    )
    cells_doc = collections.OrderedDict(
        version=1,
        language="spanish",
        note=(
            "HL10 section 5. Spanish's filling of core/grammar-slots.json, with the "
            "ordering a learner climbs. `prerequisites` is what makes this a ramp "
            "rather than a list: singular before plural one person at a time, -ar "
            "before -er before -ir, tenses in acquisition order anchored at the "
            "1SG conjugation-1 cell, the present subjunctive hanging off the present "
            "indicative 1SG where its stem actually comes from, and compounds "
            "requiring both the participle and the auxiliary's own finite cell. "
            "`cells` is the REGULAR inventory. `overlays` holds the irregular and "
            "stem-changing forms, kept separate because a learner never meets \"the "
            "irregular verbs\" as a category -- they meet the regular row, then the one "
            "verb that breaks it, one cell at a time. Every overlay hangs off the regular "
            "cell it deviates from, so the DAG gains depth and nothing becomes reachable "
            "earlier. Generated; do not hand-edit."
        ),
        conjugationClasses=collections.OrderedDict(sorted(ES_CONJ.items())),
        counts=collections.OrderedDict(
            regularCells=len(cells),
            productive=sum(1 for c in cells if c.get("productive") is not False),
            receptiveOnly=sum(1 for c in cells if c.get("productive") is False),
            overlayCells=len(overlays),
            overlaysByKind=collections.OrderedDict(
                sorted(collections.Counter(o["kind"] for o in overlays).items())
            ),
            totalCells=len(cells) + len(overlays),
        ),
        cells=cells,
        overlays=overlays,
    )

    targets = [
        (ROOT / "core" / "grammar-slots.json", slot_doc),
        (ROOT / "spanish" / "grammar-cells.json", cells_doc),
    ]
    drift = False
    for path, doc in targets:
        text = json.dumps(doc, indent=2, ensure_ascii=False) + "\n"
        if args.check:
            current = path.read_text(encoding="utf-8") if path.exists() else ""
            if current != text:
                print(f"DRIFT: {path.relative_to(ROOT)} differs from the generator output")
                drift = True
        else:
            path.write_text(text, encoding="utf-8")
            print(f"wrote {path.relative_to(ROOT)}")
    if drift:
        return 1
    if not args.check:
        print(f"slots {len(slots)} ({dict(kinds)}), spanish regular {len(cells)}, "
              f"overlays {len(overlays)}, total {len(cells) + len(overlays)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
