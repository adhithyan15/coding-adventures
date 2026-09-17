## HL-C386 — a headword is not always a glyph, so the corrected glyph check was wrong too

`HL-C383` found that `measureScriptClosure` credits a glyph to **any script
lesson whose BODY contains it**, and prescribed building the taught set from
**headwords** instead. That fix shipped as an audit helper and was load-bearing
in four PRs.

**It has the same bug one level up.** A script lesson's headword is not always a
glyph. Sometimes it is a whole word.

| lesson | headword | what it silently taught |
|---|---|---|
| `HI-W01-shirorekha-na-ma` | शिरोरेखा | **श** and **ो** |
| `HI-W06-name-sentence-stop` | । / पूर्ण विराम | **व** and **ू** |
| `HI-W05-virama-namaste` | नमस्ते | **ं** |
| `HI-A1F01-name-delayed` | B → अरुण | **ण** |

None of those four lessons teaches the glyph credited to it. The first is about
the head-line, the second about the danda, the third about the virama, the
fourth about copying a name into a form.

### What it cost

Every "undrawn goes N → M" figure reported in this campaign was **too
optimistic**, always in the same direction:

| reported | actual |
|---|---|
| 7 → 5 | higher |
| 5 → 4 | higher |
| 4 → 2 | higher |
| 2 → 1 | **7** |

And it nearly shipped a false exam-point closure: `HI-A1-SCR-15` was probed as
covered on the strength of a consonant series that looked complete and was not.
**ण, व and श are real debt** — *vah*, *shukriyā*, *vinatī* and *shām* are all
taught vocabulary — and the probe was withdrawn before merge.

### The rule that works

A glyph is **taught** when the lesson's headword is a **glyph inventory**: every
whitespace-separated token is at most two codepoints, a base plus an optional
combining mark. That admits `ौ`, `ज़ फ़ ख़ क़` and `।`, and rejects `शिरोरेखा`,
`नमस्ते` and `पूर्ण विराम`.

The predicate, so this does not have to be rediscovered:

```js
const isInventory = (hw) =>
  hw.replace(/◌/g, "").trim().split(/[\s/·]+/).filter(Boolean)
    .every((tok) => [...tok].length <= 2);
```

A glyph counts as taught only when its lesson's headword satisfies that **and**
the lesson is a script lesson. **This belongs in the repo**, next to whatever
eventually replaces `measureScriptClosure`.

### What to do

1. **Draw व and श.** Both are common, both are overdue, and neither has ever
   been a lesson subject.
2. Then ण. Then reconsider `HI-A1-SCR-15`.
3. `ङ` needs no Hindi lesson (`HL-C385`), and `ञ` appears only inside the
   conjunct ज्ञ, which `HI-W05-conjuncts` teaches.
4. **ू and ो are also undrawn**, which means `HI-A1-SCR-14` (the remaining
   mātrās) is further from closing than it looked.
5. When `measureScriptClosure` is finally fixed (`HL-C383`), fix it to this
   rule, not to the headword-contains rule.

### The general lesson

Both versions of this bug came from asking *"does the glyph appear in field X"*
when the real question is *"is the glyph what this lesson is about"*. **Appearing
somewhere is not being taught**, and that is true of bodies, headwords, and
anything else a future version reaches for.
