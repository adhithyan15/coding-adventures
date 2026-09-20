## Chapter 82 — opening a conversation

`ML-A1-S-07`, `ML-A1-F-35` and `ML-A1-REG-05` all close. Coverage **183/243 →
186/243 (77%)**.

The learner could count, describe, own, negate and grade — and could not
**start**. This chapter is the opening line, which is the thing a learner needs
first and usually gets last.

### The words were already there

The note said no term of address was taught. True, but the fix was cheaper than
it sounds, because **ചേട്ടൻ** and **ചേച്ചി** have been taught since the family
chapter.

**In Kerala the sibling words are used well outside the family** — a shopkeeper,
a driver, somebody a little older than you in a queue. **You address a stranger
as kin**, and that is ordinary politeness rather than a familiarity you have to
earn. English has nothing that works this way; *excuse me* names no relationship
at all.

### What was new is the calling form

| | about them | **to** them |
|---|---|---|
| older brother | **ചേട്ടൻ** | **ചേട്ടാ** |
| older sister | **ചേച്ചി** | **ചേച്ചി** |

**A word ending in -ൻ swaps that ending for -ാ when you call somebody with it.**
*cēcci* has no **-ൻ** to swap, so it stands as it is — stated as **a rule with
nothing to act on** rather than as an exception, because that is what it is.

### The register lesson teaches no word at all

`ML-A1-REG-05`'s note contained its own method: *"the evidence is in the
lessons' own register fields; the guidance is not in their prose."* So the
lesson states the rule those fields already encode.

| greeting | its own `register` field | reach |
|---|---|---|
| **നമസ്കാരം** | `respectful-neutral` | anyone, any hour |
| **സുപ്രഭാതം** | `formal` | the morning — **formal or not** |
| **ശുഭ മധ്യാഹ്നം** | `formal` | the afternoon |
| **ശുഭ സായാഹ്നം** | `formal` | the evening |
| **ശുഭ രാത്രി** | `formal` | the night |

**നമസ്കാരം is never wrong.** The **ശുഭ** family is written and announced more
than it is spoken. **സുപ്രഭാതം is the exception inside its own family** — its own
lesson already records it as used in *both* formal and informal contexts and as
the most general morning greeting — so anybody can say it before noon.

**Knowing a word and knowing when it is used are two different pieces of
knowledge**, and the corpus had given the first and not the second.

### A forward reference the full suite caught, and the real fix

A first version made `ML-C82-chechi-address` a **`word`** lesson with the
headword **ചേച്ചി**. The suite failed at `forwardReferences` 13 against a ceiling
of 12, naming a script lesson **255 lessons earlier**.

The cause is worth recording. `ML-C12-kudumbam` teaches six family words under
**one six-word headword**, and `continuity.ts` keeps a multi-word headword
**whole** rather than splitting it. So no lesson owned the bare token ചേച്ചി, and
the script lesson's use of it was **invisible debt**. Giving that token an owner
converted the debt into a visible forward reference.

**The fix was not to raise the pin.** This lesson does not teach the word — the
family chapter did. It teaches a **use**, and the new knowledge is a rule, so its
type is `grammar`. That is the accurate classification and it happens to be the
one that does not claim ownership of a token taught long before.

### Verified before writing

**ചേട്ടാ** returned **zero** files. **ചേട്ടൻ** and **ചേച്ചി** returned three each,
all citation uses — which is what made the chapter cheap, and also what created
the trap above.

### Metrics

| metric | before → after |
|---|---|
| exam-point coverage | 183/243 → **186/243 (77%)** |
| new words taught | **one** — ചേട്ടാ |
| `forwardReferences` | unchanged at 12 (13 at one point — see above) |

