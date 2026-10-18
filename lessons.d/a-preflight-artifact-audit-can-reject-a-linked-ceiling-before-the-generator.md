---
category: Testing & coverage
---

# A preflight artifact audit can reject a linked ceiling before the generator's write guard runs

Adding a preflight check for missing full-mock papers made the artifact CLI read
the existing ceiling before its write path. Two security tests still expected
the later `writeCeilingFile` directory-guard message, so they failed even
though the earlier `artifactExists` guard correctly rejected the junction and
no write occurred. Keep the safety assertion tied to rejection and unchanged
outside contents; accept the earlier, equally specific symlink diagnostic when
a new preflight changes which guard runs first. Never bypass the preflight to
make a message assertion pass.
