---
category: CI & GitHub Actions
---

# A figure macro bounded only by width lets a tall, narrow figure run off the page

HL-C443 put stroke-order filmstrips into fifteen human-language books. All the
data, book and app tests passed locally. CI's XeLaTeX build then failed the
LaTeX warning baseline:

```
persian overfull rose to 5 against a baseline of 0; first log line:
Overfull \vbox (857.3039pt too high) has occurred while \output is active
```

Gujarati, Urdu and Chinese also went over the baseline.

`\hlblockfigure` in `_shared/visual.tex` scaled every figure to
`width=0.92\linewidth` and nothing else. That was enough while every block
figure was a wide etymology route. A filmstrip of a tall, narrow letter is
different: alef is about four times taller than it is wide. Scaled to the line
width, it came out well over a page tall. A block figure cannot break across
pages, so TeX reported an overfull page.

The fix bounds the figure's height as well: `height=0.45\textheight` together
with `keepaspectratio`. A figure then scales to whichever limit it meets first,
so wide figures do not change. `tests/figure-targets.test.ts` checks that the
macro keeps both limits, because nothing in the local test suite compiles TeX.

**What to do differently:** when a change sends a new kind of figure through a
shared macro, check the extremes of the new figures' shapes (the tallest and
the widest aspect ratio) against the page. Any `.tex` or figure change needs
the CI books build, which is the only XeLaTeX run.
