## The pronunciation reference stops being hand-written LaTeX

`kannada/book/chapters/appendix-pronunciation.tex` was hand-authored and printed as a
`\chapter*`. It is now rendered from `kannada/pronunciation-reference.md`. The
chapter title, the contents line and the running head keep their three separate
strings, so the head over the page still reads "Kannada script" rather than
"Pronunciation".

**Restored from the LaTeX before the flip:**

- **Nine worked syllables**, ಕಿ ಕೀ ಕು ಕೂ ಕೆ ಕೇ ಕೊ ಕೋ ಕೌ, and with them the
  romanization **kau** — the Markdown kept only ಕಾ and ಕಿ.
- **"one of the four literary Dravidian languages"**, with Tamil, Telugu and
  Malayalam named. The Markdown had replaced this with a generic statement that
  Dravidian is separate from Indo-European.

**Corrected:** the same conjunct arithmetic as Telugu — **ಸ + ಕ → ಸ್ಕ** becomes
**ಸ್ + ಕ → ಸ್ಕ**, and the virama ್ is shown rather than only named.

**Fixed:** the Markdown hard-wrapped *script-sister* across a line at its hyphen.
Reference prose is joined line by line with a space, so the page would have read
"its closest script- sister".

