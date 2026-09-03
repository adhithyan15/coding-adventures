## HL-C317 — consecutive blockquote lines join into one paragraph, and the contrast is always the lesson

Found by compiling the Punjabi book and reading it. Ten instances across two
tranches, and every single one destroyed the point of its own lesson.

### The mechanism

`book.ts` renders a run of consecutive `> ` lines as ONE paragraph. Authored:

    > **ਧੰਨਵਾਦ** · **ਪਰ** — the inherited and Sanskritic side
    > **ਸ਼ੁਕਰੀਆ** · **ਲੇਕਿਨ** — the Perso-Arabic side

Printed:

    ਧੰਨਵਾਦ · ਪਰ — the inherited and Sanskritic side ਸ਼ੁਕਰੀਆ · ਲੇਕਿਨ — the
    Perso-Arabic side

A single `>` line is a display block and renders correctly. Two or more collapse.

### Why it matters more than it looks

This is the same class as HL-C312's ordered lists, but worse, because of WHAT
authors put in two-line blockquotes. A census of the ten instances found:

- the two sides of a Sanskritic / Perso-Arabic doublet
- **ਕਿ** against **ਕੀ**, a minimal pair one stroke apart
- **ਤ** against **ਥ**, a minimal pair one stroke apart
- *I did not follow* against *I do not have the fact*
- the exclusive and inclusive **we**

**In every case the contrast was the lesson.** The two-line blockquote is the
shape an author reaches for precisely when two things must be seen apart, and it
is the one shape that guarantees they are printed together. No text assertion can
see it; the lesson markdown is correct and the rendered page is wrong.

### The rule

**Never use two or more consecutive `> ` lines.** Use a bulleted list — it
renders as `itemize` with real line breaks, and the corpus already prefers it.
Keep single-line blockquotes; they are fine and they are the right shape for one
display line.

    - **ਧੰਨਵਾਦ** · **ਪਰ** — the inherited and Sanskritic side
    - **ਸ਼ੁਕਰੀਆ** · **ਲੇਕਿਨ** — the Perso-Arabic side

### How to find them

    python3 - <<'EOF'
    import io, glob
    for f in glob.glob('*/lessons/*.md'):
        run = 0
        for n, l in enumerate(io.open(f, encoding='utf-8').read().split('\n')):
            run = run + 1 if l.startswith('> ') else 0
            if run == 2: print(f, n)
    EOF

Seven were fixed in the Punjabi tranche before it shipped. Three shipped in the
Gujarati tranche (`GU-C34-kem-kemke`, `GU-C36-ame-aapne`, `GU-C37-kyaan-write`)
and want a follow-up — the Gujarati one is the exclusive/inclusive **we**, which
is exactly the contrast a reader most needs to see on two lines.

### And the standing rule this is the third instance of

Compile the book and read the pages. HL-C312 was ordered lists, HL-C313 was a
paradigm table the gate could not see, and this is blockquotes. All three were
invisible to every test in the suite and obvious on the page.
