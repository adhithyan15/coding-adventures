"""Decode the small scalar subset used by lesson-authoring scripts.

Keep this in step with ``human-language-data/src/frontmatter.ts``. Lists remain
the caller's responsibility; these scripts only need their scalar fields.
"""

LITERAL_SCALAR_FIELDS = {"headword", "romanization", "gloss", "concept_tag"}


def decode_scalar(value: str, key: str = "") -> str:
    value = value.strip()
    if len(value) < 2:
        return value
    first, last = value[0], value[-1]
    if first == last == '"':
        if key in LITERAL_SCALAR_FIELDS and "\\" in value[1:-1]:
            raise ValueError(
                f"{key} cannot use a double-quoted frontmatter scalar "
                "containing backslashes; use single quotes around the scalar instead"
            )
        return value[1:-1]
    if first == last == "'":
        return value[1:-1].replace("''", "'")
    return value
