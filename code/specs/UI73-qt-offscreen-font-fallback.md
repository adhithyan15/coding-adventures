# UI73: Qt offscreen font fallback

## Status

Implemented by Mosaic's generated Qt project shell.

## Purpose

Qt's offscreen platform plugin reports `Sans Serif` as its generic system font.
CoreText does not expose that placeholder as an installed macOS family, so the
first text lookup scans localized family aliases and emits a slow-path warning.
Generated shells need deterministic headless acceptance without choosing a
macOS-specific family or changing an application's real font policy.

## Contract

After `QApplication` exists and before QML initializes, a generated shell may
normalize the font only when both conditions hold:

1. `QGuiApplication::platformName()` is `offscreen`.
2. The inherited application family is Qt's exact `Sans Serif` placeholder.

The shell clears that unavailable family, retains all other inherited font
attributes, and sets `QFont::SansSerif` as the abstract matching hint. Any real
platform family or host-selected family bypasses normalization. Generated QML
must not contain a platform font name.

## Acceptance

Emitter tests pin the guard, family-free style hint, and initialization order in
both project-shell modes. Venture's direct Qt launch captures application
diagnostics and rejects the CoreText family-alias scan warning.
