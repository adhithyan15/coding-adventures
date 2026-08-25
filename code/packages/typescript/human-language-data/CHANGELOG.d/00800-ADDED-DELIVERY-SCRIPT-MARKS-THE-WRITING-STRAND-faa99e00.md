### Added — `delivery: script` marks the writing strand

The eleven writing lessons declare `delivery: script` in their frontmatter, so a
spoken-only edition can filter on one field instead of inferring the strand from `type`,
`skills` or a computed modality. A new test pins the marker to exactly the writing
lessons of any track that adopts it, so a writing lesson that forgets it — or a speaking
lesson that gains it by copy-paste — fails rather than shipping in the wrong edition.
Script material *inside* a speaking lesson is already typed at the block level
(`block.type === "script"`), so a spoken edition can drop those blocks with no new
metadata.

