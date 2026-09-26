# Replace a file with one update operation in apply_patch

An `apply_patch` request tried to delete and add the same path in one patch.
The patch engine rejects multiple operations targeting one file. Replace a
small file with a single `Update File` hunk that removes the old contents and
adds the new contents, or split deletion and creation into separate calls when
that is genuinely necessary.
