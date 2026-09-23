## HL-C435-ae03b945 — Nested bold-italic in wrap-up answers renders garbled in 37 book chapters and no gate catches it

**Status: OPEN.** Found while shipping HL-C434, in a lesson I had just written;
the same defect turns out to be pre-existing across thirteen tracks. Recorded
rather than fixed, because fixing it touches 37 chapters in tracks this branch
does not otherwise go near.

### The shape

Wrap-up Recall answers are written as parenthesised bold, and the answer often
contains a target word that wants italics:

```markdown
(***Sechzehn*, *siebzehn***.)
```

The intent is **bold**, with *Sechzehn* and *siebzehn* italic inside it. What
the book generator emits is:

```latex
\emph{\textbf{Sechzehn\emph{, }siebzehn}}.)
```

The comma is italicised and the second word is not — the delimiters paired
backwards. Worse cases leak a literal asterisk into the PDF:

```latex
\emph{*nas-\emph{\textbf{, a genuine cousin pair.) Where does English ...
```

The one I introduced swallowed an entire paragraph, running the bold across two
following sentences before it closed.

### It is not the `***x***` idiom that breaks

`(***Warum***.)` — a balanced triple on both sides — is everywhere in this
corpus and renders correctly. The failure needs **an inner run of emphasis
between the triples**, so that a paragraph carries an odd count of `*` runs and
the parser pairs them across sentence boundaries rather than inside one.

That is why it is invisible to authors: the idiom looks like the one right above
it in the same file.

### Scale

37 chapters, by track:

```
spanish 9   marathi 7   french 4   hindi 3   german 3
telugu 2    tamil 2     portuguese 2
sanskrit 1  punjabi 1   malayalam 1  latin 1  bengali 1
```

Detected with:

```
grep -rlE '\\emph\{[^}]*\\textbf\{[^}]*\\emph\{' */book/chapters/*.tex
```

### NO GATE CATCHES THIS, WHICH IS THE REAL FINDING

`check:books` compares generated output against its inputs, so garbled markup
round-trips as agreement. `check:compile` builds the PDFs, and this is all valid
LaTeX — it compiles clean and prints the wrong thing. `info-dump`, `glyph-coverage`
and the corpus rules read the Markdown source, where the text is correct.

Every existing gate passes on all 37. The defect is only visible in the rendered
output, which nothing asserts on.

### Proposed fix

1. **A gate first**, before touching content: fail on the
   `\emph{…\textbf{…\emph{` nesting signature in generated `.tex`, pinned as a
   ceiling at 0 for fixed tracks. Without it the 37 will regrow.
2. Then repair per track, smallest first, rewriting the answers to use bold
   alone (`(**Dank is a noun, so viel takes an ending.**)`) or balanced triples
   with no inner run. Both read identically on the page.
3. A `lessons.d` entry on the authoring rule: inside a parenthesised bold answer,
   do not open a second emphasis run.

Cheap, mechanical, and worth doing before the corpus grows further — the count
has gone up with every vocabulary tranche, because the idiom is copied from
neighbouring lessons.
