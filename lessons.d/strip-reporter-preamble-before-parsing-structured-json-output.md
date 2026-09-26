# Strip reporter preamble before parsing structured JSON output

The package-parity reporter's JSON mode may emit human-readable warnings before
the JSON document. Passing the complete stdout directly to a JSON parser then
fails even when the report itself succeeded. When consuming this CLI
programmatically, locate the first JSON object boundary, preserve the preamble
as diagnostic evidence, and parse only the structured suffix; still check the
process exit code and the report's collision and unknown-language fields.
