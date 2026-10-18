## The root-slug guard caught a cousin I had missed

`golpe` was drafted with a newly minted `colaphus-latin`. `generate:root-slug-splits`
rejected it:

```
colaphus-latin / latin-colaphus
```

The corpus already held the etymon — in the **French** track. `FR-C40-beaucoup`
carries `latin-colaphus`, because *beaucoup* is *beau* + *coup*, "a beautiful
blow". My grep had looked for *colpus*, not *colaphus*, so I missed it.

Corrected to `latin-colaphus`, which makes Spanish *golpe* a genuine cousin of
French *beaucoup* — and the lesson now says so, because it is a better fact
than the one it replaced: the same punch gives Spanish its *golpe* and French
its word for plenty.

Worth recording that **preflight's `--new-roots` is an assertion, not a
verification**. It accepted `colaphus-latin` because I told it the slug was
new. Only the corpus-wide split check knows whether it actually is.

