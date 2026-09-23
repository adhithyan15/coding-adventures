---
category: Repo policy / workflow reminders
---

# Copying one sibling record teaches you its values, not its invariant

Wiring four new Spanish chapters meant writing 29 curriculum-membership shards.
I read one existing shard to learn the shape:

```json
{ "id": "ES-C447-analisis", "pathSegment": "ES-PATH-447-JUICIO",
  "pathOrder": 0, "extensions": [{ "id": "ES-EXT-447-JUICIO", "order": 0 }] }
```

I varied `pathOrder` across the set, because that field's name says it is an
ordering. I left `extensions[0].order` at `0` everywhere, because that is what
the example I had read contained.

`npm run validate` rejected it:

```
curriculum extension 'ES-EXT-448-MUDANZA': lesson 'ES-C448-estropear'
  has order 0, expected 1
```

`exactDenseOrder` in `src/curriculum-membership.ts` requires both fields to be
dense `0..n-1` across the set. The example I copied had `order: 0` because it
was the **first** lesson in its extension, and the value I generalised from was
a coincidence of which record I happened to open.

**One record cannot distinguish a constant from a variable.** Every field in it
holds some value, and nothing in the file says which of those values is fixed
for all siblings and which is a position in a sequence. I copied a sample of
one and read a `0` as a constant.

The fix costs nothing: when copying a record's shape, **read two or three
siblings, not one**, and specifically read one that is NOT first in whatever
sequence it belongs to. Any field that differs between them is a variable you
now have to reason about. Here, dumping the whole `ES-C447-*` set side by side
would have shown `order` climbing 0..5 next to `pathOrder`, and the invariant
would have been visible in one line of output:

```bash
for f in spanish/curriculum-membership.d/ES-*447*.json; do
  python3 -c "import json,sys;d=json.load(open(sys.argv[1]));\
print(d['id'],d['pathOrder'],[e['order'] for e in d['extensions']])" "$f"
done
```

The gate caught this one, so it cost a minute. The same reasoning applied to a
field no gate checks — a `stage`, a `kind`, a `category` — would have shipped.
