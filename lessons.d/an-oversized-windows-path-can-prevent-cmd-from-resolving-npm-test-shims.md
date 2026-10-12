---
category: Cross-platform & Windows BUILD_windows
---

# An oversized Windows PATH can prevent CMD from resolving npm test shims

During TypeScript Arc2D validation, PowerShell found `npm.ps1` and `node.exe`,
but `npm run build` could not resolve `tsc`, `npx vitest` could not resolve
`vitest`, and even `cmd /d /c where node` could not find `where`. The inherited
PATH was 11,821 characters; the packages and their local shims were installed.

Direct `node node_modules/vitest/vitest.mjs` and
`node node_modules/typescript/bin/tsc` passed. Repeating the actual npm scripts
with a process-local PATH containing Node, Windows system directories and Git
also passed. Use a bounded PATH for Windows verification when this symptom
appears; do not modify the user's persistent environment or reinstall a package
whose files are already present. Python capture of Vitest logs also needs
explicit UTF-8 decoding on Windows, since the default cp1252 codec cannot
decode its Unicode diagnostic output.
