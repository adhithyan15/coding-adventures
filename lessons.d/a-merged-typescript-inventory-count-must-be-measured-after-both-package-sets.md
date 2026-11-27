---
category: TypeScript / JavaScript
---

# A merged TypeScript inventory count must be measured after both package sets are present

The benchmark-root branch and fresh `origin/main` each carried a TypeScript portability test whose project total was 522, but their package sets were different. Keeping both sides added the unique root TypeScript packages and the main OS-architecture simulator; the merged tree measured 524 projects, 91 Node API/provider projects, and 537 locked compilers. Reusing either branch's expected totals made the merged test fail. Resolve inventory conflicts by preserving both package-specific evidence comments, then run the inventory against the merged tree and update the expected totals from observed results.
