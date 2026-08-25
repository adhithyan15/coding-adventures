### Fixed — live generated curriculum links

- Preserve canonical Markdown links as live LaTeX `\href` targets instead of
  dropping every destination during book generation.
- Resolve relative lesson and pronunciation-reference links against stable
  GitHub source URLs while preserving absolute source citations and rich link
  labels from the same canonical blocks consumed by Language Ladder.
- Reject missing relative-link bases and non-HTTP(S) destinations, escape URL
  metacharacters for LaTeX, and regenerate the nine affected chapters with 55
  working links.

