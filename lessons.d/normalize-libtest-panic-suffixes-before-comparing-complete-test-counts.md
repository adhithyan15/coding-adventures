---
category: Rust
---

# Normalize libtest panic suffixes before comparing complete test counts

The normal Venn green helper saw 59 successful native tests in the actual
summary but parsed only58 status names: its expression omitted libtest's
` - should panic` suffix on an existing passing compatibility control. The
helper correctly withheld its aggregate even though both libraries and both
strict Cargo commands succeeded. Preserve that original exit1/false receipt.

Normalize the documented suffix, reject duplicate outer identities, and join
the complete status census to the actual running count and final summary.
With `--show-output`, stop collecting outer status lines before captured
success/failure bodies so nested child harnesses cannot inflate totals. A
separate corrected audit can establish what the immutable raw run actually
did; changing a parser never turns the earlier helper result into success.
