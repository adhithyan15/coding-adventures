## Unreleased — Route Elixir and Haskell historical Unix inputs

- **Fix: include exact Elixir and Haskell fixture-adapter inputs in selection and cache keys.** The shared historical Unix corpus and replay helper select the two packages; each adapter source selects only its matching consumer. The Elixir package uses its discovered underscore name `elixir/unix_history_simulator`. The Haskell adapter's Cabal manifest and `Main.hs` are both declared. No broad fixture-directory route was added.
