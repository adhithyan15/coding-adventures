## Chapter 1 re-sequenced: atom-first, compositional

Reworked Chapter 1 so words are built from their pieces, then assembled —
per feedback ("start with Hola; then bien and its origin/English cousins and
how it responds on its own and becomes bueno/buena; then día + plurals →
días; then put them together for buenos días; then practice"):

- `L01` **hola** · `L02` **bien** (adverb "well"; ← Latin *bene* → benefit/
  benevolent/benediction; stands alone as an answer; its adjective sibling
  *bueno/buena* ← *bonus*; the well/good ↔ bien/bueno parallel) · `L03`
  **día** (← *dies*; introduces the **plural rule**, día → días; flags día as
  masculine) · `L04` **buenos días** (assembled from *bueno* + *días*;
  introduces **adjective agreement**, *bueno* → *buenos*; the "may God give
  you good days" blessing) · `L05` **practice** (hola & buenos días).
- Grammar is introduced exactly where a word needs it (plurals at *día*,
  agreement at *buenos días*), never front-loaded.
- Removed the previous, out-of-sequence Chapter 1 lessons (buenos días as a
  monolith, plus tardes/noches/gracias/por favor/adiós/cómo estás/me llamo/
  quiero) — kept in git history; tarde/noche/etc. will be rebuilt atom-first
  in their own later chapters, and Chapter 2 will cover *responding* to a
  greeting (a recursive cycle).
- Book Chapter 1 rewritten to match; compiles clean (12 pages). Book author
  set to **Adhithya Rajasekaran** (title page, copyright, PDF metadata).
- **CI fixed**: the book workflow failed on a non-existent apt package
  (`texlive-lang-indic`); replaced the fragile package list with
  `texlive-full` (every engine/language/font). The workflow now goes green
  and uploads each book as a PDF artifact.

