---
category: Swift
---

# Every Swift package must `.gitignore` `.build/` and `.swiftpm/` BEFORE the first `swift test`

The directories contain thousands of deeply nested files that break Windows CI with "Filename too long".
