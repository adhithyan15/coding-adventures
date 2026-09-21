---
category: Cross-platform & Windows BUILD_windows
---

# Path assertions written on Windows silently assert nothing on Unix, because a backslash is an ordinary filename character there

`source-preprocessor`'s path-containment tests were written and run on
Windows, passed, and failed the macOS CI job:

```
assertion failed: Path::new(r"C:\a\b\ok.h").starts_with(wroot)
assertion failed: RootedFs::screen_spelling("\\Windows\\win.ini").is_err()
```

Neither was a code bug. On Unix a backslash is **an ordinary filename
character**, not a separator, so:

- `C:\a\b\ok.h` is a *single* path component, not four. `starts_with` compares
  components, so it cannot match a root of `C:\a\b` — the assertion was
  testing nothing about the property it named.
- `\Windows\win.ini` has no root and is not absolute; it is a legal relative
  filename that happens to contain backslashes. Refusing it would reject a
  valid file, and allowing it is correct — it joins under a search root and
  canonicalises inside it, so it cannot escape.

The trap is that both assertions *look* platform-neutral and both pass locally
on the platform this repo primarily develops on.

**What to do differently**

- Assert the *property* with separators that are real on every platform.
  Forward slash works on both — `/a/b` vs `/a/bc` exercises component-wise
  comparison on Windows too.
- Put genuinely platform-specific spellings behind `#[cfg(windows)]`, and say
  in a comment why the other platform is not merely untested but *different*.
  A bare `#[cfg]` reads like a dodge; the reason is what stops someone
  deleting it.
- Distinguish "this check is Windows-only by nature" from "this check has a
  hole on Unix". Drive-qualified spellings (`C:foo`) are caught by a byte
  comparison and so behave identically everywhere; backslash-rooted spellings
  are meaningful only on Windows. Only the second needs gating.

**The wider point:** this repo is Windows-primary, so local green is a weaker
signal than it feels, and the macOS/Linux jobs are the first place a
path assumption gets tested. Any test touching `Path`, separators, absolute
vs relative, reserved names, or case sensitivity should be assumed
platform-dependent until shown otherwise.

Related: `containment_compares_whole_components_not_string_prefixes` in
`fs.rs`, which pins the component-wise behaviour that containment depends on;
and the same file's `/etc/passwd` case, where the platform difference ran the
*other* way — `Path::is_absolute()` is false for a root-relative path on
Windows, so a gate checking only `is_absolute()` had a real hole on the
primary platform and needed `has_root()` as well.
