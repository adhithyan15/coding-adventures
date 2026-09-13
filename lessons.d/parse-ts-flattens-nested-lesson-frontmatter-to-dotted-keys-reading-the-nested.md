# `parse.ts` FLATTENS nested lesson frontmatter to dotted keys — reading the nested shape returns an empty ledger, silently

Screening sixteen candidate headwords against the atom ledger, I wrote:

```js
const kn = lesson.frontmatter.introduces?.knowledge;   // always undefined
```

The lesson file really does say

```yaml
introduces:
  knowledge: [ES-LEX-BEBER]
```

but the parser stores it as the literal key `"introduces.knowledge"`. `frontmatter.introduces`
is `undefined`, the `?.` swallows it, and the screen loaded **zero atoms** — so every
candidate came back clean, including any word the course already owned through a `grammar`
lesson. That is the exact failure the atom-ledger screen exists to prevent, arriving through
the screen itself.

**What caught it was a two-directional self-test**, not the code review:

```js
if (!atoms.has("ES-LEX-BEBER")) fail("atom ledger did not load");
if (atoms.has("ES-LEX-LAVAR"))  fail("atom ledger loaded something impossible");
```

The first assertion fired. A one-directional "did I load anything?" check would have passed
on a ledger of zero, because zero *is* something.

**Rules:** confirm a parser's storage shape by printing `Object.keys(...)` before indexing
into it; and every screen that compares against a corpus needs a positive control *and* a
negative one, because "found no problems" and "looked at nothing" produce identical output.
