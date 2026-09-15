---
category: BUILD files & dependency management
---

# Shared-infrastructure changes cascade

Editing `grammar-tools`, `lexer`, etc. marks 50+ dependents for rebuild. Use `--list-affected` first; expect that any pre-existing broken BUILDs anywhere in the closure will surface.
