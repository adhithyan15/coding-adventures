---
category: Compiler / VM / language pipeline
---

# Every name a lowerer invents must be claimed from one shared pool, or two generators can invent the same name

**What happened.** `python-to-semantic-ir` invents four kinds of name:
parameter copies `__param_<p>_<N>`, temporaries `__chain_<N>`/`__for_<N>`,
lambdas `__lambda_<N>` and hoisted methods `<Class>__<method>`.  Each
generator was made collision-free against *some* other names: temporaries
skipped source names, and synthesised functions skipped source names and
already-pushed functions.  Neither skipped the other.  So the copy of a
parameter `a__b` and the method `b_0` of a class `__param_a` were both
`__param_a__b_0`; inside the function holding both, the local shadowed the
method and JavaScript threw `TypeError: __param_a__b_0 is not a function`
(#18771).  A hoisted method's name was also computed *before* its body was
lowered but recorded only *after*, so a temporary in the body could not see
it.

**Fix.** One set, `used_names`, seeded with every source name.  Every
generator goes through one helper, `try_claim_name`, which checks the set
(and the user `def`/`class` tables) and inserts the name the moment it is
chosen.  Order of invention no longer matters.

**Do differently.** When a second gensym is added, do not give it its own
"skip these" list.  Route it through the existing claim, and claim at the
point of choice, not at the point of use.  The test is a module that uses
every generator with each first candidate already taken, asserting that no
name is both a function and a variable.
