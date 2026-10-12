---
category: Cross-platform & Windows BUILD_windows
---

# Read persistent UTF-8 checkpoints with explicit encoding in Windows Python

Python on this Windows host defaults text files to cp1252. A checkpoint updater
used `Path.read_text()` on a UTF-8 state file, then wrote UTF-8, corrupting
historical Unicode notes even though JSON still parsed. A later default-codec
read exposed the corruption. The faulty local checkpoint was never committed
or pushed.

Recover the historical text from `git show HEAD:<path>` decoded explicitly as
UTF-8, apply only the current small metadata update, and verify every older
owner is unchanged before writing. Always use `encoding="utf-8"` for both
reads and writes of persistent checkpoints, and explicit UTF-8 decoding for
subprocess captures. JSON validity alone does not prove Unicode preservation.
