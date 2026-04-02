/**
 * seed.ts — First-visit example entry.
 *
 * On first launch (no entries in storage), the app seeds one example entry
 * to demonstrate the editor and GFM rendering. The entry showcases headings,
 * bold, italic, links, code, blockquotes, lists, and task lists — giving
 * the user a quick reference for GFM syntax.
 */

import type { Store } from "@coding-adventures/store";
import type { AppState } from "./types.js";
import { entryCreateAction } from "./actions.js";

const WELCOME_CONTENT = `# Welcome!

This is your first journal entry. It's written in **GitHub Flavored Markdown** — the same format used for READMEs and documentation.

## What you can do

- Write entries in markdown
- See a live preview as you type
- Organize entries by date

## Formatting examples

Here are some things you can write:

- **Bold text** and *italic text*
- [Links](https://example.com) and images
- \`Inline code\` and code blocks
- > Blockquotes for emphasis
- Task lists:
  - [x] Create journal app
  - [ ] Write first real entry

Happy journaling!`;

export function seedEntries(appStore: Store<AppState>): void {
  appStore.dispatch(
    entryCreateAction("Welcome to Your Journal", WELCOME_CONTENT),
  );
}
