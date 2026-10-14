---
category: Testing & coverage
---

# Native browser acceptance must use an isolated bookmark profile

The Windows browser acceptance launched with the default persistent profile,
then required exactly one bookmark. Previous acceptance entries made the real
WinUI interaction fail even after the generated project built and launched.
Use the existing VENTURE_BOOKMARKS_PATH override on the acceptance child with
a file inside its unique output directory. Preserve the user's default profile,
all real interaction assertions and the original failed receipt. Never replace
the desktop acceptance with its optional build-only skip to hide this failure.
