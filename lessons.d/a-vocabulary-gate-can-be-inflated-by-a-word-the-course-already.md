# A vocabulary gate can be inflated by a word the course already owns

`vocabularyOf` counts distinct headwords on lessons whose `type` is `word`/`phrase`. That
restriction is correct — it stops drill titles and grammar labels being counted as
vocabulary. Its side effect is that **a lexeme introduced by a `grammar` lesson is owned
but uncounted.**

Spanish teaches `dar`. `ES-C65-di` introduces the atom `ES-LEX-DAR`, and its `type` is
`grammar`. So `dar` appears in no headword list, passes every duplicate test — no article,
no compound, no shared stem, no spent root — and adding it as a new `word` headword would
have **raised the A1 number while re-teaching a word the learner already had**.

That is the same failure direction as a near-duplicate, through a different door, and no
string rule of any sophistication reaches it.

**Check the atom ledger, not only the headword list.** `grep -l "ES-LEX-<WORD>" lessons/*.md`
is enough by hand. The permanent fix is in `validate.ts`: flag a new `word` lesson whose
headword matches a lexical atom another lesson already introduces.
