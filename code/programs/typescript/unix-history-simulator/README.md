# Unix History Simulator (browser)

An original browser model of a file read inspired by First Edition Unix. See
[HOS00](../../../specs/HOS00-historical-os-simulator.md) for historical scope.

With Node.js 22.12+ or a newer supported Vite runtime:

```sh
npm ci
npm run dev
```

Open the URL printed by Vite. **Step** executes one modeled syscall. **Run**
executes the rest of the loaded command. Load `cat /hello`, `cat /empty` or
`ls /etc` to compare successful reads, immediate EOF and directory lookup.
`npm run build` builds static files into `dist/`.

This program renders the console, syscall trace and open descriptors.
The reusable simulator lives in the [TypeScript library](../../../packages/typescript/unix-history-simulator).
Run `npm test` to test the browser controls in a DOM environment.
