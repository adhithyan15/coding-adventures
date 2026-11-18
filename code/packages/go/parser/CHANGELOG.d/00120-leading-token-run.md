# Batch leading token alternatives

The compiled matcher recognizes a leading contiguous run of direct token
alternatives and checks it with one lookahead. It preserves choice order,
newline skipping, legacy token-enum fallback, failed-prefix diagnostics, and
resource-limit behavior. Mixed alternatives continue through the ordinary
matcher. The bounded differential test checks the optimized behavior against
the frozen parser engine.
