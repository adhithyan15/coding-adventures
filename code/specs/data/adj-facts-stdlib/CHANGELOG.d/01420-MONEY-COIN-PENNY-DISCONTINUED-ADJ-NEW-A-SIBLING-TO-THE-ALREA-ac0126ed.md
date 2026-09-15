- `money/coin-penny-discontinued.adj` (new) — a sibling to the already-shipped `us-coins.adj`
  (`coin_cents(coin, cents)`, ONE cent-value per coin). That table's own header already quotes,
  verbatim, the penny's U.S. Mint Coin Classroom span ("The one-cent coin ceased circulating in
  2025 after 232 years of production."), and packed inside that SAME quote, alongside the cent
  value the parent table already captures, are two more atomic facts the value-only schema had no
  room for: a discontinuation status/year (2025) and a production span (232 years). New
  `coin_status(coin, status, year, production_years)` table: penny → (discontinued, 2025, 232).
  Deliberately a SINGLE row — every other coin carries no row at all, since none of their own
  cited U.S. Mint spans states any circulation-status change; abstention by simple absence, not an
  invented `circulating` status. New e2e test file `facts_coinpennydiscontinued_e2e.rs` (3 tests:
  forward recall with citation, recall with status pre-bound, honest abstention on other coins). No
  manifest objective, matching `us-coins.adj`'s own precedent of not having one. Second and final
  strong slice from the money/ sweep tranche — money/ (2 tables) is now EFFECTIVELY CLOSED (2/2
  strong candidates shipped); a moderate candidate (`coin-collectible-denominations.adj`, us-coins
  .adj's own everyday-vs-collectible partition) was deliberately not pursued, judged too thin
  (requires inferring a two-way classification from one descriptive sentence).
