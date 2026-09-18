### Fixed — `HL-C400`: four lessons said the anusvara goes whenever anything attaches

- No coverage change. Malayalam A1 stays at 208/243 (86%).

`HL-C400` was filed while writing chapter 96 and has been open since. Four
lessons told the learner that a word-final **ം** is dropped *"the moment
anything is added"* — and the corpus breaks that four ways.

#### What each site now says

| lesson | now reads |
|---|---|
| `ML-C83-keralathil` | *"the moment **a case ending** is added"*, plus *"Other kinds of ending do other things to it, and you will meet them in their own chapters."* Its recall asks *"when a case ending is added"*. |
| `ML-R83-from-recall` | *"dropped the moment **a case ending** is added"* |
| `ML-C91-vila-panam` | body and recall both say *"when **a case ending** attaches"* |
| `ML-C92-bhakshanam` | see below |

**`ML-C92-bhakshanam` was the worst of the four and the easiest to miss.** It
demonstrated *nothing* — it only asserted *"you already know how it will behave
when anything attaches to it"*, pointing at knowledge the learner did not have.
It now names what it does know: *"you already know what a **case ending** will
do to it — the **ം** gives way to **-ത്ത-**, as it did on **കേരളം** and
**പുസ്തകം**."*

#### Method notes, after two PRs where the method was the thing that failed

- **The sweep was run wide before trusting the shard's list** — script *and*
  romanization, per the census rule that `HL-C402` cost me. It returned exactly
  these four lessons and nine lines. **No fifth site.**
- **The recall partners were checked**: `ML-R91-shopping-recall` and
  `ML-R92-eating-out-recall` are clean. **Null result stated**, because `HL-C402`
  was the PR where a recall partner's table survived its own drill's correction.
- **"Case ending" is not new metalanguage.** `ML-C06-dative-ikku` uses it at
  sequence **320**, long before any of these sites, and `ML-C96-eluppam` (3410)
  already models the exact phrasing: *"A case ending takes the **ം** away and
  puts **ത്ത** in its place."*
- **The whole-token owner sweep** over the four edited files returns only
  inflected forms of owned stems (**ഇന്ത്യയിൽ**, **പുസ്തകത്തിൽ**,
  **പുസ്തകത്തിന്റെ**, **ഊരിൽ**) and the metalinguistic **-ത്ത-** fragment.

#### What was deliberately not done

The shard's own advice: **do not teach the four-way picture in chapter 83.** The
learner has met one of the four at that point, and `ML-C96-eluppam` already
states the full account where it belongs. The early lessons simply stop claiming
more than they show.
