## El jefe and la cabeza are one Latin noun, by way of France

The two-roads pattern again, but through a **third** language this time.
*Caput* wore down through Spanish speech into **la cabeza**; it was borrowed
from French *chef*, centuries later, as **el jefe**. *Rueda*/*rotativo* and
*consejo*/*consultar* were both Latin-to-Spanish — this one detoured, which is
why it resembles neither. A boss is literally a head, and *chief*, *chef* and
*jefe* are one word in three languages.

### The join key forced a choice, and the new guard watched it

`caput` is one of the 31 etymons the corpus spells more than one way:
`caput-latin` (on *acabar*) against `latin-caput` (on *la cabeza* and three
others). Nothing in CI catches a wrong pick — the guard shipped last night only
refuses a **new** spelling.

The rule applied was **match the etymon's existing majority**, because that is
what actually joins: `latin-caput` puts *el jefe* next to *la cabeza*, which is
the cousin a reader wants. `check:root-slug-splits` confirms no new split.

