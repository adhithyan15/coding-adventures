# TEX00 — TeX from scratch: the whole plan, and where DocumentAST fits

**Status:** Plan (no implementation)
**Question this answers:** we have DocumentAST — can TeX integrate into it?
**Short answer:** yes, at *two* layers, and keeping them apart is the whole design.

---

## 1. What already exists

This is the first thing to establish, because the obvious plan — "write a TeX
parser" — is largely **already done**, and planning it again would be waste.

| Crate | Lines | What it is |
|---|---|---|
| `latex` | ~19,600 | A **full-fidelity LaTeX parser**, documents *and* math, catcode-driven state machine with a text-mode-primary mode stack. Supports `\newcommand`/`\def`; surfaces the programmable TeX tail as an explicit unsupported node rather than mis-parsing it |
| `math-frontend` | ~1,400 | The pluggable framework: a **neutral `MathExpr` AST** plus a `MathFrontend` trait |
| `asciimath` | ~1,900 | AsciiMath → `MathExpr` |
| `mathml` | ~1,800 | Presentation-MathML → `MathExpr` |
| `unicode-math` | ~1,300 | Unicode plain math → `MathExpr` |

So four notations already lower to one neutral math AST. **Parsing is not the
gap.**

### What is missing

Nothing turns a `MathExpr` into **geometry**. There is no box model, no font
metrics, no line breaking for math, and no renderer. That is the actual work,
and it is the half TeX is famous for — Knuth's contribution was at least as much
the typesetting algorithms as the language.

---

## 2. The DocumentAST answer

DocumentAST (TE00) is a format-agnostic IR for document *structure*, explicitly
modelled on LLVM IR: N front-ends + M back-ends instead of N × M converters. Its
own diagram **already lists LaTeX on both sides**.

It also already has the math nodes:

```typescript
interface MathInlineNode { readonly type: "math_inline"; readonly value: string }
interface MathBlockNode  { readonly type: "math_block";  readonly value: string }
```

Note what `value` is: **an opaque string**. DocumentAST carries math without
understanding it. That is not an oversight — it is the correct seam, and it is
what makes the two-layer answer work.

### Layer 1 — TeX as a document format: yes, straight into DocumentAST

`\section{}`, `\emph{}`, `itemize`, `tabular`, `\caption` are *structure*. They
map onto `HeadingNode`, `EmphasisNode`, `ListNode`, `TableNode` — nodes that
already exist. This is exactly what DocumentAST is for, and both directions are
worth having:

- **LaTeX front-end**: `latex` crate AST → DocumentAST. Then LaTeX documents
  convert to HTML, DOCX, PDF, and Markdown for free, through back-ends that
  already exist.
- **LaTeX back-end**: DocumentAST → LaTeX. Then Markdown converts to LaTeX for
  free.

Neither needs a new node type. Both are lowering passes.

### Layer 2 — TeX math as typesetting: **not** into DocumentAST

Rendering `\frac{1}{2}` means computing boxes, baselines, and glyph positions.
That is a *geometry* problem, and DocumentAST is a *structure* IR.

**Putting boxes into DocumentAST would be a category error.** It would make
every consumer — the HTML back-end, the sanitiser, the DOCX writer, the
CommonMark converter, and every one of the language ports — carry a typesetting
model it has no use for, and it would couple document structure to font metrics.

So math layout gets **its own IR**, and the seam stays exactly where it is:

```
   LaTeX source ──► latex crate ──► DocumentAST (structure)
                                      │
                                      ├── HeadingNode, ListNode, TableNode … ──► HTML / PDF / DOCX
                                      │
                                      └── MathBlockNode { value: "\frac{1}{2}" }
                                                │  (opaque string — the seam)
                                                ▼
                                          math-frontend ──► MathExpr
                                                              │
                                                              ▼
                                                        MathList (new)
                                                              │  Appendix G
                                                              ▼
                                                         Box tree (new)
                                                              │
                                                     ┌────────┴────────┐
                                                     ▼                 ▼
                                                    SVG               PDF
```

A back-end that cannot typeset math still works: it emits the `value` string, or
a cached image, exactly as today. A back-end that can, calls down.

---

## 3. The work, in order

### TEX-1 — `math-layout`: MathExpr → MathList

TeX's *math list*: atoms classified as Ord, Op, Bin, Rel, Open, Close, Punct,
Inner. The classification is what produces correct spacing — TeX's inter-atom
spacing table is indexed by the pair of adjacent atom classes, which is why
`a+b` spaces differently from `f(x)`.

Pure data transformation, no fonts. Independently testable.

### TEX-2 — Font metrics

Math layout needs per-glyph advance, height, depth, italic correction, and the
math constants (axis height, fraction rule thickness, superscript shift).

Modern OpenType carries these in the **MATH** table. The repo has no font
parsing at all today, so this is genuinely new, and it is the **critical
dependency** — every layout decision needs numbers from here.

Cheapest honest start: a small metrics table for one math font, hand-extracted
and committed with provenance, so TEX-3 can proceed while a real OpenType
parser is written beside it. Record it as temporary, in the issue, so it does
not quietly become permanent.

### TEX-3 — `math-typeset`: MathList → Box tree

TeX's Appendix G. Boxes and glue: hbox, vbox, kern, rule. Fractions, radicals,
scripts with cramped styles, delimiter sizing, matrices.

The hardest and most interesting piece, and the one that makes output look like
TeX rather than like a browser's best guess.

### TEX-4 — Renderers

Box tree → SVG first: resolution-independent, works in the browser and in wasm,
and directly usable by Engram cards. Then box tree → PDF content streams,
sharing the geometry with #13944.

### TEX-5 — DocumentAST lowerings

`latex` AST → DocumentAST, and the reverse. Independent of TEX-1..4 and
deliverable at any point.

### TEX-6 — Engram wiring

`MathBlockNode` / `MathInlineNode` → the pipeline above, rendering to SVG in
cards (#13936).

---

## 4. Sequencing against Engram

#13936 records the two-phase pattern: ship behind an existing engine, then
replace it and keep the library as a differential oracle — the shape that worked
for zstd.

This plan is the **phase 2** side of that. But note the ordering consequence:
TEX-1..4 is a large body of work, and Engram users need working maths sooner. So
phase 1 (borrowed engine) is not a shortcut to feel bad about; it is what buys
the time to do TEX-1..4 properly.

**Check before either:** Anki pre-renders `[latex]` to cached images. If real
decks rely on that, the immediate Engram need is displaying cached images and
this pipeline matters for cards authored *in* Engram. Measure against the real
fixtures (#13940) first — it could reorder everything here.

---

## 5. What makes this provable

Every layer needs an oracle that is not itself:

| Layer | Oracle |
|---|---|
| TEX-1 MathList | Atom classes and spacing are tabulated in *The TeXbook*; assert against the published table |
| TEX-2 Metrics | Compare extracted values against the font's own MATH table read by an independent tool |
| TEX-3 Boxes | Real TeX emits `.dvi` with exact positions. **Rendering the same source through real TeX and comparing box positions is the strongest oracle available**, and it should be built early rather than bolted on |
| TEX-4 SVG | Rasterise and compare against a reference render |

The TEX-3 oracle is the one to insist on. Without it, a from-scratch typesetter
agrees only with itself — the failure this repository has now hit three times:
the zstd encoder agreeing with its own decoder, the emitted Vite project
asserted as text and never built, and Anki fixtures generated by the code they
tested.
