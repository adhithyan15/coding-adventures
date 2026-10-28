### Added the Swift source-preserving fenced-block contract

Swift's Document AST now distinguishes fenced input from ordinary code blocks,
the existing CommonMark parser preserves the fence name, complete normalized
info string, and literal payload, and the HTML renderer provides an inert
escaped fallback without interpreting metadata.
