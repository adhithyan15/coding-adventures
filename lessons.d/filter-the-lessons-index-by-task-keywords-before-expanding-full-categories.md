---
category: Repo policy / workflow reminders
---

# Filter the lessons index by task keywords before expanding full categories

Filtering `lessons.py index` only by broad categories such as Rust, testing,
BUILD, and CI still produced hundreds of entries and truncated the result.  A
truncated prerequisite review hides exactly the lesson that may matter.

Start with task nouns and concrete operations (`x509`, `DER`, `certificate`,
`cargo`, `BUILD`, `security review`, `worktree`, or `PR watch`), inspect those
titles, and then open the matching shards in full.  Expand to a whole category
only when the focused search has no useful match.  This keeps the mandatory
lesson review complete enough to act on instead of turning it into an
unreadable inventory dump.
