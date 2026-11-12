---
category: Testing & coverage
---

# F# xUnit array equality assertions must use the element type

Passing an array type as xUnit's generic argument makes Assert.Equal<T> look
for a sequence of arrays. For arrays of strings, Assert.Equal<string array>
therefore fails to compile because the actual value is a sequence of strings,
not a sequence of string arrays.

Use the element type, Assert.Equal<string>(expected, actual), so xUnit compares
the ordered string elements. This keeps sequence equality and does not weaken
the assertion to a count or set comparison.
