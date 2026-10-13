---
category: Testing & coverage
---

# Listening source-band tests must count only the spoken scored script

The first Sanskrit C1 test counted the entire confidential listening part,
including the unscored orienting preview and unspoken scoring markers, against
the scored duration band. Mock 1 passed only because those excluded tokens hid
four short recording masters. Delimit the spoken scored script explicitly,
count only that slice, and add a regression that previews and confidential
markers cannot satisfy a source-length minimum.
