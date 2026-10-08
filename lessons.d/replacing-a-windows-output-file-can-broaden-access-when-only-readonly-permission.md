---
category: Security boundaries
---

# Replacing a Windows output file can broaden access when only readonly permissions are copied

A native actual-process probe created an output with a protected owner-only DACL inside a parent with inheritable BUILTIN Users read access. Compilation succeeded but replacing the file inherited Users read access instead of preserving the original DACL. An independent reviewer reproduced it. Rust `Permissions` on Windows represents readonly state, so copying it does not preserve access controls. Staging directories also inherit parent ACLs; Windows traversal bypass means restrictive directory access alone is insufficient once a child file becomes broadly readable. Preserve the relevant file security policy with checked platform APIs and keep staged files private before installation. Do not treat passing ordinary permissions/read-only tests as DACL preservation evidence.
