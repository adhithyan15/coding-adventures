export const meta = {
  name: 'adj52-haiku-descent',
  description: 'Descent rung: HAIKU as the answering model on 100 self-found, perturbed clinical cases, two arms — (1) Haiku blind (plain answer), (2) Haiku + framework discipline but NO engine/program: Haiku decomposes an IR, derives a cited rulebook from it, then reasons to a conclusion CITING the input facts and rulebook rules. A strong blind judge scores both arms vs ground truth. Prepare + Judge stay strong; only the two answering arms are Haiku.',
  phases: [
    { title: 'Prepare' },
    { title: 'HaikuBlind', model: 'haiku' },
    { title: 'HaikuIR', model: 'haiku' },
    { title: 'HaikuDerive', model: 'haiku' },
    { title: 'HaikuConclude', model: 'haiku' },
    { title: 'Judge' },
  ],
}

// Same 100-seed design as the run-3 pipeline so this is a comparable rung.
const SPECIALTIES = [
  'cardiology', 'pulmonology', 'nephrology', 'endocrinology', 'infectious disease',
  'hematology', 'medical oncology', 'rheumatology', 'neurology', 'gastroenterology',
  'dermatology', 'hepatology', 'clinical immunology', 'emergency medicine',
  'geriatric medicine', 'vascular medicine', 'otolaryngology', 'urology',
  'allergy and immunology', 'general internal medicine',
]
const ANGLES = [
  'where the initial working diagnosis turned out to be wrong',
  'where the presentation mimicked a far more common condition',
  'with an unexpected final diagnosis after an initially misleading workup',
  'initially misattributed to a benign or unrelated cause',
  'where a rare disease masqueraded as a common one',
]
const SEEDS = []
for (const s of SPECIALTIES) for (const a of ANGLES) SEEDS.push(`an adult clinical case report in ${s} ${a}`)
// Full 100 (20 specialties x 5 angles). Smoke-validated on 3 (0 skips, full data persisted).
const seeds = (args && args.seeds && args.seeds.length) ? args.seeds : SEEDS
if (!seeds.length) { log('no seeds'); return { error: 'no seeds' } }
log(`haiku-descent over ${seeds.length} seed(s)`)

// ---- Schemas ----
const PREPARE_SCHEMA = {
  type: 'object',
  required: ['skipped', 'ground_truth', 'prose', 'perturbations', 'diagnosis_unchanged'],
  properties: {
    skipped: { type: 'boolean' },
    source_url: { type: 'string' },
    ground_truth: { type: 'string', description: 'final confirmed dx + how confirmed + correct disposition; NOT shown to the answering arms' },
    prose: { type: 'string', description: 'PERTURBED, sanitised case prose' },
    perturbations: { type: 'array', items: { type: 'string' } },
    diagnosis_unchanged: { type: 'boolean' },
  },
}
const BLIND_SCHEMA = {
  type: 'object',
  required: ['answer_text'],
  properties: { answer_text: { type: 'string', description: 'most likely diagnosis + recommended next action + confidence + brief reasoning' } },
}
const IR_SCHEMA = {
  type: 'object',
  required: ['facts', 'queries'],
  properties: {
    inferred_domain: { type: 'string' },
    facts: { type: 'array', items: { type: 'object' }, description: '[{id, term, source_span}] — every clause of the prose typed or discarded-with-reason' },
    uncertainties: { type: 'array', items: { type: 'object' } },
    queries: { type: 'array', items: { type: 'object' } },
    discarded: { type: 'array', items: { type: 'object' } },
  },
}
const RULEBOOK_SCHEMA = {
  type: 'object',
  required: ['rules'],
  properties: {
    rules: {
      type: 'array',
      description: 'cited rules derived from the IR: [{id, rule, bears_on, direction, source}]',
      items: { type: 'object' },
    },
  },
}
const CONCLUDE_SCHEMA = {
  type: 'object',
  required: ['top_diagnosis', 'recommended_next_step', 'confidence', 'conclusion_text', 'citations'],
  properties: {
    top_diagnosis: { type: 'string' },
    recommended_next_step: { type: 'string' },
    confidence: { type: 'string' },
    conclusion_text: { type: 'string', description: 'the judge-facing answer: diagnosis + next step + reasoning, every key claim citing input fact id(s) and rule id(s)' },
    citations: { type: 'array', items: { type: 'object' }, description: '[{claim, input_facts:[ids], rules:[ids]}]' },
  },
}
const VERDICT_SCHEMA = {
  type: 'object',
  required: ['winner', 'rationale', 'a_correct', 'b_correct'],
  properties: {
    winner: { type: 'string', enum: ['A', 'B', 'tie'] },
    a_correct: { type: 'string', enum: ['correct', 'partial', 'incorrect'] },
    b_correct: { type: 'string', enum: ['correct', 'partial', 'incorrect'] },
    rationale: { type: 'string' },
  },
}

// ---- Prompts ----
const preparePrompt = (seed) => `Find and prepare a published clinical case for a blinded diagnostic-reasoning experiment. SEED: ${seed}.
1. Use WebSearch/WebFetch to FIND ONE real published case report matching the seed (prefer open-access PMC) with a CLEARLY DOCUMENTED final/confirmed diagnosis (biopsy, culture, genetics, imaging, autopsy, or follow-up). Favor initially-misleading presentations. AVOID the most famous textbook cases. If none found, return skipped=true with empty fields.
2. Extract GROUND TRUTH (final dx + how confirmed + correct disposition) and source_url. Held aside; not shown to the answering arms.
3. Produce a PERTURBED, SANITISED vignette: remove any sentence naming the dx / confirmatory result / expert discussion; change EVERY diagnosis-irrelevant surface detail (age within dx-preserving range, sex/ethnicity if not relevant, all lab numbers shifted but kept same side of reference + magnitude, anecdotes, timeline, drug doses, institution, ordering, phrasing); PRESERVE in substance every load-bearing finding so the diagnosis is UNCHANGED. It must read as a fresh vignette.
Return skipped, source_url, ground_truth, prose, perturbations, diagnosis_unchanged.`

const blindPrompt = (prose) => `Read this clinical case and answer as you normally would. Do NOT look up the specific published case; do not read local files. Give the most likely diagnosis, the recommended next action, your confidence, and brief reasoning.

=== CASE ===
${prose}
=== END ===`

const irPrompt = (prose) => `Decompose this clinical case into a human-readable IR. Read EVERY byte: each span is a typed fact OR explicitly discarded with a reason (no silent omission). Ambiguity becomes an uncertainty with candidate readings, never a guess. Infer the domain yourself. Raise the queries the case actually asks. Do NOT solve it here; do NOT look up the outcome.
Return inferred_domain, facts [{id, term, source_span}], uncertainties [{id, about, domain}], queries [{id, asks}], discarded [{source_span, reason}].

=== CASE ===
${prose}
=== END ===`

const derivePrompt = (irJson) => `You are given an ingested IR (facts + queries) for a clinical case. Derive the rulebook needed to answer the queries — the set of clinical rules linking findings to candidate diagnoses. Use WebSearch/WebFetch for REAL citations. Recurse into subtypes where evidence differs. Do NOT write any program or compute probabilities — produce a human-readable cited rulebook only.
Return rules: [{id, rule (plain-language: which finding raises/lowers which candidate), bears_on (the candidate diagnosis), direction (raises|lowers), source (real citation)}].

=== IR ===
${irJson}`

const concludePrompt = (prose, irJson, rbJson) => `You have a clinical case, its ingested IR (facts + queries), and a derived cited rulebook. WITHOUT writing any program or running any engine, reason to a conclusion.
Rules: (a) use ONLY facts present in the IR and rules present in the rulebook — introduce no new facts or rules; (b) for every key claim in your conclusion, CITE the supporting IR fact id(s) and rulebook rule id(s); (c) give an honest confidence; (d) name the recommended next step.
Return top_diagnosis, recommended_next_step, confidence, conclusion_text (the full answer with inline citations to fact/rule ids), citations [{claim, input_facts:[ids], rules:[ids]}].

=== CASE ===
${prose}
=== IR ===
${irJson}
=== RULEBOOK ===
${rbJson}`

const judgePrompt = (gt, a, b) => `You are an impartial judge. Score two responses (OUTPUT A, OUTPUT B) from systems whose identities are hidden, against the ground truth. Judge only content; do not guess which is which. Assess correctness vs ground truth, hallucination, calibration, and defensibility (can the reasoning be traced/verified). Pick a winner.

=== GROUND TRUTH ===
${gt}

=== OUTPUT A ===
${a}

=== OUTPUT B ===
${b}`

// ---- Pipeline ----
const results = await pipeline(
  seeds,
  (seed, _o, idx) => agent(preparePrompt(seed), { phase: 'Prepare', label: `prepare:case-${idx + 1}`, agentType: 'general-purpose', schema: PREPARE_SCHEMA })
    .then((p) => {
      if (p.skipped || !p.prose) { throw new Error(`no case for seed ${idx + 1}`) }
      return { id: `case-${idx + 1}`, source_url: p.source_url, ground_truth: p.ground_truth, prose: p.prose, perturbations: p.perturbations, diagnosis_unchanged: p.diagnosis_unchanged }
    }),
  // Arm 1: Haiku blind
  (o) => agent(blindPrompt(o.prose), { phase: 'HaikuBlind', label: `blind:${o.id}`, model: 'haiku', agentType: 'general-purpose', schema: BLIND_SCHEMA })
    .then((b) => ({ ...o, blind_answer: b.answer_text })),
  // Arm 2 step 1: Haiku decomposes IR
  (o) => agent(irPrompt(o.prose), { phase: 'HaikuIR', label: `ir:${o.id}`, model: 'haiku', agentType: 'general-purpose', schema: IR_SCHEMA })
    .then((ir) => ({ ...o, ir })),
  // Arm 2 step 2: Haiku derives the cited rulebook from the IR
  (o) => agent(derivePrompt(JSON.stringify(o.ir)), { phase: 'HaikuDerive', label: `derive:${o.id}`, model: 'haiku', agentType: 'general-purpose', schema: RULEBOOK_SCHEMA })
    .then((rb) => ({ ...o, rulebook: rb.rules })),
  // Arm 2 step 3: Haiku concludes over IR + rulebook, citing input + rules (NO engine)
  (o) => agent(concludePrompt(o.prose, JSON.stringify(o.ir), JSON.stringify(o.rulebook)), { phase: 'HaikuConclude', label: `conclude:${o.id}`, model: 'haiku', agentType: 'general-purpose', schema: CONCLUDE_SCHEMA })
    .then((c) => ({ ...o, framework: c })),
  // Judge (strong): blind A/B
  (o, _o2, idx) => {
    const fwIsA = (idx % 2) === 0
    const A = fwIsA ? o.framework.conclusion_text : o.blind_answer
    const B = fwIsA ? o.blind_answer : o.framework.conclusion_text
    return agent(judgePrompt(o.ground_truth, A, B), { phase: 'Judge', label: `judge:${o.id}`, agentType: 'general-purpose', schema: VERDICT_SCHEMA })
      .then((v) => ({
        id: o.id,
        source_url: o.source_url,
        diagnosis_unchanged: o.diagnosis_unchanged,
        perturbations: o.perturbations,
        prose: o.prose,
        ground_truth: o.ground_truth,
        ir: o.ir,
        rulebook: o.rulebook,
        blind_answer: o.blind_answer,
        framework_conclusion: o.framework.conclusion_text,
        framework_top: o.framework.top_diagnosis,
        framework_next_step: o.framework.recommended_next_step,
        framework_confidence: o.framework.confidence,
        framework_citations: o.framework.citations,
        framework_is: fwIsA ? 'A' : 'B',
        winner: v.winner,
        framework_correct: fwIsA ? v.a_correct : v.b_correct,
        blind_correct: fwIsA ? v.b_correct : v.a_correct,
        framework_won: v.winner === (fwIsA ? 'A' : 'B'),
        blind_won: v.winner === (fwIsA ? 'B' : 'A'),
        rationale: v.rationale,
      }))
  }
)

// ---- Aggregate ----
const done = results.filter(Boolean)
const c = (v) => v === 'correct'
const tally = {
  seeds_attempted: seeds.length,
  cases_completed: done.length,
  skipped_or_failed: seeds.length - done.length,
  framework_haiku_won: done.filter((r) => r.framework_won).length,
  blind_haiku_won: done.filter((r) => r.blind_won).length,
  tie: done.filter((r) => r.winner === 'tie').length,
  framework_haiku_correct: done.filter((r) => c(r.framework_correct)).length,
  blind_haiku_correct: done.filter((r) => c(r.blind_correct)).length,
  both_correct: done.filter((r) => c(r.framework_correct) && c(r.blind_correct)).length,
  only_framework_correct: done.filter((r) => c(r.framework_correct) && !c(r.blind_correct)).length,
  only_blind_correct: done.filter((r) => !c(r.framework_correct) && c(r.blind_correct)).length,
  neither_correct: done.filter((r) => !c(r.framework_correct) && !c(r.blind_correct)).length,
}
log(`HAIKU DESCENT: ${tally.cases_completed}/${tally.seeds_attempted} completed; framework-haiku correct ${tally.framework_haiku_correct} vs blind-haiku ${tally.blind_haiku_correct}; wins framework ${tally.framework_haiku_won} / blind ${tally.blind_haiku_won} / tie ${tally.tie}; only-framework ${tally.only_framework_correct}, only-blind ${tally.only_blind_correct}`)
return { tally, per_case: done }
