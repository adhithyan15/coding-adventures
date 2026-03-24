"""HTML Entity Decoder.

CommonMark requires decoding three forms of HTML character references
within text content and link destinations:

  Named:   &amp;  → &    &lt; → <    &gt; → >    &quot; → "
  Decimal: &#65;  → A    &#169; → ©
  Hex:     &#x41; → A    &#xA9; → ©

=== Implementation ===

Python's `html.unescape` handles most named entities, but does not cover
all HTML5 named entities (it only covers a subset). We use `html.entities.html5`
which IS the full HTML5 named character reference table used by the spec.

=== Why entity decoding matters ===

CommonMark says: "An entity reference consists of & + any of the valid
HTML5 named entities + ;". The decoded character should appear in the
output, not the raw reference. So `&amp;copy;` in source text should
render as © in HTML (and &copy; in HTML source).

There are ~2200 HTML5 named entities. Python's html.entities.html5
covers all of them for full CommonMark compliance.
"""

import html.entities
import re


def decode_entity(ref: str) -> str:
    """Decode a single HTML character reference.

    Accepts the full reference including the leading & and trailing ;:
      "&amp;" → "&"
      "&#65;" → "A"
      "&#x41;" → "A"
      "&unknown;" → "&unknown;" (unrecognised — returned as-is)

    This is called during inline parsing whenever a & is encountered.
    The return value is a decoded Unicode string ready for the output IR.

    === How decoding works ===

    Three forms:
      1. Numeric decimal: &#NNNN; — parse N as base 10, convert to chr()
      2. Numeric hex:     &#xHHHH; — parse H as base 16, convert to chr()
      3. Named entity:    &name; — look up in html.entities.html5 table

    For named entities, the table key format includes the trailing semicolon:
      html.entities.html5["amp;"] == "&"

    Special cases:
      - &#0; → U+FFFD (null character → replacement character)
      - Codepoints > U+10FFFF → U+FFFD (out of Unicode range)
    """
    if ref.startswith("&#"):
        # Numeric entity: decimal &#NNN; or hexadecimal &#xHHH;
        inner = ref[2:-1]  # strip "&#" and ";"
        try:
            if inner.startswith(("x", "X")):
                codepoint = int(inner[1:], 16)
            else:
                codepoint = int(inner, 10)
            # Null character and out-of-range code points → replacement character
            if codepoint == 0 or codepoint > 0x10FFFF:
                return "\uFFFD"
            return chr(codepoint)
        except ValueError:
            return ref  # malformed — return as-is
    else:
        # Named entity: &name;
        # html.entities.html5 keys include the semicolon: "amp;" → "&"
        key = ref[1:]  # strip leading "&"
        decoded = html.entities.html5.get(key)
        if decoded is not None:
            return decoded
        return ref  # unrecognised entity — return as-is


# Pre-compiled regex for entity matching in text. This matches the three
# forms of HTML character references defined by CommonMark:
#   Named:   &[a-zA-Z][a-zA-Z0-9]{0,31};  (max 32 chars per spec)
#   Decimal: &#[0-9]{1,7};
#   Hex:     &#[xX][0-9a-fA-F]{1,6};
#
# We compile this once at module level (not inside the function) to avoid
# re-analysing the pattern on every call.
_ENTITY_RE = re.compile(
    r"&(?:#[xX][0-9a-fA-F]{1,6}|#[0-9]{1,7}|[a-zA-Z][a-zA-Z0-9]{0,31});"
)


def decode_entities(text: str) -> str:
    """Decode all HTML character references in `text`.

    Replaces every valid & ... ; entity reference with its decoded character.
    Unrecognised references (e.g. &unknownentity;) are left as-is.

    This is the batch version of `decode_entity`, suitable for processing
    whole strings (e.g. link titles, code-fence info strings).

    Example:
        decode_entities("Hello &amp; world") == "Hello & world"
        decode_entities("caf&eacute;") == "café"
        decode_entities("&#169;") == "©"
        decode_entities("&unknown;") == "&unknown;"  # unrecognised
    """
    return _ENTITY_RE.sub(lambda m: decode_entity(m.group(0)), text)


def escape_html(text: str) -> str:
    """Escape text for safe inclusion in HTML output.

    The five characters with HTML significance in content/attributes are encoded:
      &  → &amp;
      <  → &lt;
      >  → &gt;
      "  → &quot;

    Note: ' is NOT escaped. CommonMark's reference HTML renderer (cmark)
    does not escape single quotes — only the four above. Escaping ' would
    produce &apos; which is not universally supported in HTML4 and some XML
    parsers, and would fail CommonMark spec tests that expect the literal '.

    This is the inverse of decode_entities — escape_html makes text safe for
    embedding in HTML, decode_entities converts HTML references back to Unicode.
    """
    text = text.replace("&", "&amp;")
    text = text.replace("<", "&lt;")
    text = text.replace(">", "&gt;")
    text = text.replace('"', "&quot;")
    return text
