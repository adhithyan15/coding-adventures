### Fixed — the shell-free local dependency installer now runs on Windows

`npm ci` in a clean Windows checkout reached `local-deps.mjs` and failed before
installing any sibling with `spawnSync npm.cmd EINVAL`: Windows command shims
cannot be launched by `execFileSync` without a shell. The postinstall now asks
npm for its JavaScript CLI entry point and runs that fixed argv through the
current Node executable. This preserves the no-shell argument boundary, the
leaf-first reproducible installs, and the recursion guard while making the same
clean install used on other platforms work on Windows.
