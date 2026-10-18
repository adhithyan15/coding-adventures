---
category: Python
---

# Windows parity journal helpers must read explicit UTF-8

A one-off PR-tracking helper used Path.read_text() on the UTF-8 parity journal.
Python 3.10 on Windows selected cp1252 and rejected an existing Unicode byte
before making a change. Read and write repository JSON with encoding="utf-8"
explicitly (utf-8-sig only for externally BOM-prefixed receipts), and stop shell
follow-up mutations when the helper fails. Check lesson categories before using
`lessons.py new`; the attempted generic tooling category was not registered.
