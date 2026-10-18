---
category: Python
---

# Insert complete preparer function blocks separately from call anchors

The minima publisher v3 deriver mixed a triple-quoted function block with
a partial call anchor and raised SyntaxError before running or writing its
successor. The original failed preparer is retained. No Rust, publication
or repository mutation happened in that attempt.

Keep the complete function block in one string and concatenate the original
uniquely checked call anchor separately. Compile the complete successor
Python source before writing it, then independently review the full delta.
