## Audit: which spine nodes hold content their `canDo` does not describe?

Twice now, the same defect has been found by accident rather than by looking:

- **#13154** found `SPINE-SAY-WHAT-I-WANT` staged at **A2** because it declared an A2 prerequisite
  nobody had checked. Its `canDo` — *"I can say what I want or need, and ask for it"* — was an A1
  capability all along, and nothing was wrong with the sentence. The node was in the wrong place.
- **`HL23` §13** found `SPINE-COUNT-ONE-TO-FIVE` — *"I can understand and produce the cardinal
  numbers one through five"* — carrying **twenty-two quality adjectives** (`alto`, `gordo`,
  `alegre`, `feo`, `necesario`, `dulce`, …). Here the stage was right and the **contents** were
  wrong.

Two shapes of the same question — *does this node's `canDo` describe what it actually holds?* — and
both were found while doing something else. §12.2 had even written down a refusal of the second
one, phrased forward-looking, while twenty-two instances already sat in the corpus.

**The audit is cheap and mechanical.** For every node, list its realized lessons with their
headwords and concept tags beside the node's `canDo`, and read down the column. The signal is loud:
a numbers node whose tags are `ES-COUNT-UGLY` and `ES-COUNT-CHEERFUL` does not need subtle
judgement to spot. A first pass could flag any node where a majority of concept tags share a prefix
the `canDo` never mentions.

Do it once, across all 23 tracks and all 39 nodes, and record the result — including the nodes that
come back clean, so the next person knows the question has been asked. Both instances above cost
more to find by accident than the whole audit would cost deliberately.
