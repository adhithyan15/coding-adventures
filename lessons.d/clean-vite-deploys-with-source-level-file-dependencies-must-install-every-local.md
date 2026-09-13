---
category: Workspace & package metadata
---

# Clean Vite deploys with source-level `file:` dependencies must install every local source package in dependency order

Vite follows `main: src/index.ts` links back into the repository, so each package needs its own `node_modules` on a fresh runner. `npm install --install-links` is not a portable shortcut: npm 11 packed the recursive graph in a Windows repro, but GitHub's Node 20/npm 10 runner stopped at a nested `file:` dependency with ENOENT. Mirror the full production closure explicitly, then run the app's normal install and prove it on the deployment runner.
