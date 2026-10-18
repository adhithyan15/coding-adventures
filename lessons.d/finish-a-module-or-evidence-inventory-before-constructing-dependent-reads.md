---
category: Repo policy / workflow reminders
---

# Finish a module or evidence inventory before constructing dependent reads

The Timeline size audit tried a guessed split IR module, a guessed temporal
tests directory, and guessed evidence filenames. Those reads failed because
the actual IR is in lib.rs, temporal contracts are src modules, and saved
inventories use different names. No source or test result was inferred from
the failures.

Run rg --files or inspect the saved manifest first, consume that result, then
construct the dependent read from its exact returned path. Do not batch an
inventory with reads whose names are still guesses. Keep broad font searches
out of monolithic modules when a focused Timeline function boundary is known.
