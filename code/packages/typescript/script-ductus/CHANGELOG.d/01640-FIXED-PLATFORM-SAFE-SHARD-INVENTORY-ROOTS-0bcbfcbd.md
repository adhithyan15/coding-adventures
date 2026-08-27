### Fixed — platform-safe shard inventory roots

- Convert Vite config file URLs with `fileURLToPath`, preserving valid Windows
  drive paths when the shard-native inventory plugin resolves its roots.
- Restore local Vitest startup on Windows without weakening the plugin's
  realpath confinement, and make its watcher and reparse-point tests portable.
