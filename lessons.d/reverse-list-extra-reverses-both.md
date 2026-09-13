---
category: Perl
---

# `reverse @list, $extra` reverses BOTH

— Perl precedence parses it as `reverse(@list, $extra)`. Use explicit double parens: `((reverse @list), $extra)`.
