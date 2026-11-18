# Reuse bounded token scratch in the compact line parser

Hook-free `LineParser.ParseLineInto` now reuses its private token buffer. It
clears occupied entries before reuse, drops buffers above 4096 tokens, and
keeps the owning path when `PreParse` hooks could retain or replace tokens.
This change does not claim a Linux throughput improvement; see KILN04 for the
measurement contract.
