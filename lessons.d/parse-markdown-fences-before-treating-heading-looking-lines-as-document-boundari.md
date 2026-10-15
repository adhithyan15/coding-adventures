# Parse Markdown fences before treating heading-looking lines as document boundaries

The README migration treated every heading-looking line as a section boundary.
One `## Writing — delayed recall` line was example content inside a fenced
Markdown block, so the mechanical split created two documents with unmatched
fences and promoted the example to a false owner. Track fenced-code state when
discovering Markdown headings, and make direct-owner validation reject
unbalanced fences while ignoring heading syntax inside balanced examples.
