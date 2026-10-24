---
category: Repo policy / workflow reminders
---

# After a context mismatch, patch one file and one hunk

Even a retry containing one exact hunk can fail atomically when it is bundled
with an unverified hunk from another file. After any `apply_patch` context
mismatch, the next edit must target exactly one file and one freshly copied
hunk. Verify that edit before touching another section or file.
