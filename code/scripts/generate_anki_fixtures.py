#!/usr/bin/env python3
"""Generate `.apkg` / `.colpkg` fixtures using **real Anki**.

Every Anki fixture in this repository used to be built by our own code: a test
inserted hand-chosen rows with `rusqlite` and called our own
`write_legacy_apkg`. Real SQLite wrote the bytes, so the *file format* was
oracled — but the **Anki semantics were entirely our own understanding of
them**. If our reading of what `queue = 2` means, or how `left` packs learning
steps, or what belongs in the `col` table's `models` JSON were wrong, every test
still passed.

That is the same circularity that let the zstd decoder ship without Huffman
support while its whole suite was green: two halves wrong in the same way agree
perfectly. See #13940.

This script closes it. It drives Anki's own backend — the `anki` package on
PyPI, which is the Rust engine the desktop app uses — so the resulting archives
are produced by Anki rather than by us, and our importer is tested against what
Anki actually writes.

## Why generated rather than downloaded

Every fixture here is **purpose-built and minimal**. No shared decks, so no
third-party content and no licensing question, and each file contains exactly
the one situation it is named for. Provenance is recorded in `PROVENANCE.md`
beside the files, including the Anki version that produced them.

## Determinism

Anki stamps ids and modification times from the wall clock, so re-running this
produces different bytes for the same logical content. **That is expected.** The
fixtures are committed artefacts, not reproducible builds — regenerate only
deliberately, and record why. Tests must therefore assert on *content* (this
card is in learning with two steps left) rather than on byte equality, which is
the right thing to assert anyway.

## Usage

    python -m venv env && env/bin/pip install anki
    env/bin/python code/scripts/generate_anki_fixtures.py --output <dir>
"""

from __future__ import annotations

import argparse
import sys
import tempfile
from pathlib import Path

try:
    from anki.collection import Collection
    from anki.decks import FilteredDeckConfig
    from anki.consts import QUEUE_TYPE_SUSPENDED
except ImportError:  # pragma: no cover - guidance, not logic
    print(
        "This script needs Anki's own backend. Install it into a virtualenv:\n"
        "    python3 -m venv env\n"
        "    env/bin/pip install anki\n"
        "    env/bin/python code/scripts/generate_anki_fixtures.py --output <dir>",
        file=sys.stderr,
    )
    raise SystemExit(2)


def _new_collection(tmp: Path) -> Collection:
    return Collection(str(tmp / "collection.anki2"))


def _basic_note(col: Collection, deck_id: int, front: str, back: str):
    note = col.new_note(col.models.by_name("Basic"))
    note["Front"] = front
    note["Back"] = back
    col.add_note(note, deck_id)
    return note


def _export(col: Collection, out_path: Path, *, legacy: bool) -> None:
    """Export the whole collection as an `.apkg`.

    `legacy=True` yields the V11 layout with uncompressed zip members that
    `write_legacy_apkg` targets; `legacy=False` yields the modern zstd-backed
    `.anki21b` layout.
    """

    from anki.collection import ExportAnkiPackageOptions

    options = ExportAnkiPackageOptions()
    options.with_scheduling = True
    options.with_media = True
    options.legacy = legacy
    col.export_anki_package(out_path=str(out_path), options=options, limit=None)


# --- the corpus -------------------------------------------------------------
#
# One situation per file, named for what it exercises. Each function documents
# which Anki column semantics it is there to pin, because that is the thing our
# own fixtures could never establish.


def review_scheduled(col: Collection, out: Path) -> str:
    """A card on the review queue: `type`/`queue`/`due`/`ivl`/`factor`.

    The common path, and the one whose meaning our hand-built fixture simply
    asserted. `due` here is a *day number* relative to collection creation, not
    a timestamp -- a distinction that only a real export can settle.
    """

    deck_id = col.decks.id("Review Deck")
    note = _basic_note(col, deck_id, "capital of France", "Paris")
    card = note.cards()[0]
    card.type = 2  # review
    card.queue = 2
    card.due = col.sched.today + 5
    card.ivl = 21
    card.factor = 2500
    card.reps = 4
    card.lapses = 1
    col.update_card(card)
    _export(col, out, legacy=True)
    return "one Basic note whose card is scheduled for review in 5 days"


def in_learning(col: Collection, out: Path) -> str:
    """A card mid-learning: the `left` step encoding and `due` as a timestamp.

    Anki packs two numbers into `left`. Our importer has an opinion about that
    encoding which nothing has ever checked against a real file.
    """

    deck_id = col.decks.id("Learning Deck")
    note = _basic_note(col, deck_id, "capital of Japan", "Tokyo")
    card = note.cards()[0]
    card.type = 1  # learning
    card.queue = 1
    card.due = 1_700_000_000  # learning cards use a timestamp, not a day number
    card.left = 1001
    col.update_card(card)
    _export(col, out, legacy=True)
    return "one Basic note whose card is in the learning queue with steps remaining"


def suspended_and_buried(col: Collection, out: Path) -> str:
    """Negative `queue` values, which encode suspension and burial."""

    deck_id = col.decks.id("Flagged Deck")
    first = _basic_note(col, deck_id, "suspended card", "should not appear")
    second = _basic_note(col, deck_id, "buried card", "also should not appear")

    card = first.cards()[0]
    card.queue = QUEUE_TYPE_SUSPENDED
    col.update_card(card)

    buried = second.cards()[0]
    buried.queue = -2  # user-buried
    col.update_card(buried)

    _export(col, out, legacy=True)
    return "two Basic notes, one card suspended (queue -1) and one buried (queue -2)"


def cloze_note(col: Collection, out: Path) -> str:
    """The Cloze note type's `models` JSON and its generated cards."""

    deck_id = col.decks.id("Cloze Deck")
    note = col.new_note(col.models.by_name("Cloze"))
    note["Text"] = "The {{c1::mitochondrion}} is the {{c2::powerhouse}} of the cell"
    col.add_note(note, deck_id)
    _export(col, out, legacy=True)
    return "one Cloze note with two deletions, generating two cards"


def with_media(col: Collection, out: Path) -> str:
    """The `media` map and its numbered archive members.

    Both reference syntaxes appear: `<img src>` and `[sound:]`, because our
    scanner handles them through different code paths.
    """

    deck_id = col.decks.id("Media Deck")
    media_dir = Path(col.media.dir())
    (media_dir / "engram-fixture.png").write_bytes(
        bytes.fromhex("89504e470d0a1a0a0000000d49484452")
    )
    (media_dir / "engram-fixture.mp3").write_bytes(b"ID3\x03\x00\x00\x00\x00\x00\x00")

    note = _basic_note(
        col,
        deck_id,
        'picture <img src="engram-fixture.png">',
        "sound [sound:engram-fixture.mp3]",
    )
    del note
    _export(col, out, legacy=True)
    return "one Basic note referencing an image and an audio file, with both packaged"


def filtered_deck(col: Collection, out: Path) -> str:
    """`odue` / `odid`, which only appear for cards pulled into a filtered deck."""

    home = col.decks.id("Home Deck")
    note = _basic_note(col, home, "filtered subject", "filtered answer")
    card = note.cards()[0]
    card.type = 2
    card.queue = 2
    card.due = col.sched.today
    card.ivl = 10
    col.update_card(card)

    deck = col.sched.get_or_create_filtered_deck(deck_id=0)
    deck.name = "Filtered Deck"
    del deck.config.search_terms[:]
    term = FilteredDeckConfig.SearchTerm()
    term.search = '"deck:Home Deck"'
    term.limit = 10
    term.order = 0
    deck.config.search_terms.append(term)
    col.sched.add_or_update_filtered_deck(deck)

    _export(col, out, legacy=True)
    return "a card pulled into a filtered deck, so it carries odue/odid"


def modern_package(col: Collection, out: Path) -> str:
    """The modern zstd-backed `.anki21b` layout rather than legacy V11."""

    deck_id = col.decks.id("Modern Deck")
    _basic_note(col, deck_id, "modern format", "zstd compressed")
    _export(col, out, legacy=False)
    return "the same simple content in the modern zstd .anki21b layout"


FIXTURES = [
    ("anki-review-scheduled.apkg", review_scheduled),
    ("anki-in-learning.apkg", in_learning),
    ("anki-suspended-buried.apkg", suspended_and_buried),
    ("anki-cloze.apkg", cloze_note),
    ("anki-media.apkg", with_media),
    ("anki-filtered-deck.apkg", filtered_deck),
    ("anki-modern.apkg", modern_package),
]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, help="Directory to write fixtures into")
    args = parser.parse_args(argv)

    out_dir = Path(args.output)
    out_dir.mkdir(parents=True, exist_ok=True)

    from anki.buildinfo import version as anki_version

    described: list[tuple[str, str]] = []
    for name, build in FIXTURES:
        with tempfile.TemporaryDirectory() as tmp:
            col = _new_collection(Path(tmp))
            try:
                description = build(col, out_dir / name)
            finally:
                col.close()
        size = (out_dir / name).stat().st_size
        described.append((name, description))
        print(f"{name}: {size} bytes — {description}")

    provenance = out_dir / "PROVENANCE.md"
    lines = [
        "# Anki fixture provenance",
        "",
        "**These files were produced by Anki itself**, not by this repository.",
        "That is the entire point: our own `.apkg` tests previously validated our",
        "model against our model, so a wrong reading of Anki's column semantics",
        "would pass every test. See #13940.",
        "",
        f"- Produced by: **Anki {anki_version}** (the `anki` package on PyPI — the",
        "  same Rust backend the desktop app uses)",
        "- Generated by: `code/scripts/generate_anki_fixtures.py`",
        "- Content: purpose-built and minimal. No shared decks, so no third-party",
        "  content and no licensing question. Each file contains exactly the one",
        "  situation it is named for.",
        "",
        "## Regenerating",
        "",
        "```",
        "python3 -m venv env && env/bin/pip install anki",
        "env/bin/python code/scripts/generate_anki_fixtures.py --output \\",
        "  code/packages/rust/engram-anki-package/tests/fixtures/anki",
        "```",
        "",
        "Anki stamps ids and modification times from the wall clock, so",
        "regenerating produces **different bytes for the same logical content**.",
        "These are committed artefacts, not reproducible builds. Tests must assert",
        "on content rather than byte equality — which is the right assertion",
        "anyway, since what matters is that we read Anki's meaning correctly.",
        "",
        "## The corpus",
        "",
        "| File | What it pins |",
        "|---|---|",
    ]
    for name, description in described:
        lines.append(f"| `{name}` | {description} |")
    provenance.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"\nwrote {provenance}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
