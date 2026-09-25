### Fixed — a block figure is bounded in height as well as width, so a tall filmstrip fits the page (HL-C443)

`_shared/visual.tex`'s `\hlblockfigure` scaled every figure to
`0.92\linewidth` and nothing else. Stroke-order filmstrips of tall, narrow
letters, such as Persian and Urdu alef at about 4:1, came out taller than a
page. CI's XeLaTeX build reported overfull pages in Persian, Urdu, Gujarati and
Chinese.

The macro now also caps height at `0.45\textheight`, with
`keepaspectratio`, so wide figures are unchanged. `tests/figure-targets.test.ts`
checks that the macro keeps both bounds.
