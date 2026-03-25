# engram-docs

Documentation site for [Engram](../engram-app) — the open-source spaced repetition flashcard app.

Deployed at:
**https://adhithyan15.github.io/coding-adventures/engram-docs/**

## Sections

| Section | Description |
|---|---|
| Hero | App summary, "Try in browser" and "Download desktop" CTAs |
| Features | SM-2 scheduling, offline-first, 50 starter cards, web + desktop |
| How it works | SM-2 algorithm — ratings, interval growth, example timeline |
| Run locally | Step-by-step instructions for browser dev and Electron dev (tabbed) |
| Download | macOS / Windows / Linux installers with links to GitHub Releases |

## Running

```bash
cd code/programs/typescript/engram-docs
npm install
npm run dev
# http://localhost:5173/
```

## Building

```bash
VITE_BASE=/coding-adventures/engram-docs/ npm run build
# Output: dist/
```

Deploying to GitHub Pages is handled automatically by `deploy-engram-docs.yml`
on every push to main that touches files under this directory.
