# asciidoc-demo

Live AsciiDoc preview web app. Type AsciiDoc on the left, see the rendered
HTML on the right — updated on every keystroke.

## What it does

- Split-pane React UI: AsciiDoc editor on the left, HTML preview on the right.
- Real-time rendering using our own `@coding-adventures/asciidoc` pipeline.
- Shows render time, character count, and word count in the footer.
- Dark theme matching the rest of the coding-adventures demo apps.

## Running locally

```bash
cd code/programs/typescript/asciidoc-demo

# Install dependencies (including transitive file: deps)
cd ../../packages/typescript/document-ast && npm install
cd ../document-ast-to-html && npm install
cd ../asciidoc-parser && npm install
cd ../asciidoc && npm install
cd ../../../programs/typescript/asciidoc-demo && npm install

# Start the dev server
npm run dev
```

Then open http://localhost:5173/coding-adventures/asciidoc/

## Building for production

```bash
npm run build
```

Output goes to `dist/`.
