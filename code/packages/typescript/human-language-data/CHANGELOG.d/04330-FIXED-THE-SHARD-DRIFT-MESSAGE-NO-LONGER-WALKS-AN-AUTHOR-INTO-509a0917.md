### Fixed - the shard drift message no longer walks an author into data loss

- `--check`'s stale-monolith message used to say only "Run `npm run unshard`".
  That was correct while `core/spine.json` was the only sharded ledger, because
  its monolith is purely derived and rebuilding it from the shards is always the
  recovery.
- It stops being correct for an **authored** ledger. `<track>/chapters.json` is
  read, appended to, and written back wholesale by the Python authoring scripts
  in `learning/human-languages/data/scripts/` (`author_adjective_wave.py`,
  `author_deixis_wave.py`). An author who adds a chapter that way and then
  follows a bare "unshard" **discards the chapter they just wrote**, because
  unshard overwrites the monolith from shards that never saw it.
- The message now names both directions and lets the author pick: `npm run
  unshard <path>` if they edited the shards, `npm run shard <path>` if they
  edited the monolith, and "do not hand-merge either side". The tool cannot know
  which edit was intended; the person who just made it can. A drift message that
  destroys the reader's work is worse than no message.
- Those scripts are otherwise unchanged and still work: `check:shards` catches
  the desync loudly rather than letting the loader — which prefers `.d/` —
  silently ignore the new chapter.

