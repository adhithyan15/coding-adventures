#!/usr/bin/env python3
"""sync-templates.py — put the emitted FormulaBar and Grid fragments into
index.html, or check that they are already there.

index.html embeds every compiled variant as a `<template>` so the page works
when opened straight from disk:

    <template id="grid-template-dark">   <- build/Grid.dark.html
    <template id="grid-template-light">  <- build/Grid.light.html
    <template id="fb-desktop-dark">      <- build/FormulaBar.desktop.dark.html
    <template id="fb-desktop-light">     <- build/FormulaBar.desktop.light.html
    <template id="fb-touch-dark">        <- build/FormulaBar.touch.dark.html
    <template id="fb-touch-light">       <- build/FormulaBar.touch.light.html

They used to be pasted in by hand after running scripts/build.sh, so the page
silently kept whatever the emitter produced on the day someone last pasted.
scripts/build.sh now calls this to write them, and `scripts/build.sh --check`
(run by CI) fails when a fresh compile differs from what the page carries.

Usage: sync-templates.py BUILD_DIR INDEX_HTML [--check]
"""

import re
import sys

TEMPLATES = {
    "grid-template-dark": "Grid.dark.html",
    "grid-template-light": "Grid.light.html",
    "fb-desktop-dark": "FormulaBar.desktop.dark.html",
    "fb-desktop-light": "FormulaBar.desktop.light.html",
    "fb-touch-dark": "FormulaBar.touch.dark.html",
    "fb-touch-light": "FormulaBar.touch.light.html",
}


def synced(page, build_dir):
    for template_id, file_name in TEMPLATES.items():
        with open(f"{build_dir}/{file_name}", encoding="utf-8") as handle:
            fragment = handle.read().rstrip("\n")
        pattern = re.compile(
            rf'(<template id="{re.escape(template_id)}">)(.*?)(</template>)', re.DOTALL
        )
        if len(pattern.findall(page)) != 1:
            sys.exit(f'sync-templates.py: index.html must hold exactly one <template id="{template_id}">')
        page = pattern.sub(lambda match: match.group(1) + fragment + match.group(3), page)
    return page


def main():
    if len(sys.argv) not in (3, 4) or (len(sys.argv) == 4 and sys.argv[3] != "--check"):
        sys.exit(__doc__)
    build_dir, index_html = sys.argv[1], sys.argv[2]
    with open(index_html, encoding="utf-8") as handle:
        current = handle.read()
    expected = synced(current, build_dir)
    if len(sys.argv) == 4:
        if expected != current:
            sys.exit(
                "index.html carries stale Mosaic fragments: the emitter's output changed.\n"
                "Regenerate with: bash code/programs/typescript/visicalc-html/scripts/build.sh"
            )
        print("index.html fragments are current")
        return
    if expected != current:
        with open(index_html, "w", encoding="utf-8") as handle:
            handle.write(expected)
        print("index.html fragments updated")
    else:
        print("index.html fragments already current")


if __name__ == "__main__":
    main()
