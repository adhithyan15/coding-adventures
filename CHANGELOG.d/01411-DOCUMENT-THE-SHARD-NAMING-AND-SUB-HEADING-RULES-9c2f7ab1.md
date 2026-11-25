### Document the shard naming and sub-heading rules

- `CHANGELOG.d/_meta.md` now states the shard filename rule
  (`NNNNN-SLUG-HEX8.md`: five-digit rank, uppercase ASCII slug, eight lowercase
  hex digits), that zero is not a valid rank, and that `_meta.md` is the only
  reserved name. The rule was already enforced by `isValidDocShardName` in
  `code/packages/typescript/human-language-data/src/doc-shard.ts`, but the
  directory's own entry point did not mention it, so a lowercase slug or a
  seven-character digest failed CI with nothing on disk to read first.
- `lessons.d/_meta.md` now states that a lesson's title is `#` and a sub-heading
  inside its body is `###`, not `##`. The renderer demotes every body heading by
  one level, so a `##` sub-heading becomes `###` and collides with the title,
  making the aggregate's outline claim more lessons than exist and failing
  `test_render_has_exactly_one_heading_per_lesson`.
- Both rules were found by losing a CI cycle to them: the changelog name on
  #19849 and the lesson sub-heading level on #19862. Documenting them next to
  the directories removes the same cost from the next author.
