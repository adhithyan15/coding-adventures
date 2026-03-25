# Changelog — engram-docs

## 0.1.0 — 2026-03-25

Initial release.

### Added

- **Hero section** — app tagline, "Try in browser" link to the live web app,
  "Download desktop app" scroll-to-download CTA, flashcard stack visual showing
  the study UI with SM-2 rating pills.

- **Features section** — six feature cards: SM-2 scheduling, 50 starter cards,
  offline-first, web + desktop, four-rating system, per-deck statistics.

- **How it works section** — SM-2 algorithm explained: four rating cards
  (again/hard/good/easy) with color-coded effects and interval examples;
  six-review timeline showing a card growing from 2 days to 118 days to
  demonstrate long-term interval growth; ease-factor range note.

- **Run locally section** — tabbed: Browser (4 steps: prerequisites, clone, install
  packages, run dev) and Desktop/Electron (3 additional steps). Each step has a
  copy-to-clipboard code block.

- **Download section** — macOS, Windows, and Linux platform cards with installer
  formats and installation notes. Link to GitHub Releases filtered by `engram-v*`
  tag. "Open in browser" fallback CTA.

- **Dark/light theme toggle** — persists to localStorage, respects system preference
  on first visit.

- **`deploy-engram-docs.yml`** — deploys to
  `https://adhithyan15.github.io/coding-adventures/engram-docs/` on every
  push to main.
