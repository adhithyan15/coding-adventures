## Unreleased — handwriting tools load only when a letter opens

- The cited pen-path corpus, filmstrip renderer, and TrueType parser now arrive
  through the handwriting view's dynamic import instead of the initial page
  preload. This restores the intended lazy boundary as the source-verified
  corpus grows, while the lightweight script inventory remains available for
  first paint.
- The bundle gate now rejects an eager `handwriting-tools` chunk explicitly,
  so splitting the same preload bytes into smaller files cannot masquerade as
  a fix for the 500 kB startup budget.
- Telugu's shipped font is now registered with the live handwriting view, so
  its source-verified filmstrips can upgrade from the prose fallback in the
  browser as well as appear in the generated book.
