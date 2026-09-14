# A `roots:` ledger only knows what a lesson CHOSE to declare — screen the PROSE too

Two candidate headwords came back clean from a three-way screen (headword list, atom
ledger, root ledger) and both were already taught. `ES-C288-vuelta` declares

```yaml
roots: []
```

and then spends *volvere*, *volume*-as-a-rolled-scroll and *revolve* across its gloss,
its `etymology_hook` and its body. `ES-C286-dolor` does the same with *dolere*,
*condolence* and *indolent*. Neither etymon is in the root ledger, because neither
lesson listed one — so `volver` and `doler` screened as free vocabulary, and a second
lesson telling the identical story would have shipped.

The root ledger is **opt-in metadata**. The prose is where the teaching actually is.

**Index the text as a fourth ledger**: gloss + `etymology_hook` + body, lowercased and
NFD-stripped so `saccāre` matches `saccare`, matched on word boundaries against each
candidate's proposed etymon *and* its intended English payoff. Over one track that is
~1,000 short documents — a regex sweep, not a search problem.

It caught six more on the same run: `llorar` (*plorare*, already told by `el llanto`),
`mover` (*movere*, by `el momento`), `pintar` (*pingere*, by `el pimiento`), `firmar`
(*firmus*, by `enfermo`), `curar` (*secure*, by `seguro`) and `dibujar` (by `el bosque`).
None was visible to any other screen.

**Expect false positives and read them.** An English-cognate hit may be an incidental
use of an ordinary word — "collocation" as a taught linguistics term, "increase" in a
plain sentence — rather than an etymological claim. The SOURCE-etymon hits are
decisive; the cognate hits are a prompt to go look. Auto-dropping on both costs good
candidates.

**Generalisable:** whenever a corpus has a structured ledger *and* free text that can
carry the same commitment, the ledger is a lower bound and the text is the truth.
Screening against the ledger alone measures how diligently authors filled it in.
