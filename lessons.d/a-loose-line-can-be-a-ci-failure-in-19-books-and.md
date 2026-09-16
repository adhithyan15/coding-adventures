# A loose line can be a CI failure in 19 books and invisible in 4 — hbadness is set per track, not per corpus (human-language books)

Two PRs (#15256 Hindi/Marathi, #15260 Punjabi) added the *same* A1 timed-writing
lesson to three tracks. CI failed the Books gate on Marathi and Punjabi:

```
marathi underfull rose to 1 against a baseline of 0
  Underfull \hbox (badness 1831) in paragraph at lines 36--37
```

Hindi got the byte-identical paragraph and reported **zero**. The tempting
reading — "the same prose happens to break differently under different font
metrics" — is wrong, and it sends you looking at fonts.

**The paragraph is main-font Latin text, and it sets an identical badness-1831
line in every track.** What differs is the preamble:

| | `\hbadness` | tracks |
|---|---|---|
| default | 1000 | 19 tracks — **reports** 1831 |
| raised | 2000 | chinese, hindi, japanese, tamil — **suppresses** it |

`\hbadness` is a *reporting* threshold, not a typesetting parameter. 1831 < 2000,
so Hindi's book carried exactly the same loose line and simply never mentioned
it. Proven by re-compiling pre-fix Hindi with `\hbadness=1000`: the identical
warning appears.

So the rules:

- **A track that passes the LaTeX warning gate is not evidence the typography is
  sound.** It may only mean that track's preamble is more tolerant. When the same
  content lands in several tracks, fix the content everywhere, not just where CI
  went red.
- **Do not "fix" it by raising `\hbadness` (or the warning baseline) on the red
  track.** That converts a real loose line into a silent one and widens the
  four-track blind spot. `core/latex-warning-baseline.json` says the same thing
  about its own numbers: never raise one without explaining why.
- The trigger here is a **long unbreakable `\texttt{}` run stranded at a line
  break**. `` `task-shapes/a1.json` `` is ~19 unhyphenatable characters; when it
  does not fit on the current line TeX pushes the whole box down and stretches
  what is left. Put such a token **early** in its paragraph, where ordinary
  breakable prose follows it, rather than after a long lead-in:

  ```
  -  Those numbers are not invented here. They are read out of
  -  `task-shapes/a1.json`, the same file the assessment contract points at.
  +  Those numbers are read out of `task-shapes/a1.json`, the same file the
  +  assessment contract points at. They are not invented here.
  ```

### Verifying it locally is worth the install

There is no need to guess at this class of bug. `check-book-compile.sh` plus
`scan_latex_log_warnings.py` reproduce the CI failure exactly:

```bash
sudo apt-get install -y texlive-xetex texlive-latex-extra \
  texlive-fonts-recommended texlive-lang-arabic texlive-lang-cyrillic latexmk
cd code/packages/typescript/human-language-data && npm install && npm run build
bash code/scripts/check-book-compile.sh marathi
python3 code/scripts/scan_latex_log_warnings.py \
  --book-root code/learning/human-languages \
  --baseline code/learning/human-languages/core/latex-warning-baseline.json
```

One track compiles in well under a minute. Tracks whose preamble loads bidi
(hindi, and the RTL tracks) additionally need `texlive-lang-arabic`, or they die
on a missing `bidi.sty` that has nothing to do with the content under test.

### Editing a lesson touches three generators, not one

`generate:books` alone leaves `check:modality` red. After changing any
`lessons/*.md`, run all three and then the gates:

```bash
npm run generate:books && npm run generate:narration && npm run generate:modality
```
