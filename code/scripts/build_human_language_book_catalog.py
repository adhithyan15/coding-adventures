#!/usr/bin/env python3
"""Build a static download catalog for compiled human-language books."""

from __future__ import annotations

import argparse
import html
import json
from pathlib import Path
from typing import Any
from urllib.parse import quote


def load_registry(path: Path) -> dict[str, dict[str, Any]]:
    """Load language metadata keyed by its stable curriculum identifier."""

    payload = json.loads(path.read_text(encoding="utf-8"))
    return {language["id"]: language for language in payload["languages"]}


def build_catalog(
    pdf_dir: Path,
    registry: dict[str, dict[str, Any]],
) -> list[dict[str, str]]:
    """Describe the PDFs that actually exist, in registry order."""

    pdfs = {path.stem: path for path in pdf_dir.glob("*.pdf") if path.is_file()}
    ordered_ids = [language_id for language_id in registry if language_id in pdfs]
    ordered_ids.extend(
        sorted(language_id for language_id in pdfs if language_id not in registry)
    )

    entries: list[dict[str, str]] = []
    for language_id in ordered_ids:
        metadata = registry.get(language_id, {})
        pdf = pdfs[language_id]
        entries.append(
            {
                "id": language_id,
                "name": str(metadata.get("name", language_id.replace("-", " ").title())),
                "family": str(metadata.get("family", "Unclassified")),
                "script": str(metadata.get("script", "Unknown")),
                "file": pdf.name,
            }
        )
    return entries


def render_html(entries: list[dict[str, str]]) -> str:
    """Render an accessible, dependency-free GitHub Pages catalog."""

    book_word = "book" if len(entries) == 1 else "books"
    cards = []
    for entry in entries:
        name = html.escape(entry["name"])
        family = html.escape(entry["family"])
        script = html.escape(entry["script"])
        href = quote(entry["file"])
        cards.append(
            f"""      <article class="book">
        <div>
          <h2>{name}</h2>
          <p>{family} family · {script} script</p>
        </div>
        <a href="{href}" download>Download PDF</a>
      </article>"""
        )

    return f"""<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta name="description" content="Free downloadable language-learning books from Coding Adventures.">
  <title>Human Languages Books · Coding Adventures</title>
  <style>
    :root {{ color-scheme: light dark; font-family: ui-sans-serif, system-ui, sans-serif; }}
    body {{ margin: 0; background: #f6f3ed; color: #20251f; }}
    main {{ width: min(52rem, calc(100% - 2rem)); margin: 0 auto; padding: 4rem 0; }}
    h1 {{ margin-bottom: .5rem; font-family: Georgia, serif; font-size: clamp(2rem, 7vw, 4rem); }}
    .intro {{ max-width: 42rem; color: #4b5448; font-size: 1.08rem; line-height: 1.6; }}
    .catalog {{ display: grid; gap: .8rem; margin-top: 2.5rem; }}
    .book {{ display: flex; align-items: center; justify-content: space-between; gap: 1rem; padding: 1rem 1.2rem; border: 1px solid #ccd2c7; border-radius: .7rem; background: #fff; }}
    .book h2 {{ margin: 0 0 .25rem; font-family: Georgia, serif; font-size: 1.25rem; }}
    .book p {{ margin: 0; color: #596255; }}
    a {{ color: #fff; background: #275d45; border-radius: .45rem; padding: .65rem .85rem; font-weight: 700; text-decoration: none; white-space: nowrap; }}
    a:hover, a:focus-visible {{ background: #153e2c; text-decoration: underline; }}
    footer {{ margin-top: 2.5rem; color: #596255; font-size: .9rem; }}
    @media (prefers-color-scheme: dark) {{
      body {{ background: #151914; color: #f2f4ee; }}
      .intro, .book p, footer {{ color: #c1c8bd; }}
      .book {{ background: #20271f; border-color: #465044; }}
      a {{ background: #69a985; color: #101812; }}
    }}
    @media (max-width: 34rem) {{ .book {{ align-items: stretch; flex-direction: column; }} a {{ text-align: center; }} }}
  </style>
</head>
<body>
  <main>
    <header>
      <p>Coding Adventures</p>
      <h1>Human Languages Books</h1>
      <p class="intro">{len(entries)} free, etymology-first language {book_word}, built from the latest curriculum on the main branch. Each PDF introduces vocabulary, grammar, and script gradually.</p>
    </header>
    <section class="catalog" aria-label="Available books">
{chr(10).join(cards)}
    </section>
    <footer>Licensed CC BY-SA 4.0. New editions are published automatically as the curriculum grows.</footer>
  </main>
</body>
</html>
"""


def write_catalog(
    pdf_dir: Path,
    registry_path: Path,
    output_dir: Path,
) -> list[dict[str, str]]:
    """Write the HTML and machine-readable catalogs, returning their entries."""

    entries = build_catalog(pdf_dir, load_registry(registry_path))
    if not entries:
        raise ValueError(f"no PDF books found in {pdf_dir}")
    output_dir.mkdir(parents=True, exist_ok=True)
    (output_dir / "index.html").write_text(render_html(entries), encoding="utf-8")
    (output_dir / "catalog.json").write_text(
        json.dumps({"version": 1, "books": entries}, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    return entries


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pdf-dir", type=Path, required=True)
    parser.add_argument("--registry", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    entries = write_catalog(args.pdf_dir, args.registry, args.output_dir)
    print(f"Wrote a public catalog for {len(entries)} books to {args.output_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
