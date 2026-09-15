#!/usr/bin/env python3
"""Work with `lessons.d/`, the repo's sharded record of engineering failures.

WHY THIS EXISTS
===============

`lessons.md` used to be one 640KB file. Every PR that hit a CI failure appended
to it, which meant every pair of concurrent PRs conflicted on it -- always
additively, always resolved by "keep both", and always at the cost of a blocked
auto-merge and a round trip. The conflicts carried no information whatsoever.

Sharding removes the shared file. A new lesson is a NEW FILE, and two new files
cannot conflict.

THE ONE DESIGN DECISION WORTH EXPLAINING
========================================

A shard's **filename is its identity**. There is no `id:` field and no ordinal
prefix. Both alternatives have already failed in this repo:

  * An ordinal prefix is POSITIONAL. A `(index + 1) * 10` scheme used elsewhere
    renamed 21 files when one shard was inserted in the middle -- and a rename
    is a conflict for every branch holding the old name.

  * A separate `id:` field collides SILENTLY. Two branches once wrote the id
    `MR-EXT-038` into two differently-named files. Git saw two unrelated new
    files and merged both without a murmur; the duplicate was invisible from
    either branch alone, because each was internally consistent.

Deriving the name from the title closes both holes at once, and gives exactly
the conflict behaviour you want:

    two DIFFERENT lessons  -> two different filenames -> clean merge
    the SAME lesson twice  -> one filename             -> a real conflict

The second case is not a flaw. Two branches recording the same lesson genuinely
do need reconciling, and a conflict is how you find out.

FILE FORMAT
===========

    ---
    category: Rust
    ---

    # A tail expression holding a MutexGuard fails CI but compiles locally

    <body>

`category` is optional; a shard without one renders under `Uncategorised`. When
present it must appear in `lessons.d/_meta.md`, which is the single source of
truth for the category list -- so adding a category is a deliberate edit to one
small file rather than a typo that silently invents a new heading.
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
LESSONS_DIR = REPO_ROOT / "lessons.d"
META = "_meta.md"
UNCATEGORISED = "Uncategorised"
PLACEHOLDER = (
    "TODO: what went wrong, what the fix was, and what to do differently. "
    "Replace this line."
)


@dataclass(frozen=True)
class Lesson:
    slug: str
    title: str
    category: str
    body: str
    path: Path


# --------------------------------------------------------------------------
# Slugs
# --------------------------------------------------------------------------
def slugify(title: str, taken: set[str] | None = None) -> str:
    """Turn a lesson title into a filename stem.

    Markup is stripped first so that `` `foo` `` and `**foo**` slug the same as
    plain `foo` -- otherwise the same lesson written with and without emphasis
    would produce two different files, and the duplicate-detection that makes
    this scheme safe would not fire.
    """
    text = re.sub(r"`([^`]*)`", r"\1", title)
    text = re.sub(r"\*\*([^*]*)\*\*", r"\1", text)
    text = re.sub(r"\*([^*]*)\*", r"\1", text)
    slug = re.sub(r"[^a-z0-9]+", "-", text.lower()).strip("-")
    # Cap the length so paths stay workable, but cut on a word boundary: a hard
    # character slice can end mid-word and make two unrelated lessons collide.
    slug = "-".join(slug.split("-")[:12])[:80].strip("-")
    slug = slug or "lesson"
    if taken is None:
        return slug
    base, n = slug, 2
    while slug in taken:
        slug, n = f"{base}-{n}", n + 1
    return slug


# --------------------------------------------------------------------------
# Reading
# --------------------------------------------------------------------------
def read_categories(lessons_dir: Path = LESSONS_DIR) -> list[str]:
    """Category names, in render order, from the `## Categories` list in _meta.md."""
    meta = (lessons_dir / META).read_text(encoding="utf-8")
    section = re.search(
        r"^## Categories\s*$(.*?)(?=^## |\Z)", meta, re.M | re.S
    )
    if not section:
        raise SystemExit(f"{META} has no '## Categories' section")
    return [
        m.group(1).strip()
        for m in re.finditer(r"^- +(.+?)\s*$", section.group(1), re.M)
    ]


def parse_lesson(path: Path) -> Lesson:
    raw = path.read_text(encoding="utf-8")
    category = UNCATEGORISED
    body = raw
    if raw.startswith("---\n"):
        end = raw.find("\n---\n", 4)
        if end == -1:
            raise ValueError(f"{path.name}: unterminated frontmatter")
        front, body = raw[4:end], raw[end + 5 :]
        for line in front.splitlines():
            key, _, value = line.partition(":")
            if key.strip() == "category":
                category = value.strip()
    match = re.search(r"^# +(.+?)\s*$", body, re.M)
    if not match:
        raise ValueError(f"{path.name}: no '# Title' heading")
    title = match.group(1).strip()
    return Lesson(path.stem, title, category, body[match.end() :].strip(), path)


def load(lessons_dir: Path = LESSONS_DIR) -> list[Lesson]:
    return [
        parse_lesson(p)
        for p in sorted(lessons_dir.glob("*.md"))
        if p.name != META
    ]


# --------------------------------------------------------------------------
# Commands
# --------------------------------------------------------------------------
def cmd_validate(lessons_dir: Path) -> int:
    known = set(read_categories(lessons_dir)) | {UNCATEGORISED}
    problems: list[str] = []
    seen: dict[str, str] = {}

    for path in sorted(lessons_dir.glob("*.md")):
        if path.name == META:
            continue
        try:
            lesson = parse_lesson(path)
        except ValueError as error:
            problems.append(str(error))
            continue
        if lesson.category not in known:
            problems.append(
                f"{path.name}: category {lesson.category!r} is not listed in "
                f"{META}"
            )
        if not lesson.body.strip():
            problems.append(f"{path.name}: has a title but no body")
        elif PLACEHOLDER.split(".")[0] in lesson.body:
            problems.append(
                f"{path.name}: still holds the `lessons.py new` placeholder"
            )
        # The filename IS the identity, so a title that slugs to a DIFFERENT
        # name than its own file means two lessons can still collide later --
        # one added under the correct slug, one hiding under a stale one.
        expected = slugify(lesson.title)
        if expected != lesson.slug and not re.fullmatch(
            re.escape(expected) + r"-\d+", lesson.slug
        ):
            problems.append(
                f"{path.name}: title slugs to {expected!r}; rename the file or "
                f"retitle the lesson so the name stays derivable"
            )
        # An odd number of fences means a code block runs off the end of the
        # shard. During the migration that would have meant a block split
        # across two files -- text present but unreadable in both. Checked
        # across all 502 migrated shards (zero unbalanced) and pinned here so a
        # later hand-edit cannot reintroduce it quietly.
        if sum(1 for line in lesson.body.splitlines()
               if line.startswith("```")) % 2:
            problems.append(f"{path.name}: unbalanced ``` code fence")
        if lesson.title in seen:
            problems.append(
                f"{path.name}: duplicate title, already used by {seen[lesson.title]}"
            )
        seen[lesson.title] = path.name

    for problem in problems:
        print(f"lessons: {problem}", file=sys.stderr)
    count = len(list(lessons_dir.glob("*.md"))) - 1
    print(f"lessons: {count} shards, {len(problems)} problem(s)")
    return 1 if problems else 0


def demote_headings(body: str) -> str:
    """Push a shard's own sub-headings one level deeper for the aggregate.

    In a shard, the lesson is `#` and a sub-heading is `##`. In the render the
    lesson becomes `###`, so an un-demoted `###` sub-heading would sit at the
    SAME level as a lesson title -- 21 of them do, across 6 shards -- and the
    aggregate's outline would claim 529 lessons where there are 508.

    Fenced blocks are skipped, because `# comment` on the first column of a
    shell example matches the heading pattern exactly and is not a heading.
    """
    out, in_fence = [], False
    for line in body.splitlines():
        if line.startswith("```"):
            in_fence = not in_fence
            out.append(line)
            continue
        match = None if in_fence else re.match(r"^(#{1,5}) (?=\S)", line)
        out.append(f"#{line}" if match else line)
    return "\n".join(out)


def render(lessons_dir: Path = LESSONS_DIR) -> str:
    order = read_categories(lessons_dir)
    rank = {name: i for i, name in enumerate(order)}
    lessons = load(lessons_dir)

    out = [
        "# Lessons Learned",
        "",
        "GENERATED -- do not edit, and do not commit. Each lesson lives in its",
        "own file under `lessons.d/`; this aggregate is rebuilt on demand by",
        "`code/scripts/lessons.py render`. Edit the shard, not this file.",
        "",
    ]
    for category in order + [UNCATEGORISED]:
        group = sorted(
            (item for item in lessons if item.category == category),
            key=lambda item: item.title.lower(),
        )
        if not group:
            continue
        out += [f"## {category}", ""]
        for lesson in group:
            out += [f"### {lesson.title}", "", demote_headings(lesson.body), ""]
    # A category nobody listed would otherwise vanish from the render with no
    # signal; validate() rejects it, and this makes the render fail loudly too.
    stray = {item.category for item in lessons} - set(rank) - {UNCATEGORISED}
    if stray:
        raise SystemExit(f"unlisted categories would not render: {sorted(stray)}")
    return "\n".join(out).rstrip() + "\n"


def cmd_render(lessons_dir: Path, target: Path) -> int:
    target.write_text(render(lessons_dir), encoding="utf-8")
    print(f"lessons: wrote {target}")
    return 0


def cmd_index(lessons_dir: Path) -> int:
    for lesson in sorted(
        load(lessons_dir), key=lambda item: (item.category, item.title)
    ):
        print(f"{lesson.category}\t{lesson.slug}\t{lesson.title}")
    return 0


def cmd_new(lessons_dir: Path, title: str, category: str | None) -> int:
    known = read_categories(lessons_dir)
    if category and category not in known:
        print(
            f"lessons: unknown category {category!r}. Known: {', '.join(known)}",
            file=sys.stderr,
        )
        return 1
    slug = slugify(title)
    path = lessons_dir / f"{slug}.md"
    # `is_symlink()` as well as `exists()`: `exists()` follows the link, so a
    # DANGLING symlink at this path reads as absent and the write would follow
    # it out of the directory. Hardening rather than a live hole -- it needs
    # someone to plant a link at the exact slug of a title a maintainer has yet
    # to type -- but it is one call.
    if path.exists() or path.is_symlink():
        # Not an error to route around: the same lesson already exists, and the
        # right move is to sharpen that one rather than add a near-duplicate.
        print(
            f"lessons: {path.name} already exists -- edit it instead of adding "
            f"a second copy",
            file=sys.stderr,
        )
        return 1
    front = f"---\ncategory: {category}\n---\n\n" if category else ""
    # A template, not an empty file. `new` writing a body-less shard would
    # produce something `validate` immediately rejects, so the documented
    # workflow would fail on its own first step. The placeholder is itself
    # rejected by `validate`, so forgetting to replace it is loud rather than
    # landing an empty lesson.
    path.write_text(
        f"{front}# {title}\n\n{PLACEHOLDER}\n", encoding="utf-8"
    )
    print(path)
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--dir", type=Path, default=LESSONS_DIR)
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("validate")
    render_parser = sub.add_parser("render")
    render_parser.add_argument("target", nargs="?", type=Path,
                               default=REPO_ROOT / "lessons.md")
    sub.add_parser("index")
    new_parser = sub.add_parser("new")
    new_parser.add_argument("title")
    new_parser.add_argument("--category")

    args = parser.parse_args(argv)
    if args.command == "validate":
        return cmd_validate(args.dir)
    if args.command == "render":
        return cmd_render(args.dir, args.target)
    if args.command == "index":
        return cmd_index(args.dir)
    if args.command == "new":
        return cmd_new(args.dir, args.title, args.category)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
