### Changed — etymology figure rendering takes its language tags as an input (HL-C419)

**Breaking for `index.ts` consumers.** Three exported functions gained a
required argument:

| function | was | now |
|---|---|---|
| `etymologyRootNode` | `(root)` | `(root, vocabulary)` |
| `renderEtymologyRouteFigure` | `(lesson)` | `(lesson, vocabulary)` |
| `renderFigure` | `(target, lesson, sources)` | same, but `sources.rootTags` is required for an `etymology-route` target |

`renderFigure` throws a named, actionable error when `rootTags` is absent. A
plain-JS caller of the other two gets a `TypeError` instead, which is the cost
of not keeping a filesystem-reading default.

#### Why the argument rather than a default

`etymologyRootNode` used to find the language tag by POSITION — take the last
hyphen-separated token — which is correct for exactly one of the three slug
shapes the corpus uses. The HL-C419 normalisation flipped `kahve-turkish` to
`turkish-kahve` and the published SVG for `ES-C06-cafe` went out claiming Arabic
*qahwah* became **Kahve, a word in "turkish"**. Nothing threw; the hash ledger
regenerated to match.

The first fix read `core/root-tags.json` inside the renderer. That resolved the
**package's** install location rather than the caller's curriculum root, so a
figure generated for root R depended on a file not under R — and it broke the
contract `figure.ts` states in its own header, that the module "stays free of
the filesystem and every figure remains a pure function of its inputs". The
evidence it was a live regression: making the parameter required broke two
`figure-cli` fixtures with ENOENT, because they had been silently consuming the
real repository's copy and were never self-contained.

`figureSources(root, targets)` now loads the vocabulary from the caller's root,
only when an `etymology-route` target exists — the same conditional shape as the
filmstrip ledger, for the same reason.

#### Fixed — a blank caption could be published without an error

The guard against a term-less slug was written three times, each stopping one
step short of where the value actually goes:

| guard | let through | printed |
|---|---|---|
| `lemma === ""` | `latin--` | `""` |
| `term === ""` | `latin- -` | `" "` |
| printability | — | — |

`latin- -` is **pure ASCII** and survives the frontmatter list parser, which
trims only an item's outer edges; `latin-<U+200B>` survives because format
characters are not whitespace. Each miss published the same artifact: a box with
a language under it and no word in it, nothing thrown, hash ledger regenerated
to match. The guard now strips `\p{White_Space}` and `\p{Cf}` before testing.

No live slug reached any of these. They were latent.
