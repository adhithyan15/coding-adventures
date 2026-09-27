## Unreleased — writing lessons lazy-load their stroke-order filmstrips

Opening a writing lesson now loads its canonical `*-filmstrip.svg` and places
the stroke-by-stroke figure at the top of the first Writing or Script section.
The hundreds of filmstrip URL loaders live in a dynamically imported module,
so unopened lessons add nothing to first paint and the 500 kB eager bundle
budget remains intact. The bundle gate now requires that source-map chunk to
exist exactly once and remain outside the preload set.
