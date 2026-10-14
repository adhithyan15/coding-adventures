### Fixed — Windows direct-owner link rejection tests

- Use a directory junction for the linked owner root on Windows, so the
  rejection assertion runs without symlink privilege or Developer Mode.
- Prefer a real linked owner file when permitted; otherwise use a junction
  child and still assert that `lstat` sees a link before testing rejection.
- Keep the Linux CI file-symlink assertion mandatory, with the loader and its
  no-follow boundary unchanged.
