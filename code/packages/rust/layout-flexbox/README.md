# layout-flexbox

`layout-flexbox` resolves the shared `layout-ir` tree into positioned geometry.
It is independent of HTML, Mosaic, and paint backends. Producers write the
typed `flex` extension contract; callers provide one callback for recursively
laying out each child with its resolved constraints.

Supported CSS concepts include row/column and reverse directions, wrapping,
row/column gaps, `justify-content`, `align-items`, `align-self`,
`align-content`, `order`, and flex grow/shrink/basis. Text items use their
longest unbreakable segment as the automatic minimum main size.
