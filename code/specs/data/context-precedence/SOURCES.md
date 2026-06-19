# Sources — grounded context-precedence edges (ADJ73 PR-B-3)

Every `outranks_context` edge in [`context-precedence.adj`](context-precedence.adj) is
byte-provenanced: it carries a `source` (a verbatim quote of its charter), a `locator`, and a
`trust` tier. This ledger records where each quote came from, so the edge is auditable and one
CAS edit from correctable. Nothing here is human-authored doctrine — each edge is the literal
text of a primary/authoritative source.

| Edge | Charter | Verbatim quote (excerpt) | Locator | Trust | Retrieved |
|------|---------|--------------------------|---------|-------|-----------|
| `federal > state` | U.S. Constitution, Supremacy Clause | "This Constitution, and the laws of the United States which shall be made in pursuance thereof; … shall be the supreme law of the land; and the judges in every state shall be bound thereby, anything in the Constitution or laws of any State to the contrary notwithstanding." | U.S. Const. art. VI, cl. 2 | authoritative | 2026-06-17 |
| `ninth_circuit > district_court` | Vertical stare decisis | "a federal circuit decision is binding on all federal district courts within its circuit, but not federal courts in other circuits" | WCL 1L Legal Research Primer — Binding v. Persuasive Authority | authoritative | 2026-06-17 |

## Retrieval URLs

- Supremacy Clause (verbatim): <https://www.law.cornell.edu/constitution/articlevi> (Legal Information Institute, Cornell Law School).
- Vertical stare decisis (circuit binds district within circuit): <https://wcl.american.libguides.com/1Lresearch/authority> (American University, Washington College of Law — *Binding v. Persuasive Authority*, 1L Legal Research Primer). Corroborated by LII Wex, *stare decisis*: <https://www.law.cornell.edu/wex/stare_decisis>.

## Deliberately NOT included here (deferred to PR-B-4 as grounded meta-rules)

The other classical conflict-resolution canons are *derivable* from rule attributes and so must
be grounded **meta-rules**, not bare edges duplicated per pair:

- **lex posterior** (recency): a newer enactment/guideline supersedes an older one — e.g.
  `idsa_2024 > idsa_2004`. This is a function of each rule's enactment/publication date, not a
  standalone authority hierarchy; deriving it keeps the order honest (an edge that can be derived
  should be derived).
- **lex specialis**: the more specific rule controls the more general.
- **appeal status**: a reversed/vacated holding loses precedential force.

Each will itself be a byte-provenanced rule (citing the canon's authority) that *derives*
`outranks_context` / defeats from typed, provenanced rule metadata — the recursive structure
ADJ73 §7 calls for.
