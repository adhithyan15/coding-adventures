---
category: Testing & coverage
---

# Bind validation artifacts to the exact invocation that produced them

I copied a consumer log by a nearby filename and paired it with a different
compiler's result. The summary then claimed an invocation that the copied log
did not prove. Capture each command's executable, arguments, working directory,
source revision, exit code, and log path together. Select artifacts from that
captured invocation record, not from a glob or a familiar filename, and verify
the compiler and source identifiers in the raw log before reporting the result.
