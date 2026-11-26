---
category: Testing & coverage
---

# A Java test appended to a file must remain inside its owning test class

When adding a test method to an existing Java test source, inspect the closing
brace before appending. A method placed after the class brace compiles as an
unnamed class and fails when the file has a package declaration. Insert it
before the final class brace and rerun the native test task.
