---
category: Repo policy / workflow reminders
---

# Filling a new lesson requires removing its generated placeholder

The first CLOC34 graph-event lesson inserted its explanation after the title
but left the generated TODO at the bottom. lessons.py validate correctly found
one problem among 1,398 shards; the written prose did not make the file complete.

Replace the exact generated placeholder with the explanation instead of adding
text beside it. Read the entire short file and run lessons.py validate before
treating the documentation gate as complete. Removing the placeholder can also
leave an extra blank EOF line: retain exactly one final newline and check the
diff before committing. Retain failed gates as failed.
