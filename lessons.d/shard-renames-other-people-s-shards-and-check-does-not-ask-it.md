# `--shard` renames other people's shards, and `--check` does not ask it to

`doc-shard-cli --shard` regenerates **every** filename from its heading text. Several
committed shards were named by hand and no longer match what the generator produces —
one whose heading contains a non-ASCII letter the slug drops, several committed before the
digest suffix existed. So a routine re-shard after a merge deleted and recreated three
files this branch never touched, twice, putting unrelated renames into a content diff and
manufacturing conflicts for whoever else was editing them.

`--check` compares the **bytes of the rebuilt document**, not filenames, and its own comment
says so explicitly: ordinals are author-chosen by design, so requiring canonical names would
break the promise that wedging an entry in at `00155-…` needs no renumber.

So after a re-shard: restore every shard that is not yours to its committed name, delete the
regenerated duplicates, and then run `--unshard` so the monolith is rebuilt from the shard
set actually on disk. Only the **ordinal** has to be right, because filename order is
document order.

**The generalisable half:** when a generator is idempotent in *content* but not in *naming*,
running it wholesale attributes other people's history to your commit. Regenerate the entry
you added; leave the ones you did not.

Doing that by hand is fine. Doing it with a helper that *infers* which shards are yours is
not — see "A helper that decides ownership by pattern will discard your own work
when you rename it" below, which is how I lost three of my own.
