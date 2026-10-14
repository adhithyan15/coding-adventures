### Fixed — uppercase ownership pin fixtures on case-insensitive filesystems

- Replace the lowercase map entry before writing the uppercase pin fixture, so NTFS and ordinary APFS retain the intended uppercase filename. Verify each malformed fixture's exact directory inventory before loader rejection. Preserve all ownership baselines, malformed-content and symlink checks.
- Windows CI built 533 packages but failed this single case; the repaired fixture directly reproduces and rejects the uppercase name on NTFS. Local ownership checks reach the unchanged file-symlink fixture, which requires privileges unavailable on this host.
