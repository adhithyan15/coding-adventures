### Added - the Root Ledger, and the account it renders (HL-C83)

HL00 calls the etymology "the heart of the lesson... the signature of this
curriculum", and it is genuinely the strongest thing in the corpus. But a root
is only *useful* if it is spent again, which is what HL10 section 6.2's
`rootLedgerMinReuse: 3` says: a root may be taught only if at least three LATER
lessons draw on it.

The first measurement, across both etymology namespaces:

    2,717 roots
    2,624 spent fewer than three times   (97%)
    1,807 never spent at ALL             (taught once, never returned to)

Spanish alone: 303 roots, 290 underspent, 190 never spent. The best-spent root
in the entire corpus is `LA-ETYMON-SALVE-02`, at eight payoffs.

The etymology is real, it is good, and almost none of it is being spent. That is
the difference between a curriculum whose vocabulary compounds and one where
every lesson starts over -- and it is the machinery the friends layer (HL10
section 6.7) needs, since a root with recorded payoffs already knows which later
words it predicts.

**An introduction is not a payoff.** A root named in exactly one lesson scores
zero, not one. Counting the introduction would have started every root at 1 and
flattered the corpus by exactly the number of roots it has.

**Both namespaces, deliberately.** The corpus records etymology twice --
cross-language `roots:` slugs (1,966) that let a Spanish root and an Italian one
be recognised as the same root, and `<LANG>-ETYMON-*` atoms (751) that
participate in prerequisites and reinforcement windows. A ledger over only one
would report a root unspent while the other namespace was quietly spending it.

