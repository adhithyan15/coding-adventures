---
category: C#
---

# System.Text.Json writes a lone surrogate as U+FFFD, so a test that serializes a hostile string sends a different, harmless one

**What went wrong.** The XAML platform library's headless harness (UI87 §7.6)
checked that `files.save` refuses every hostile `suggestedName` by building
each payload with `JsonSerializer.SerializeToElement(dictionary)`. For
`"a\uD800.json"` the check failed: the library had answered `ok`. The rule
itself was right (`IsPlainFileName` refused the string when called directly);
the serializer had replaced the lone surrogate with U+FFFD, a visible symbol,
so the payload the library received named `a�.json`, which is a legitimate
plain file name. The test was sending a different input from the one it
named.

**Fix.** Payloads for the hostile names are written as JSON text by hand,
with every non-ASCII UTF-16 unit as a `\uXXXX` escape, and parsed with
`JsonDocument.Parse` -- which is what arrives on the wire. The library reads
the field with `GetString()`, which throws `InvalidOperationException` for an
escaped lone surrogate; it treats that as "not a name" and refuses it.

**Do differently.** When a test feeds a boundary a malformed value, build the
wire form yourself rather than asking a serializer to: encoders "helpfully"
repair what they are given (U+FFFD for surrogates, normalisation, escaping),
and then the test proves nothing about the malformed case. Assert on the
exact outcome message, not just "did not crash", so a silently repaired input
shows up as a wrong answer.
