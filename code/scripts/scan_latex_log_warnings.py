#!/usr/bin/env python3
"""Scan compiled Human Languages book logs for typesetting warnings.

Why this script exists
----------------------

Every language track's README and CHANGELOG asserts that its book builds with
zero missing glyphs, zero overfull or underfull boxes, zero duplicate PDF
destinations, zero hyperref warnings, and zero font substitutions. Until this
script existed, **nothing checked that claim**: the books workflow ran
``latexmk -halt-on-error``, which stops on hard TeX *errors* but is entirely
happy with a log full of warnings. The claim was prose, not a contract.

XeLaTeX reports these problems as ordinary lines in ``book.log``. So the gate is
simply: read the log, count the lines that match each known warning shape, and
compare those counts against a per-track baseline recorded in the repository.

Why a baseline instead of "must be zero"
----------------------------------------

The books on ``main`` build green today, and we do not know how much warning
debt they already carry. A gate that demanded zero would fail an
already-green branch the moment it landed, which is the wrong way round: a gate
should stop *new* damage, not retroactively condemn old work. So this follows
the same "report first, measure the debt, never retroactively fail" pattern
HL-V01 established for the curriculum gap report:

* a track with a recorded baseline fails only when it **exceeds** that baseline;
* a track with **no** recorded baseline is measured and reported, never failed;
* a track that comes in **under** its baseline is flagged as an invitation to
  tighten the number, but that is good news and never fails the build;
* a track that *has* a baseline but whose ``book.log`` has gone missing fails,
  because otherwise deleting a file would quietly switch its gate off. A track
  with neither a baseline nor a log is only reported: nothing is being lost.

Bootstrap: how the baseline actually gets filled in
---------------------------------------------------

Seeding real numbers requires a real XeLaTeX run over all 20 books, which only
CI performs. So the script always prints a copy-paste-ready ``tracks`` block of
the counts it just observed whenever any track is unseeded (see
``render_bootstrap_block``). The first CI run after this lands emits those
numbers into the job summary; a human pastes them into
``core/latex-warning-baseline.json`` and the gate goes live for those tracks.
Numbers are never invented here — an unseeded track reads ``null``, which means
"nobody has measured this yet", not "zero".

The warning classes
-------------------

::

    class                 example log line
    --------------------- ----------------------------------------------------
    overfull              Overfull \\hbox (12.3pt too wide) in paragraph at ...
    underfull             Underfull \\vbox (badness 10000) has occurred ...
    missing_character     Missing character: There is no ऀ in font ...!
    hyperref_warning      Package hyperref Warning: Token not allowed ...
    duplicate_destination ... destination with the same identifier ...
    font_substitution     LaTeX Font Warning: Font shape `...' undefined

``duplicate_destination`` lines are *also* counted by their own class only —
they are emitted by the PDF driver (``xdvipdfmx``) or by ``pdfTeX``, not by the
hyperref package, so they do not double-count against ``hyperref_warning``.
Every other class is disjoint by construction because the patterns cannot match
the same text.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any

# ---------------------------------------------------------------------------
# The patterns
#
# Each class maps to the regexes that identify it. A log line counts once for a
# class if *any* of that class's patterns matches, so overlapping spellings of
# the same warning (pdfTeX's phrasing versus xdvipdfmx's) never inflate a count.
#
# Patterns are deliberately anchored on the distinctive fixed text TeX prints,
# not on punctuation that varies with line wrapping: TeX hard-wraps its log at
# ~79 columns, so a pattern that spans more than a few words risks being split
# across two lines and silently matching nothing.
# ---------------------------------------------------------------------------

WARNING_PATTERNS: dict[str, tuple[re.Pattern[str], ...]] = {
    "overfull": (re.compile(r"Overfull \\[hv]box"),),
    "underfull": (re.compile(r"Underfull \\[hv]box"),),
    "missing_character": (re.compile(r"Missing character:"),),
    "hyperref_warning": (re.compile(r"Package hyperref Warning"),),
    "duplicate_destination": (
        # pdfTeX: "destination with the same identifier (name{...}) has been
        # already used, duplicate ignored"
        re.compile(r"destination with the same identifier"),
        re.compile(r"duplicate ignored"),
        # xdvipdfmx: "Object @name already defined."
        re.compile(r"Object @\S+ already defined"),
    ),
    "font_substitution": (
        # "LaTeX Font Warning: Font shape `TU/foo/m/n' undefined"
        re.compile(r"Font shape `[^']*' undefined"),
        # "LaTeX Font Warning: Some font shapes were not available, defaults
        # substituted."
        re.compile(r"font shapes were not available"),
    ),
}

# Stable display order and human-readable column headings for the summary table.
WARNING_CLASSES: tuple[str, ...] = tuple(WARNING_PATTERNS)

CLASS_HEADINGS: dict[str, str] = {
    "overfull": "Overfull",
    "underfull": "Underfull",
    "missing_character": "Missing glyph",
    "hyperref_warning": "hyperref",
    "duplicate_destination": "Dup dest",
    "font_substitution": "Font subst",
}

# Status values a track can end a scan with. A status alone does not decide the
# build: each scan result carries its own `blocking` flag, because NO_LOG fails
# for a seeded track and merely reports for an unseeded one.
STATUS_OK = "ok"
STATUS_OVER = "over baseline"
STATUS_UNSEEDED = "unseeded"
STATUS_IMPROVED = "under baseline"
STATUS_NO_LOG = "no log"


# ---------------------------------------------------------------------------
# Counting
# ---------------------------------------------------------------------------


def count_warnings(log_text: str) -> dict[str, int]:
    """Count each warning class in one book's log text.

    A single log line contributes at most one to each class, which keeps the
    count meaningful as "how many warnings did TeX report" rather than "how many
    regexes happened to fire".

    >>> counts = count_warnings("Overfull \\\\hbox (1.0pt too wide)\\nMissing character: x")
    >>> counts["overfull"], counts["missing_character"], counts["underfull"]
    (1, 1, 0)
    """

    counts = {name: 0 for name in WARNING_CLASSES}
    for line in log_text.splitlines():
        for name, patterns in WARNING_PATTERNS.items():
            if any(pattern.search(line) for pattern in patterns):
                counts[name] += 1
    return counts


def read_log(log_path: Path) -> str | None:
    """Read a TeX log, or return ``None`` when it is absent.

    TeX logs are not UTF-8 in general: fonts, file names, and echoed source can
    all carry bytes from any encoding, and a stray invalid byte must not crash
    a gate whose entire job is to be reliable. So decoding replaces bad bytes
    rather than raising. A missing log is a real possibility (a build that was
    skipped, a working directory that moved) and is reported, not fatal — the
    build step itself already fails on hard TeX errors.
    """

    if not log_path.is_file():
        return None
    return log_path.read_text(encoding="utf-8", errors="replace")


def discover_books(book_root: Path) -> list[tuple[str, Path]]:
    """Find every ``<track>/book/`` directory, exactly as the workflow does.

    Returns ``(track, book_dir)`` pairs sorted by track, so the report order is
    deterministic across runs and the diff of a summary stays readable.
    """

    books: list[tuple[str, Path]] = []
    for book_dir in sorted(book_root.glob("*/book")):
        if book_dir.is_dir():
            books.append((book_dir.parent.name, book_dir))
    return books


# ---------------------------------------------------------------------------
# The baseline
# ---------------------------------------------------------------------------


def load_baseline(path: Path | None) -> dict[str, Any]:
    """Load the recorded per-track baseline.

    A missing file is not an error: it means nothing has been recorded yet, so
    every track is unseeded and the run is purely a measurement. That is the
    same outcome as the checked-in file listing every track as ``null``, which
    is how the file ships before the first CI run fills it in.
    """

    if path is None or not path.is_file():
        return {}
    payload = json.loads(path.read_text(encoding="utf-8"))
    tracks = payload.get("tracks") or {}
    if not isinstance(tracks, dict):
        raise ValueError(f"{path}: 'tracks' must be an object")
    return tracks


def baseline_for(tracks: dict[str, Any], track: str) -> dict[str, int] | None:
    """Return one track's recorded counts, or ``None`` when it is unseeded.

    Three spellings all mean "not measured yet", because all three are things a
    human editing JSON by hand will plausibly write:

    * the track key is absent entirely;
    * the track maps to ``null``;
    * the track maps to an object whose values are ``null``.

    A partially seeded object is honoured class by class: a class holding an
    integer is enforced, a class holding ``null`` or missing is unseeded.
    """

    if track not in tracks:
        return None
    entry = tracks[track]
    if entry is None:
        return None
    if not isinstance(entry, dict):
        raise ValueError(f"baseline for {track!r} must be an object or null")

    seeded: dict[str, int] = {}
    for name in WARNING_CLASSES:
        value = entry.get(name)
        if value is None:
            continue
        if not isinstance(value, int) or isinstance(value, bool) or value < 0:
            raise ValueError(
                f"baseline for {track!r}.{name} must be a non-negative integer or null"
            )
        seeded[name] = value
    return seeded or None


# ---------------------------------------------------------------------------
# Scanning and judging
# ---------------------------------------------------------------------------


def scan(book_root: Path, baseline_tracks: dict[str, Any]) -> list[dict[str, Any]]:
    """Scan every discovered book and judge it against the baseline.

    Each result carries the observed counts, the baseline it was judged against
    (possibly ``None``), the classes that exceeded their baseline, a status, and
    a ``blocking`` flag saying whether it should fail the build.

    Two things block. The obvious one is a track above its baseline. The other
    is a track that *has* a baseline but whose log has vanished: once a track is
    measured, silently losing its log would silently switch its gate off, and a
    gate that can be disabled by deleting a file is not a gate. A track with no
    baseline and no log is only reported — there is nothing to lose yet.
    """

    results: list[dict[str, Any]] = []
    for track, book_dir in discover_books(book_root):
        log_text = read_log(book_dir / "book.log")
        if log_text is None:
            recorded = baseline_for(baseline_tracks, track)
            results.append(
                {
                    "track": track,
                    "log": str(book_dir / "book.log"),
                    "counts": None,
                    "baseline": recorded,
                    "regressions": [],
                    "improvements": [],
                    "status": STATUS_NO_LOG,
                    "blocking": recorded is not None,
                }
            )
            continue

        counts = count_warnings(log_text)
        recorded = baseline_for(baseline_tracks, track)

        regressions: list[dict[str, int | str]] = []
        improvements: list[str] = []
        if recorded is not None:
            for name, allowed in recorded.items():
                observed = counts[name]
                if observed > allowed:
                    regressions.append(
                        {"class": name, "observed": observed, "baseline": allowed}
                    )
                elif observed < allowed:
                    improvements.append(name)

        if recorded is None:
            status = STATUS_UNSEEDED
        elif regressions:
            status = STATUS_OVER
        elif improvements:
            status = STATUS_IMPROVED
        else:
            status = STATUS_OK

        results.append(
            {
                "track": track,
                "log": str(book_dir / "book.log"),
                "counts": counts,
                "baseline": recorded,
                "regressions": regressions,
                "improvements": improvements,
                "status": status,
                "blocking": bool(regressions),
            }
        )
    return results


# ---------------------------------------------------------------------------
# Reporting
# ---------------------------------------------------------------------------


def safe_track_name(track: str) -> str:
    """Reduce a track name to characters that cannot break a report.

    A track name is a directory name, and anyone who can open a pull request
    can add a directory. That name then reaches two places that interpret their
    input: ``$GITHUB_STEP_SUMMARY``, which GitHub renders as Markdown, and the
    job log, where a line beginning ``::`` is a workflow command. A name holding
    a backtick or a pipe would escape its code span or its table column; a name
    holding a newline could forge a workflow command outright. Neither is
    catastrophic, but a gate whose own report can be scrambled by the change it
    is judging is not worth much.

    Nothing else untrusted reaches either place: every other value reported is
    an integer this script computed, never a string taken from a ``.log``.

    Characters outside a conservative allowlist are replaced rather than
    dropped, so two different names can never collapse into one label.

    >>> safe_track_name("tamil")
    'tamil'
    >>> safe_track_name("ev|il`name")
    'ev?il?name'
    """

    return "".join(
        character if character.isalnum() or character in "-_." else "?"
        for character in track
    )


def track_label(track: str) -> str:
    """Render a track name as a Markdown code span that cannot break out.

    >>> track_label("ev|il`name")
    '`ev?il?name`'
    """

    return f"`{safe_track_name(track)}`"


def format_cell(observed: int, allowed: int | None) -> str:
    """Render one table cell as ``observed`` or ``observed / baseline``.

    An over-baseline cell is marked so the eye lands on it immediately in a job
    summary that may list twenty tracks and six columns.
    """

    if allowed is None:
        return f"{observed} / –"
    if observed > allowed:
        return f"**{observed} / {allowed}** ⛔"
    return f"{observed} / {allowed}"


def render_table(results: list[dict[str, Any]]) -> str:
    """Render the per-track Markdown table for the job summary."""

    headings = " | ".join(CLASS_HEADINGS[name] for name in WARNING_CLASSES)
    lines = [
        f"| Track | {headings} | Status |",
        "|---" * (len(WARNING_CLASSES) + 2) + "|",
    ]
    for result in results:
        counts = result["counts"]
        if counts is None:
            cells = ["–"] * len(WARNING_CLASSES)
        else:
            recorded = result["baseline"] or {}
            cells = [
                format_cell(counts[name], recorded.get(name))
                for name in WARNING_CLASSES
            ]
        lines.append(
            f"| {track_label(result['track'])} | "
            + " | ".join(cells)
            + f" | {result['status']} |"
        )
    return "\n".join(lines)


def render_bootstrap_block(results: list[dict[str, Any]]) -> str:
    """Render a copy-paste-ready ``tracks`` block from the observed counts.

    This is the bootstrap path described in the module docstring: the numbers
    can only come from a real XeLaTeX run, so CI measures them and prints them
    here for a human to paste into the checked-in baseline file.
    """

    tracks = {
        result["track"]: result["counts"]
        for result in results
        if result["counts"] is not None
    }
    return json.dumps({"tracks": tracks}, indent=2, sort_keys=True) + "\n"


def render_summary(results: list[dict[str, Any]]) -> str:
    """Render the whole job summary: table, verdict, and bootstrap if needed."""

    unseeded = [r["track"] for r in results if r["status"] == STATUS_UNSEEDED]
    over = [r for r in results if r["status"] == STATUS_OVER]
    missing = [r["track"] for r in results if r["status"] == STATUS_NO_LOG]

    parts = [
        "## LaTeX warning gate",
        "",
        "Each cell is `observed / baseline`; `–` means no baseline has been "
        "recorded for that track yet, so it is measured but never failed.",
        "",
        render_table(results),
        "",
    ]

    if over:
        parts.append("### New warnings above baseline")
        parts.append("")
        for result in over:
            for regression in result["regressions"]:
                parts.append(
                    f"- {track_label(result['track'])} "
                    f"**{regression['class']}**: "
                    f"{regression['observed']} observed, "
                    f"{regression['baseline']} allowed."
                )
        parts.append("")

    if missing:
        parts.append(
            "> No `book.log` was found for: "
            + ", ".join(track_label(track) for track in missing)
            + ". These tracks were not measured."
        )
        parts.append("")
        blocked = [
            r["track"]
            for r in results
            if r["status"] == STATUS_NO_LOG and r["blocking"]
        ]
        if blocked:
            parts.append(
                "> ⛔ "
                + ", ".join(track_label(track) for track in blocked)
                + " already had a recorded baseline, so a missing log turns a "
                "measured track back into an unmeasured one and fails the gate."
            )
            parts.append("")

    if unseeded:
        parts.append("### Bootstrap: seed these baselines")
        parts.append("")
        parts.append(
            "These tracks have no recorded baseline yet: "
            + ", ".join(track_label(track) for track in unseeded)
            + ". Paste the `tracks` block below into "
            "`code/learning/human-languages/core/latex-warning-baseline.json` "
            "to turn the gate on for them. The numbers below are what this run "
            "actually measured — they are never guessed."
        )
        parts.append("")
        parts.append("```json")
        parts.append(render_bootstrap_block(results).rstrip("\n"))
        parts.append("```")
        parts.append("")

    if not any(result["blocking"] for result in results):
        parts.append("No track exceeded its recorded baseline.")
        parts.append("")

    return "\n".join(parts)


def render_text_report(results: list[dict[str, Any]]) -> str:
    """Render a plain-text report for the job log (no Markdown decoration)."""

    lines = []
    for result in results:
        track = safe_track_name(result["track"])
        counts = result["counts"]
        if counts is None:
            lines.append(f"{track}: no book.log")
            continue
        rendered = " ".join(f"{name}={counts[name]}" for name in WARNING_CLASSES)
        lines.append(f"{track}: {rendered} [{result['status']}]")
    return "\n".join(lines)


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--book-root",
        type=Path,
        required=True,
        help="Directory holding <track>/book/ subdirectories.",
    )
    parser.add_argument(
        "--baseline",
        type=Path,
        default=None,
        help="Recorded per-track baseline JSON. Missing file means unseeded.",
    )
    parser.add_argument(
        "--summary",
        type=Path,
        default=None,
        help="Append the Markdown report here (typically $GITHUB_STEP_SUMMARY).",
    )
    parser.add_argument(
        "--emit-baseline",
        type=Path,
        default=None,
        help="Write the observed counts as a baseline-shaped JSON file.",
    )
    parser.add_argument(
        "--json",
        dest="json_output",
        type=Path,
        default=None,
        help="Write the full machine-readable scan result here.",
    )
    parser.add_argument(
        "--github-annotations",
        action="store_true",
        help=(
            "Emit GitHub Actions ::error:: and ::warning:: commands. "
            "Opt in only from the real workflow gate so failure-path unit "
            "tests cannot attach fixture annotations to their own run."
        ),
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)

    if not args.book_root.is_dir():
        prefix = "::error::" if args.github_annotations else "error: "
        print(f"{prefix}book root {args.book_root} does not exist", file=sys.stderr)
        return 1

    results = scan(args.book_root, load_baseline(args.baseline))
    if not results:
        prefix = "::error::" if args.github_annotations else "error: "
        print(
            f"{prefix}no <track>/book/ directories under {args.book_root}",
            file=sys.stderr,
        )
        return 1

    print(render_text_report(results))

    summary = render_summary(results)
    if args.summary is not None:
        with args.summary.open("a", encoding="utf-8") as handle:
            handle.write(summary + "\n")

    if args.emit_baseline is not None:
        args.emit_baseline.parent.mkdir(parents=True, exist_ok=True)
        args.emit_baseline.write_text(render_bootstrap_block(results), encoding="utf-8")

    if args.json_output is not None:
        args.json_output.parent.mkdir(parents=True, exist_ok=True)
        args.json_output.write_text(
            json.dumps({"version": 1, "tracks": results}, indent=2, sort_keys=True)
            + "\n",
            encoding="utf-8",
        )

    if args.github_annotations:
        # Every value in a workflow command below is either an integer this
        # script computed or a track name reduced by `safe_track_name`, so a
        # crafted directory name cannot forge a `::` command line of its own.
        for result in results:
            if result["status"] != STATUS_NO_LOG:
                continue
            track = safe_track_name(result["track"])
            if result["blocking"]:
                print(
                    f"::error::{track} has a recorded baseline but no book.log; "
                    "the gate cannot measure it"
                )
            else:
                print(f"::warning::no book.log found for {track}")

        for result in results:
            track = safe_track_name(result["track"])
            for regression in result["regressions"]:
                print(
                    f"::error::{track} {regression['class']} rose to "
                    f"{regression['observed']} against a baseline of "
                    f"{regression['baseline']}"
                )

    return 1 if any(result["blocking"] for result in results) else 0


if __name__ == "__main__":
    raise SystemExit(main())
