export const meta = {
  name: 'adj67-atomic-feed',
  description: 'ADJ67 — feed a WEAK model (Haiku) one decomposed fact at a time, not the whole case. The framework already decomposed the scenario; we never hand the weak model the raw paragraph (with its priming/framing). For each atomic fact, Haiku makes ONE tiny local judgment — does this single observation argue for/against each candidate cause — seeing nothing else. The HARNESS accumulates those local verdicts into a weight-of-evidence matrix and the deterministic engine decides. The weak model contributes local ratings; the framework holds the global structure and makes the call. Tests whether atomic feeding keeps a weak model off the holistic "shark, because sharks eat seals" trap.',
  phases: [{ title: 'RateAtoms' }],
}

// the framework's decomposition of the seal case (neutral phrasing — NO framing/priming).
const HYPOTHESES = ['white shark bite (predation)', 'boat propeller strike', 'fishing-gear entanglement']
const FACTS = [
  'Three separate cuts that are roughly parallel to one another, of similar length, spaced at fairly regular intervals.',
  'The cut edges are clean and relatively smooth, each following a shallow even curve.',
  'Little tissue loss between the cuts; no flaps of skin are torn back from the wound margins.',
  'The wounds are open but not deep — they expose blubber but do not reach the body cavity.',
  'No bite injuries to the hind flippers or other limbs; the injuries are confined to the flank and back.',
  'The surrounding waters are a place where white sharks are known to feed on seals at this time of year.',
  'Recreational and fishing vessels are active close to shore in these waters.',
]

const RATE_SCHEMA = {
  type: 'object', required: ['ratings'],
  properties: {
    ratings: {
      type: 'array',
      items: {
        type: 'object', required: ['hypothesis', 'rating'],
        properties: {
          hypothesis: { type: 'string' },
          rating: { type: 'integer', description: '-2 strongly argues against, -1 argues against, 0 neutral, +1 argues for, +2 strongly argues for' },
        },
      },
    },
  },
}

const ratePrompt = (fact) => `You are rating ONE piece of forensic evidence about an injured animal. You are seeing ONLY this one observation — not the rest of the case.

OBSERVATION: "${fact}"

CANDIDATE CAUSES:
${HYPOTHESES.map((h) => `  - ${h}`).join('\n')}

For THIS single observation only, rate how it bears on each cause, using your knowledge of wound morphology:
  -2 = strongly argues against,  -1 = argues against,  0 = neutral / uninformative,  +1 = argues for,  +2 = strongly argues for
Give a rating for EACH cause. Judge only from this one observation; do not assume anything else about the case.`

// rating (-2..+2) -> decibans (harness-fixed, deterministic mapping)
const TO_DB = 5

// ---- run: one tiny Haiku call per decomposed fact, in parallel; harness aggregates ----
const rated = await parallel(FACTS.map((f, i) => () =>
  agent(ratePrompt(f), { phase: 'RateAtoms', label: `atom-${i + 1}`, agentType: 'general-purpose', model: 'haiku', schema: RATE_SCHEMA })
    .then((r) => ({ fact: f, ratings: r.ratings }))))

// build the weight-of-evidence matrix from the weak model's local verdicts.
const evidence = rated.filter(Boolean).map((r, i) => {
  const weights = {}
  for (const h of HYPOTHESES) {
    const hit = (r.ratings || []).find((x) => x.hypothesis === h || x.hypothesis.includes(h.split(' ')[0]))
    weights[h] = (hit ? hit.rating : 0) * TO_DB
  }
  return { name: `atom_${i + 1}`, fact: r.fact, source: 'weak-model-local-verdict', weights }
})

return { model: 'haiku-4.5 (atomic feed)', hypotheses: HYPOTHESES, evidence }
