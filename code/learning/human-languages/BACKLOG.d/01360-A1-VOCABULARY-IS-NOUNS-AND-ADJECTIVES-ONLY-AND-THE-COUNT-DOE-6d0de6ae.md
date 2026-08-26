## A1 vocabulary is nouns and adjectives only, and the count does not show it

**This is a content defect, not a tooling note.** Spanish is at 549 of the 600
distinct headwords HL09 3.1 asks for at or below A1, and the number is honest as
far as it goes. What it does not say is that essentially none of those headwords
is a verb, and that the run to 600 cannot add one.

A CEFR A1 vocabulary that contains no verbs is not an A1 vocabulary, whatever the
count says. A1 descriptors are about doing things -- introducing yourself, asking
for something, saying what you need. A learner who has 600 nouns and adjectives
and cannot say *I need*, *I learn*, *I break*, *I wash* has not reached A1; they
have reached a picture dictionary.

### How the restriction happens

`vocabularyOf` credits a headword to a level through its lesson's curriculum
segment, which names a spine node, which declares a `stage`. So the set of things
that can be taught at A1 is exactly the set of things that can be honestly
attached to an A1 spine node.

The vocabulary tranches use three:

- `SPINE-DEFINITE-REFERENCE` -- "mark out a specific known person or thing"
- `SPINE-ASK-LOCATION` -- "ask where a familiar person, place or object is"
- `SPINE-COUNT-ONE-TO-FIVE` -- "the cardinal numbers one through five"

None of them can host a verb. Writing a chapter `canDo` that reads "I can say
*aprender*, *olvidar*, *necesitar*, *terminar* and *usar*, and mark out which
specific thing I mean" is not a capability statement; it is a slot being filled
so a lesson can be credited to a level.

Tranche 5 dropped ten verb candidates for exactly this and nothing else --
`aprender`, `olvidar`, `necesitar`, `terminar`, `usar`, `lavar`, `romper`,
`bailar`, `tocar`, `oler`. Every one had cleared the headword screen, the atom
screen and the root screen. Every one had its etymology verified against current
scholarship. They were replaced with nouns and adjectives that fit the three
nodes, and the tranche shipped 35 headwords with the level number moving exactly
as planned. **Nothing anywhere reported that the composition had been decided by
the taxonomy rather than by anyone's judgement of what a learner needs.**

### Why the existing escape hatch is not an answer

Pre-A1 nodes also count toward "at or below A1", and one of them already carries
verbs: chapter 6 teaches `hablar`, `estudiar` and `trabajar` on
`SPINE-POLITE-REQUEST-REPAIR`. That is a stretch on its face -- those verbs are
not a politeness repair -- and it was set two hundred chapters ago rather than
chosen. Using it deliberately now would mean knowingly filing the entire verb
vocabulary of a language under "make a request politely and repair a small social
mistake", which trades a visible gap for an invisible lie.

### What is actually missing

An A1 spine node for the thing a learner does at A1 with a verb: naming an
everyday action, and saying they do it. Something on the order of "I can name
common everyday actions and say that I do them." Until one exists:

- the run from 549 to 600 is structurally restricted to things and their
  properties;
- the resulting curriculum shape is one nobody chose;
- and the gate that is supposed to certify A1 readiness will certify it.

**Do not fix this inside a vocabulary tranche.** It is a spine change, it affects
every track that reaches A1 rather than Spanish alone, and it wants the owner's
decision on the node's wording before any lesson is written against it.

### The general form, for whoever meets this next

A metric defined by a join against a taxonomy is silently bounded by that
taxonomy. The count answers "how many", and the taxonomy quietly decides "of
what" -- so a shortfall reads as a content problem when it may be a shape
problem, and a target that is *met* can be met by the wrong thing entirely. Ask
what a metric **cannot** count before trusting what it does.


### Tranche 6 confirms it, and the restriction is now load-bearing

Spanish A1 vocabulary tranche 6 (chapters 374-380) took the count 549 -> 584
without authoring a single verb, because the constraint above still holds. It
did not repeat tranche 5's ten candidates; it screened a fresh twenty common A1
verbs through all three duplicate ledgers to find out how much of the shortfall
is real.

Nine cleared the string and atom screens: `lavar`, `subir`, `bajar`, `buscar`,
`guardar`, `reir`, `nacer`, `morir`, `enviar`. Six of those also clear the
ROOT ledger and are therefore fully authorable on every criterion the project
has -- `lavar`, `subir`, `buscar`, `guardar`, `morir`, `enviar`. They were not
written, and the only reason is that no A1 spine node can host them.

Two tranches have now spent their entire budget on nouns and adjectives. Spanish
is sixteen headwords from the 600 floor, which means **the gate will report A1
vocabulary attained, on a vocabulary with essentially no verbs in it, within one
more tranche.** After that the number stops moving and the defect stops being
visible in any measurement at all -- a met target reports nothing.

That makes the owner decision on the spine node time-sensitive in a way it was
not when this was first recorded.
