# Engram — rendering LaTeX and MathJax (plan for #13936)

**Status:** plan. **Blocked on a prerequisite (§0).** Once that exists, Tier 1
needs no decision; Tier 2 needs the owner's decision (§5).

## 0. Prerequisite: no Engram build shows card HTML or images today

Checked while starting Tier 1:

- the Mosaic app hands the card's face to `ReviewCard` as `prompt : text` /
  `answer : text`, drawn by a plain `Text` on every host;
- the TypeScript web app (`code/programs/typescript/engram-app`) has no HTML
  rendering path either.

So images already embedded in fields (`<img src=…>`) are not shown, and a
LaTeX tag rewritten to `<img src="latex-….png">` would only show more markup.
Before either tier can "display formatted maths", Engram needs a card view
that renders rich content. At minimum that means text runs plus images, in
Mosaic terms a rich-content or image primitive. Journal photos are blocked on
the same primitive.

Tiers 1 and 2 below stay valid as the *content* half. That primitive is the
*display* half, and it comes first.

**Part of:** [the Anki parity goal](engram-anki-parity.md) §3.1 / §4.2, which
ranks LaTeX first among the rendering gaps.

---

## 1. What #13936 asked to check first

> Anki pre-renders LaTeX to images … Imported decks may therefore already
> contain rendered images referenced from fields; check whether that path is
> what real decks actually rely on before building a renderer at all.

The answer depends on which of Anki's two maths syntaxes a field uses. They
behave completely differently.

| syntax | how Anki renders it | images in the deck? |
| --- | --- | --- |
| `[latex]…[/latex]`, `[$]…[/$]`, `[$$]…[/$$]` | Anki desktop runs `latex` + `dvipng`/`dvisvgm` and stores the result as a **media file** | **yes, when desktop generated them** |
| `\(…\)`, `\[…\]` (MathJax) | rendered in the card webview by MathJax at display time | **never** |

So the work splits in two. The first half needs no renderer at all.

## 2. How Anki names a LaTeX image (verified)

From Anki's own `rslib/src/latex.rs` (`extract_latex_refs`, `fname_for_latex`,
`strip_html_for_latex`):

1. Take the tag body. `[latex]` uses it as is. `[$]` wraps it as `$…$`.
   `[$$]` wraps it as `\begin{displaymath}…\end{displaymath}`. Tags match
   non-greedily (`\[latex\](.+?)\[/latex\]`, and likewise for `$` and `$$`).
2. Replace `<br>`, `<br />` and `<div>` with a newline, then strip the
   remaining HTML tags and decode entities (`&nbsp;` becomes a plain space).
3. The file is `latex-<hex SHA-1 of that UTF-8 string>.png`, or `.svg` when the
   note type renders LaTeX as SVG. The note type's LaTeX header and footer are
   **not** part of the hash.

This rule reproduces all three of Anki's own test vectors (checked in Python;
the test for §3 should pin them in Rust):

| field text | file |
| --- | --- |
| `[latex]one<br>and<div>two[/latex]` | `latex-ef30b3f4141c33a5bf7044b0d1961d3399c05d50.png` |
| `[$]<b>hello</b>&nbsp; world[/$]` | `latex-060219fbf3ddb74306abddaf4504276ad793b029.svg` |
| `[$$]math &amp; stuff[/$$]` | `latex-8899f3f849ffdef6e4e9f2f34a923a1f608ebc07.png` |

## 3. Tier 1 — `[latex]` tags from the deck's own images (no dependency)

When a card is rendered, replace each `[latex]` / `[$]` / `[$$]` match with:

- `<img src="latex-<sha1>.svg">` or `.png`, whichever the collection's media
  holds. Engram's media store already resolves `<img src>` (`engram-core`
  `media.rs`, `html_scan.rs`).
- if neither exists: the formula source in a monospace span, marked
  "LaTeX image not in this deck". A visible fallback is honest. Raw markup
  that looks like a broken card is not, and that is #13936's complaint.

This needs no TeX engine and no third-party code, and it matches what Anki
itself shows. The media check should also count these images as *referenced*,
so "Prune unused media" never deletes them. That is a real bug risk once they
render.

**Still to verify with a real exported deck:** does Anki's `.apkg` export
include the generated `latex-*.png` files? Media export is driven by the same
reference extraction, so it should, but #13936 is right that this must be
checked, not assumed. It decides how often Tier 1 finds its image. The
verification is: export a deck with each tag kind from Anki desktop, import
it, and run the test in §6.

## 4. Tier 2 — MathJax `\(…\)` and `\[…\]`

Anki's editor has offered MathJax since 2.1 and needs no TeX install, so many
shared maths decks likely use it. How many is unmeasured; the §3 check should
count both syntaxes in a sample of real decks. Where a deck uses MathJax,
Tier 1 alone leaves raw `\(…\)`. There is no image to find; something has to
typeset.

| option | cost | against |
| --- | --- | --- |
| (A) KaTeX (or MathJax) in the web build only | small; mature and exact | a third-party JS dependency, against `engram-zero-dep-plan.md`; native hosts still need their own answer |
| (B) An in-repo TeX-maths subset renderer (fractions, sub/superscripts, Greek, common operators, roots) to SVG or MathML | large; fits the zero-dependency rule; one renderer for every host | never complete; decks using rarer macros show the fallback |
| (C) MathML output only, rendered by the platform | small in Engram; browsers render MathML Core | native toolkits (Compose, Qt, …) have no MathML view, so native hosts are uncovered |

## 5. Decisions for the owner

1. **Tier 2 backend:** (A), (B) or (C)? B is the only one that serves every
   host without a dependency, and it is the largest.
2. **The display primitive (§0):** a Mosaic rich-content view (a small HTML
   subset: text, `b/i/u`, `br`, `img`) or an image-only slot? Shared with
   Journal photos.
3. **Order:** the §0 primitive first, then Tier 1, then Tier 2. The export
   check in §3 can run at any time.

## 6. Done when (Tier 1)

A deck whose fields hold all three tag kinds imports, and each renders as its
`latex-<sha1>` image, or as the marked fallback when the image is missing. A
Rust test pins the three Anki vectors in §2. The media check counts the images
as referenced.
