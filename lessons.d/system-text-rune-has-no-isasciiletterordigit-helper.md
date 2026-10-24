---
category: C#
---

# System.Text.Rune has no IsAsciiLetterOrDigit helper

The first C# document renderer port used `Rune.IsAsciiLetterOrDigit`, mirroring
a JavaScript URL-normalization predicate. .NET 9's `System.Text.Rune` exposes
Unicode category helpers such as `IsLetterOrDigit`, but no ASCII-specific
combined helper, so the first native compile failed with CS0117. For an ASCII
URL allowlist, compare `Rune.Value` explicitly against `a`-`z`, `A`-`Z`, and
`0`-`9`; do not substitute the Unicode-wide helper because that would silently
admit characters the percent-encoding contract requires encoded. Keep the UTF-8
scratch buffer outside the rune loop as well, so analyzers do not flag repeated
`stackalloc` growth in a long input.
