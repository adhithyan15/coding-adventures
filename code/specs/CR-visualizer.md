# CR-Visualizer — Cryptography Visualizer

## Overview

An interactive web application for exploring cryptographic algorithms visually.
The visualizer follows the renderer-first pattern established by other apps in
the monorepo (code39-visualizer, arithmetic-visualizer, etc.): core computation
happens in the `@coding-adventures/caesar-cipher` and
`@coding-adventures/atbash-cipher` packages, and the React app renders the
intermediate structures for learning.

## What It Shows

### Input Panel
- Plaintext text area for user input
- Cipher selector dropdown (Caesar, Atbash — more added as the CR series grows)
- Key/shift controls (shown only for ciphers that have a key, hidden for Atbash)
- ROT13 quick-toggle button (shown only for Caesar)

### Substitution Table
- Full 26-letter A→Z mapping grid for the selected cipher and key
- Highlights the letters present in the current plaintext
- Updates live as the cipher or key changes

### Step-by-Step Panel
- Shows each character being transformed one at a time
- For each letter: original → position → formula → new position → encrypted
- Non-alpha characters shown with a "pass-through" indicator

### Output Panel
- The encrypted ciphertext, updating live as input changes
- Copy-to-clipboard button

### Frequency Analysis Panel (Caesar only)
- Bar chart comparing ciphertext letter frequencies to English frequencies
- Brute-force results panel showing all 25 candidate decryptions
- Best-guess highlight from chi-squared analysis

## Technical Stack

- **Framework**: React 19 + TypeScript
- **Build**: Vite 6
- **Testing**: Vitest + @testing-library/react
- **Styling**: Lattice (transpiled in-browser via lattice-transpiler)
- **Deploy**: GitHub Pages at `/coding-adventures/crypto/`

## Dependencies

- `@coding-adventures/caesar-cipher` — encrypt, decrypt, rot13, brute_force, frequency_analysis
- `@coding-adventures/atbash-cipher` — encrypt, decrypt
- `@coding-adventures/lattice-transpiler` — CSS-in-Lattice for styling
- `react`, `react-dom` — UI framework

## Design Notes

### Color Palette
Follow the monorepo's warm-paper aesthetic (matching code39-visualizer):
- Paper background, ink text, accent for interactive elements
- Green for valid/encrypted output, red for errors
- Muted tones for metadata and labels

### Responsive Layout
- Desktop: side-by-side panels (input left, output right, analysis below)
- Mobile: stacked panels

### Extensibility
The cipher selector is designed to grow as the CR series expands. Each cipher
module exports a common interface (`encrypt`, `decrypt`, and optional
`bruteForce`, `frequencyAnalysis`), so adding a new cipher is just:
1. Add the package dependency
2. Add an entry to the cipher registry in the app
