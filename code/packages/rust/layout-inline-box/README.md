# layout-inline-box

`layout-inline-box` owns fragmented inline edge policy for the shared Layout
IR. It keeps margins, padding, border widths, and `box-decoration-break`
continuation independent from HTML, paint backends, and native toolkits.

`InlineBoxStyle::fragment_edges` resolves which logical start/end decorations
belong to each line fragment. `decorate_fragment` expands tight content
geometry, shifts descendants, and suppresses non-continuing border paint while
preserving semantic metadata for hit testing.

Spec: [`UI44-layout-inline-box`](../../../specs/UI44-layout-inline-box.md).
