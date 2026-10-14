### Security - remove vulnerable Electron development dependency paths

- Generated Electron projects override shell-quote to patched 1.11.0 (published 2026-09-29) and global-agent to 4.1.3. Both satisfy Engram's seven-day resolution floor; concurrently remains 10.0.4. This clears GHSA-pqg4-j6r4-53mv and removes the optional roarr / sprintf-js chain without disabling proxy bootstrap.
- Extend the emitted-project regression test to require these overrides.
