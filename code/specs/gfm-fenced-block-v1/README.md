# TE04 fenced-block repair vectors

`cases.json` contains 16 independently authored Markdown-to-AST literals for
the fenced metadata component: paired full-info retention, anonymous/empty,
tilde, CRLF, indentation, escapes, entities decoded once, decoded whitespace,
interior whitespace, short closing fences and unclosed input. The TypeScript
parser reads these vectors; HTML/layout/sanitizer and Forme tests use separate
hand-built nodes.

This is a bounded repair fixture, not the complete language-neutral GFM
extension corpus or qualification of any other lane. Existing CommonMark
HTML corpus literals remain unchanged. See [TE04](../TE04-gfm-extensions.md).
