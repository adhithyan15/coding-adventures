# Lesson shards use exactly one Markdown heading

A new lesson shard used second-level headings to separate its problem and
remedy. The shard validator accepted the filename and title, but the rendered
aggregate contract failed because each shard must contribute exactly one
Markdown heading.

Keep the title as the shard's only Markdown heading. Separate any internal
sections with plain paragraphs or bold text, and run both
`python3 code/scripts/lessons.py validate` and the complete `test_lessons.py`
unit suite before pushing a lesson change.
