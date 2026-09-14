# A generator that hardcodes a shard ordinal writes a DUPLICATE the moment `--shard` moves the stride

Bit me twice in one session, in two different scripts, and the second time it threw
`duplicate ES-EXT-382-VERB id` from inside the loader — after `--check` had already
passed once.

Sharded ledgers (`core/spine.d/`, `<track>/curriculum.d/{path,extensions,spine}/`) are
named `NNNN-<ID>.json` at a canonical stride of ten, and `--shard` **recomputes every
ordinal from scratch**. Insert a spine node at A1 and everything from `SPINE-SAY-WHAT-I-DO`
onward shifts by one slot, in `core/spine.d/` *and* in all 22 track ledgers. Any script
holding a literal `"0115-"` or `"3820-"` now writes beside the renamed file instead of over
it, and you have two shards claiming one id.

The failure is quiet in the direction that matters: the *stale* copy keeps its old contents,
so a re-run "succeeds", and the corruption only surfaces when something enumerates the
directory.

**Resolve a shard by its ID, never by its ordinal:**

```js
const hit = readdirSync(dir).find((f) => f.endsWith(`-${id}.json`));
return join(dir, hit ?? `${fallbackOrdinal}-${id}.json`);
```

**Generalisable check:** whenever a filename encodes both an identity and a position, a tool
that regenerates positions makes every hardcoded filename a time bomb. Address by identity;
let the position be derived. The same shape applies to `doc-shard` ordinals, migration
numbers, and anything else with a "canonical stride".
