---
category: Repo policy / workflow reminders
---

# Pass PowerShell Select-String pattern before context switches

`Select-String` binds `-Pattern` to the following argument. Put the pattern
first, then options such as `-Context 1,5`, e.g.
`Select-String -Pattern 'not ok|Failed test' -Context 1,5 file.txt`.
