## The improvement is not a better ranking

There is no ranking left — only `explicar` appears in more than one failing
row, and a greedy set cover over the remainder is flat. What 4b does is use
the **tie-break** on purpose.

When every word costs the same, the only lever is *which rows a scene happens
to complete*. So this tranche goes after the **cheapest remaining rows** — seven
two-word rows and one three-word row, which is why chapter 455 carries five
headwords rather than four — grouped into four scenes, so **every chapter
finishes exactly two rows on its own**:

| chapter | scene | words | rows |
|---|---|---|---|
| 452 | the delivery round | almacén, la ruta, el seguimiento, cómodo | 2 |
| 453 | the shift | limpieza, instalación, rotativo, consultar | 2 |
| 454 | the closed road | la obra, la vía, el accidente, el servicio | 2 |
| 455 | the room | pintar, dormitorio, claro, tender, meter | 2 |

That is the whole method now: pick the cheapest rows, then find the scene that
holds them. It will keep working until the cheap rows run out.

