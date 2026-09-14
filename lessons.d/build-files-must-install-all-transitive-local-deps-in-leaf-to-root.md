---
category: BUILD files & dependency management
---

# BUILD files must install ALL transitive local deps in leaf-to-root order

Single most-recurring repo-wide failure (re-learned 8+ times across Python/Ruby/Go/TypeScript/Lua/Perl/Elixir/Rust). CI starts with empty `node_modules`/venvs, so every transitive sibling needs an explicit install line before the package's own. After adding a new low-level package, every package up the chain needs its BUILD updated. Use the scaffold generator — it computes the closure for you.
