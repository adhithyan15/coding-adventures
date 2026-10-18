---
category: Python
---

# Decode Git blob subprocess output explicitly as UTF-8 on Windows

During the Timeline typography audit, Python's subprocess.check_output(text=True)
decoded a Git UTF-8 source blob with Windows cp1252 and raised UnicodeDecodeError
before the requested source excerpt could be inspected. Request bytes and decode
with UTF-8 explicitly, or pass encoding="utf-8". Use bytes for saved whole-blob
identities so text decoding cannot change an inventory's hashes.

The first attempt to create this lesson also guessed an unsupported category
named tooling. Use one of lessons.py's declared category names; this lesson uses
Python. Neither failed read changed production source or validation evidence.
