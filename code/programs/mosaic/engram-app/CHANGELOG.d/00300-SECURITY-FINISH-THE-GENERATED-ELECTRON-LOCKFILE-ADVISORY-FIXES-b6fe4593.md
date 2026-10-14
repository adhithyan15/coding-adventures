### Security - finish the generated Electron lockfile advisory fixes

- Regenerate the emitted Electron lock under the seven-day age floor with shell-quote 1.11.0 and global-agent 4.1.3 overrides from the emitter. Update http-cache-semantics to 4.3.0 now that its release satisfies the floor. No known npm advisories remain.
- Check emitted overrides and prevent sprintf-js from returning through either generated lock.
