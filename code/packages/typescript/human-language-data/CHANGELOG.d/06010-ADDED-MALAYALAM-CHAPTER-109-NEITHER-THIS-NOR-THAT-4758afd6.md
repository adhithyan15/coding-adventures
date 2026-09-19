### Added — Malayalam chapter 109, neither this nor that

- `ML-A1-NEG-03` closes. Malayalam A1 coverage 216/243 -> **217/243 (89%)**, 26
  points unmapped.

#### The note named a blocker that no longer exists

> Untaught, and it depends on the missing coordinator at `ML-A1-JOIN-01`.

That was written before chapter 70, which teaches **-ഉം** across four lessons
(2390–2420). The coordinator arrived and the note was never re-read — failure
mode 3, a note simply false about what is taught, and the cheapest kind of point
to close once spotted.

#### The construction was censused by shape, not by the English word

Grepping *neither* finds only ordinary English prose — a sound description in
`ML-C33-ezhutuka`, a remark in `ML-C106-ordering`. The real check is a regex for
two **-ഉം**-marked words followed by a negative on the same line:

```
lines with two -um words then a negative: 0
```

Genuinely untaught.

#### It costs no new vocabulary, which is the shape of the point

Malayalam builds *neither … nor* from **-ഉം** on every item plus a negative at
the end, and both halves are long taught: **ഇല്ല** since sequence 40, **അല്ല**
since chapter 77. Two atoms, three lessons, **no new headword** — and the recall
asks the learner outright how many new words the chapter cost.

#### The second negative is why this is two lessons and not one

`ML-C77-alla` teaches that Malayalam makes you pick a negative where English
spends one word on both, and English's *neither … nor* hides that choice
completely:

| | |
|---|---|
| **ചായയും കാപ്പിയും ഇല്ല** | neither is **there** |
| **അധ്യാപകനും വൈദ്യനും അല്ല** | neither is **so** |

Identical on the left, different only in the last word. The chapter continues
`ML-C77-alla`'s own line that Malayalam negates twice and English only once.

#### A claim in the pin comment was wrong, and the census test caught it

The first draft said a chapter teaching a *construction* out of owned pieces
adds no headword and therefore no glyph. The headwords here are whole Malayalam
sentences, so they **do** enter the glyph census:

| glyph | before | after |
|---|---|---|
| **ധ** | 7 fields / 6 tokens | **8 / 7** |
| **ൈ** | 5 / 5 | **6 / 6** |

**അധ്യാപകനും** and **വൈദ്യനും** are *inflected* forms of taught words and count
as fresh tokens even though no new word is taught. What does hold is the part
that matters: `shown` stays **68** and the directly-owned overlap stays **59**,
so no **new** glyph is opened — two already-open ones merely deepen.

#### Verification

`npm run validate` 21/21 (first run), all twelve gates, the full suite (145
files, **2097 passed**, 1 skipped), `check-book-compile.sh --strict malayalam`,
all six LaTeX warning counters at zero, and the append-only shard check empty.

`ML-A1-JOIN-04` (distributive) and `ML-A1-JOIN-01`'s clause half remain open and
are **not** claimed here; this point is negative coordination only.
