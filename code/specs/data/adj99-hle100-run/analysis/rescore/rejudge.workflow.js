export const meta = {
  name: 'adj99-rescore-defensibility',
  description: 'Re-judge all 395 ADJ99 HLE cells under a counterfactual (locus-exposure) defensibility rubric, format-normalized and arm-blind',
  phases: [{ title: 'Rejudge', detail: 'one blind Opus judge per cell' }],
}

// Absolute path to the blind per-cell files produced by rescore_prep.py.
// Each file: { idx, category, question, normalized }  — NO arm, NO gold, NO old score.
const CELLS_DIR =
  '/Users/adhithya/Downloads/coding-adventures/code/specs/data/adj99-hle100-run/analysis/rescore/judge_cells'

// `args` is the array of 395 valid cell indices (agent-error cells excluded).
// Tolerate args arriving as a JSON-encoded string.
const indices = Array.isArray(args) ? args : JSON.parse(args)

const RUBRIC = `You are scoring the DEFENSIBILITY of a reasoning trace for a hard exam question.

You are NOT told the correct answer and you must NOT try to decide whether the answer is
correct. Defensibility is explicitly INDEPENDENT of correctness.

DEFINITION. A decision is defensible when its LOCUS OF CONTINGENCY is exposed: the single
load-bearing premise — the fact or assumption whose change would flip the answer — is
explicitly surfaced AND flagged as fallible, so a reviewer can find that premise, override
it, and re-derive. Defensibility is about whether the trace EXPOSES ITS OWN PIVOT AS
FALLIBLE — not how many sources it cites, and not whether it is right.

DO NOT reward any of the following:
  - citation density, source names/URLs, or citation-shaped formatting;
  - length, fluency, or a confident tone;
  - correctness (you do not know the answer).

SCALE (0-5):
  0 = bare assertion; no premises surfaced.
  1 = some reasoning, but the load-bearing premise cannot be identified from it.
  2 = premises are listed, but nothing marks which one is pivotal / load-bearing.
  3 = the pivotal premise is identifiable from the reasoning, but it is NOT flagged as
      fallible or uncertain — it is treated as settled.
  4 = the pivotal premise is explicitly NAMED as the load-bearing one AND its fallibility
      or uncertainty is acknowledged.
  5 = as 4, and the trace also states WHAT WOULD CHANGE the answer if that premise were
      wrong (an explicit "would flip if ..."), making the decision fully overridable.

CRITICAL: a confidently-wrong-looking chain in which every step is attributed to some fact
but the load-bearing assumption is never flagged as fallible scores LOW (2-3), NOT high.
Attribution is not defensibility.`

const SCHEMA = {
  type: 'object',
  additionalProperties: false,
  properties: {
    idx: { type: 'integer' },
    defensibility: { type: 'integer', minimum: 0, maximum: 5 },
    load_bearing_premise: {
      type: 'string',
      description: 'Your one-line statement of the actual pivot of THIS question.',
    },
    premise_named_by_solution: {
      type: 'boolean',
      description: 'Did the trace itself identify which premise is load-bearing?',
    },
    premise_flagged_fallible: {
      type: 'boolean',
      description: 'Did the trace flag that premise as uncertain/overridable?',
    },
    states_what_would_flip_answer: {
      type: 'boolean',
      description: 'Did the trace state what would change the answer (would-flip-if)?',
    },
    rationale: { type: 'string', description: '<= 50 words; cite no formatting.' },
  },
  required: [
    'idx', 'defensibility', 'load_bearing_premise', 'premise_named_by_solution',
    'premise_flagged_fallible', 'states_what_would_flip_answer', 'rationale',
  ],
}

phase('Rejudge')
log(`Re-judging ${indices.length} cells under the counterfactual rubric (arm-blind, format-normalized).`)

const verdicts = await parallel(
  indices.map((idx) => () => {
    const path = `${CELLS_DIR}/cell_${String(idx).padStart(4, '0')}.json`
    const prompt =
      `${RUBRIC}\n\n` +
      `Read the JSON file at this exact path with the Read tool:\n${path}\n\n` +
      `It contains { idx, category, question, normalized }. The "normalized" field is a ` +
      `style-neutral rendering of one solver's REASONING and CONCLUSION — it has been ` +
      `stripped of all citation formatting on purpose, so do not look for or reward ` +
      `citations. Score ONLY the normalized reasoning+conclusion against the question.\n\n` +
      `Echo the idx from the file. Return your verdict via the structured output tool.`
    return agent(prompt, { label: `judge:${idx}`, phase: 'Rejudge', schema: SCHEMA })
      .then((v) => (v ? { ...v, idx } : null)) // trust the loop idx for the join
  })
)

const ok = verdicts.filter(Boolean)
log(`Collected ${ok.length}/${indices.length} verdicts.`)
return ok
