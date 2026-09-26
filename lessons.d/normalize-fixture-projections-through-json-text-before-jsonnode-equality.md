# Normalize fixture projections through JSON text before JsonNode equality

Jackson `valueToTree` preserves JVM numeric widths, so a projected `LongNode(1)`
does not equal a fixture's parsed `IntNode(1)` even though both serialize to the
same language-neutral JSON number. Normalize fixture projections by serializing
to JSON text and parsing that text before `JsonNode` equality; keep native JVM
assertions explicit about `Long` values.
