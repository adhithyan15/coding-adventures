---
category: TypeScript / JavaScript
---

# vite-node requires a script file and does not accept node-style inline evaluation

An inspection tried `npx vite-node -e '…'` as though vite-node shared Node's
inline-evaluation flag. vite-node interpreted the invocation as having no input
file and printed its usage instead. Use a focused Vitest assertion or a
temporary script file for TypeScript modules that need Vite resolution; do not
assume Node CLI flags are accepted by adjacent runners.
