---
category: Testing & coverage
---

# A JSON reader regression must exercise complete documents through the public text API

An ingress implementation can look correct because its underlying parser is expected to reject malformed input, while the tests only assert duplicate keys or a trailing comma. That leaves truncation, trailing non-JSON text, and a second top-level value without direct evidence at the package's public raw-text entry point.

For each raw JSON API, start from a valid complete report and check those three mutations through the public function. Keep language-specific checks for duplicate decoded keys and integer boundaries separate: those test different parser behavior. An object-based replay API cannot stand in for raw text ingress because duplicate spellings may already have been discarded before the object arrives.
