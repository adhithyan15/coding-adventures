### Changed — nine more track changelogs have direct owners (#17700)

- Register the Arabic, Chinese, French, German, Italian, Latin, Persian,
  Portuguese, and Russian changelogs as newest-first level-2 document shards.
- Preserve every pre-migration byte while replacing each tracked aggregate with
  append-only release-note owners and an ignored local projection.
- Extend repository policy so owner deletion and aggregate resurrection fail
  closed for all nine tracks.
