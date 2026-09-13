#!/usr/bin/env python3
"""One-time migration: `lessons.md` -> `lessons.d/`, verified by reconciliation.

Committed rather than thrown away, because the interesting part is not the
transformation but the CHECK on it. A migration that reports success while
dropping text is the exact failure this repo keeps re-learning, most recently
from a CHANGELOG resolver that kept every line matching `^- ` and quietly ate a
90-line prose section -- and then PASSED its own assertions, because those
assertions also only looked at bullets. The check and the transform shared a
blind spot, so their agreement proved nothing.

So the check here is not a sample and not a filter. Every non-whitespace
character of the source body must land in exactly one shard, and the shortfall
must equal the bucket `## Heading` lines EXACTLY -- those being the only text
that legitimately changes form, moving into each shard's `category` field. Any
other delta fails the migration.

WHAT IS A BUCKET
================

The file is two documents in one. Early `##` sections are topical buckets
holding many `- **...**` bullets, each bullet a separate lesson. Later ones are
a single lesson whose `##` heading is a sentence-long claim and whose bullets
are sub-points of it.

The split between them is an EXPLICIT allowlist below, not a heuristic. A first
pass inferred it from title length and word count and misfiled two plain
buckets (`Compiler / VM / language pipeline`, 31 bullets; `QR / format-marker /
file-format specifics`, 8). A third, `Gradle / JVM`, reads like a bucket but has
zero top-level bullets -- it uses `###` subsections -- and treating it as one
dropped ten prose lines on the floor. There are only 23; enumeration is cheap,
and a list can be reviewed by eye in a way a threshold cannot.
"""

from __future__ import annotations

import re
import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from lessons import slugify  # noqa: E402

REPO_ROOT = Path(__file__).resolve().parents[2]
SOURCE = REPO_ROOT / "lessons.md"
TARGET = REPO_ROOT / "lessons.d"

BUCKETS = [
    "Security boundaries",
    "Supply chain & CI pinning",
    "BUILD files & dependency management",
    "Cross-platform & Windows BUILD_windows",
    "Workspace & package metadata",
    "Python",
    "Ruby",
    "Lua",
    "Perl",
    "Elixir",
    "Swift",
    "C#",
    "Haskell",
    "TypeScript / JavaScript",
    "Rust",
    "Native extensions & FFI",
    "Compiler / VM / language pipeline",
    "Cryptography & security review",
    "Testing & coverage",
    "CI & GitHub Actions",
    "QR / format-marker / file-format specifics",
    "Repo policy / workflow reminders",
    "Mosaic compiler pipeline",
]


def dense(text: str) -> str:
    return "".join(text.split())


def carve_sections(lines: list[str]) -> tuple[list[str], list[tuple[str, list[str]]]]:
    """Split into (preamble, [(title, body)]), ignoring `##` inside code fences."""
    preamble: list[str] = []
    sections: list[tuple[str, list[str]]] = []
    current: tuple[str, list[str]] | None = None
    in_fence = False
    for line in lines:
        if line.startswith("```"):
            in_fence = not in_fence
        if not in_fence and line.startswith("## "):
            if current:
                sections.append(current)
            current = (line[3:].strip(), [])
        elif current:
            current[1].append(line)
        else:
            preamble.append(line)
    if current:
        sections.append(current)
    return preamble, sections


def split_bullets(body: list[str]) -> list[list[str]]:
    """Top-level `- ` bullets with their continuation lines; prose kept separate."""
    items: list[list[str]] = []
    current: list[str] | None = None
    in_fence = False
    for line in body:
        if line.startswith("```"):
            in_fence = not in_fence
        if not in_fence and re.match(r"^- ", line):
            if current:
                items.append(current)
            current = [line]
        elif current is not None:
            current.append(line)
        elif line.strip():
            items.append(["\0PROSE", line])
    if current:
        items.append(current)
    return items


def title_of_bullet(text: str) -> str:
    """A bullet's title is its leading bolded claim, else its first sentence."""
    match = re.match(r"^- \*\*(.+?)\*\*", text, re.S)
    if match:
        return " ".join(match.group(1).split()).rstrip(".")
    stripped = re.sub(r"^- ", "", text)
    flat = " ".join(stripped.split())
    cut = flat.find(". ")
    return (flat[:cut] if 0 < cut < 140 else flat[:140]).rstrip(".")


def drop_leading_restatement(body: str, title: str) -> str:
    """Remove the body's opening sentence when it just restates the title.

    Compared on non-whitespace characters, because the title was flattened to
    one line while the body still wraps -- a literal `body.startswith(title)`
    matches almost nothing here.
    """
    flat = " ".join(body.split())
    key = " ".join(title.split())
    if not flat.startswith(key):
        return body
    rest = flat[len(key) :].lstrip(". ").strip()
    if not rest:
        return body  # the claim IS the lesson; keep it as the body too
    # Walk the original text to the same point, so wrapping and markup inside
    # the remainder survive untouched.
    seen = 0
    for index, char in enumerate(body):
        if not char.isspace():
            seen += 1
        if seen > len(key.replace(" ", "")):
            return body[index:].lstrip(". \n")
    return body


def main() -> int:
    if not SOURCE.exists():
        # The expected state once the migration has landed: `lessons.md` is
        # generated-and-ignored, so a tracked one no longer exists. Say that,
        # rather than tracebacking on a FileNotFoundError and reading as a
        # broken script.
        sys.exit(
            f"{SOURCE.name} is not present, so the migration has already run "
            f"(or the aggregate has not been rendered). This script is kept as "
            f"the auditable record of how lessons.d/ was produced; to work with "
            f"lessons now, use code/scripts/lessons.py."
        )

    lines = SOURCE.read_text(encoding="utf-8").splitlines()

    # `lessons.py render` RECREATES lessons.md at this exact path, and the
    # rendered aggregate is not shaped like the original: lessons sit at `###`
    # under `##` categories, where the source had bucket bullets and
    # sentence-titled `##` sections. Re-running the migration over a render
    # would therefore re-shard a different document and overwrite every shard
    # with garbage -- while the reconciliation still balanced, because it only
    # checks that characters are conserved, not that the input was the original.
    if any("GENERATED" in line for line in lines[:8]):
        sys.exit(
            f"{SOURCE.name} is the GENERATED aggregate from `lessons.py "
            f"render`, not the original file. Re-sharding it would overwrite "
            f"lessons.d/ with a re-parse of its own output. Refusing."
        )

    preamble, sections = carve_sections(lines)

    missing = set(BUCKETS) - {t for t, _ in sections}
    if missing:
        sys.exit(f"allowlist names sections that do not exist: {sorted(missing)}")

    # `kind` is carried explicitly rather than inferred from `category is
    # None`. That proxy is wrong: the seven preamble bullets have no category
    # AND no `##` heading, so inferring from the category credited each of them
    # a heading the source never contained, and the reconciliation came out
    # NEGATIVE -- the shards appearing to hold more text than the file they
    # came from.
    shards: list[tuple[str, str | None, str, str, str]] = []
    taken: set[str] = set()
    stranded: list[str] = []

    def add(title: str, category: str | None, text: str, kind: str) -> None:
        slug = slugify(title, taken)
        taken.add(slug)
        shards.append((slug, category, title, text.rstrip(), kind))

    # Seven real lessons sit above the first heading. The `# Lessons Learned`
    # title, the descriptive paragraph and the `---` rule are document
    # furniture, replaced by `_meta.md`; the bullets are lessons.
    for item in split_bullets(preamble):
        if item[0] == "\0PROSE":
            continue
        text = "\n".join(item).rstrip()
        if text.strip():
            add(title_of_bullet(text), None, text, "bullet")

    for title, body in sections:
        if title in BUCKETS:
            for item in split_bullets(body):
                if item[0] == "\0PROSE":
                    stranded.append(f"[{title}] {item[1][:80]}")
                    continue
                text = "\n".join(item).rstrip()
                if text.strip():
                    add(title_of_bullet(text), title, text, "bullet")
        else:
            add(title, None, "\n".join(body).rstrip(), "section")

    if stranded:
        sys.exit(
            "prose inside a bucket section has no bullet to belong to; it would "
            "be dropped:\n  " + "\n  ".join(stranded)
        )

    # ---- reconcile against the WHOLE input, before writing anything ---------
    source_dense = dense("\n".join(lines))
    # Furniture is whatever the preamble holds MINUS the bullets kept as
    # lessons -- derived by subtraction rather than by matching `^- `, because
    # a bullet's continuation lines do not start with `- ` and would otherwise
    # be counted as furniture and silently written off.
    preamble_kept = sum(
        len(dense("\n".join(item)))
        for item in split_bullets(preamble)
        if item[0] != "\0PROSE"
    )
    furniture = len(dense("\n".join(preamble))) - preamble_kept
    heading_chars = sum(len(dense(f"## {t}")) for t, _ in sections if t in BUCKETS)
    shard_dense = sum(
        len(dense((f"## {t}" if kind == "section" else "") + text))
        for _slug, _cat, t, text, kind in shards
    )
    delta = len(source_dense) - shard_dense
    explained = heading_chars + furniture
    print(
        f"{len(sections)} sections -> {len(shards)} shards\n"
        f"  source {len(source_dense)} dense chars, shards {shard_dense}\n"
        f"  delta {delta} = {heading_chars} bucket headings "
        f"+ {furniture} preamble furniture"
    )
    if delta != explained:
        sys.exit(
            f"UNRECONCILED: {delta - explained} dense chars unaccounted for. "
            f"That is dropped lesson text, not formatting -- refusing to write."
        )

    # ---- write --------------------------------------------------------------
    # Remove only the shards this script manages. An earlier version called
    # `shutil.rmtree(TARGET)` and destroyed the hand-written `_meta.md` sitting
    # beside them -- which is also the repo rule against force-deleting
    # anything that is not retrievable from git. `_meta.md` had not been
    # committed yet, so it was simply gone.
    TARGET.mkdir(exist_ok=True)
    for stale in TARGET.glob("*.md"):
        if stale.name != "_meta.md":
            stale.unlink()
    for slug, category, title, text, kind in shards:
        body = text
        if kind == "bullet":
            # A bucket bullet becomes a standalone lesson. Replace the `- `
            # marker with two spaces rather than deleting it, so EVERY line of
            # the block shares one indent, then dedent by the common prefix.
            # Deleting it instead would leave the continuation lines indented
            # relative to a now-unindented first line -- and 40 lines in this
            # file sit at 4+ spaces, which Markdown would then render as a code
            # block rather than prose.
            body = textwrap.dedent("  " + body[2:])
            body = re.sub(r"^\*\*(.+?)\*\*", r"\1", body, flags=re.S)
            body = re.sub(r"^\s*\n", "", body)
            # The bullet's leading claim became the title, so it would
            # otherwise open every one of these files twice. Drop it ONLY when
            # real text follows -- for a single-sentence bullet the claim is
            # the whole lesson, and removing it would leave a title with an
            # empty body.
            body = drop_leading_restatement(body, title)
        # Second reconciliation, on the bytes actually being STORED. The first
        # one above proved the SPLIT loses nothing, but it ran on the text
        # before the cosmetic rewrite -- so on its own it would vouch for a
        # file whose written body had been mangled afterwards.
        #
        # The rewrite is allowed exactly two effects: drop a leading
        # restatement of the title, and drop the `- `/`**` list markup. So the
        # written body must account for every dense char of the original,
        # except those the title itself now carries.
        before, after = len(dense(text)), len(dense(body))
        if after != before:
            # title, its sentence-ending period (which `title_of_bullet`
            # rstrips, so it is not in `title`), and the `- ` + `**`/`**` that
            # made the claim a bold list item.
            allowed = len(dense(title)) + 1 + len(dense("- ****"))
            if before - after > allowed:
                sys.exit(
                    f"{slug}: rewrite dropped {before - after} dense chars, "
                    f"more than the {allowed} the title and list markup can "
                    f"account for -- refusing to write"
                )

        front = f"---\ncategory: {category}\n---\n\n" if category else ""
        (TARGET / f"{slug}.md").write_text(
            f"{front}# {title}\n\n{body.strip()}\n", encoding="utf-8"
        )
    print(f"  wrote {len(shards)} shards to {TARGET}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
