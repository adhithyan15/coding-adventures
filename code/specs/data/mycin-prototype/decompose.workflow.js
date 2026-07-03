export const meta = {
  name: 'mycin-decompose',
  description: 'WARM-PATH DECOMPOSE (the one model touchpoint per case). The LLM reads a messy clinical vignette and the standard dictionary, and DECOMPOSES the prose into typed IR: findings as CANONICAL dictionary terms (each with a verbatim byte span, type stated|inferred, polarity), a DISCARD list (spans it did not map to a finding, each with a reason), and inference justifications for any inferred finding. It does NOT diagnose — the diagnosis is the CPU engine\'s. Constrained to the dictionary vocabulary so the IR and the rulebook share one vocabulary. Sequential batches of 10.',
  phases: [{ title: 'Decompose', detail: 'vignette -> typed IR, dictionary-constrained, no diagnosis' }],
}

const DIR = '/Users/adhithya/Downloads/coding-adventures/code/specs/data/mycin-prototype'
const ids = Array.isArray(args) ? args : (typeof args === 'string' ? JSON.parse(args) : args)
const BATCH = 10

const FINDING = {
  type: 'object', additionalProperties: false,
  required: ['term', 'span', 'type'],
  properties: {
    term: { type: 'string', description: 'a canonical dictionary term, e.g. csf_glucose(low)' },
    span: { type: 'string', description: 'verbatim substring of the vignette that establishes it' },
    type: { type: 'string', enum: ['stated', 'inferred'] },
    polarity: { type: 'string', enum: ['affirmed', 'denied'] },
  },
}
const SCHEMA = {
  type: 'object', additionalProperties: false,
  required: ['case_id', 'findings', 'discard'],
  properties: {
    case_id: { type: 'string' },
    findings: { type: 'array', items: FINDING },
    discard: {
      type: 'array',
      items: {
        type: 'object', additionalProperties: false, required: ['span', 'reason'],
        properties: { span: { type: 'string' }, reason: { type: 'string', description: '<= 25 words' } },
      },
    },
    inference_justifications: {
      type: 'array',
      items: {
        type: 'object', additionalProperties: false, required: ['term', 'basis_span', 'verdict'],
        properties: {
          term: { type: 'string' }, basis_span: { type: 'string' },
          verdict: { type: 'string', enum: ['ENTAILED', 'LEAP'] },
        },
      },
    },
  },
}

function decompose(id) {
  const prompt =
    `You are the DECOMPOSE stage of a clinical adjudication framework. You do NOT diagnose — a ` +
    `deterministic engine decides the differential. Read TWO files with the Read tool:\n` +
    `1. the standard dictionary: ${DIR}/dictionary.json — the CLOSED set of canonical finding terms ` +
    `(each "functor" + "value_domain" + "surface_forms") and hypotheses.\n` +
    `2. the cases: ${DIR}/cases/cases.json — find the case with "id" == "${id}" and read its "vignette".\n\n` +
    `Decompose the vignette's PROSE into typed IR:\n` +
    `- findings: for every clinical fact that maps to a dictionary term, emit { term: "functor(value)" ` +
    `(MUST be a dictionary functor with a value in its value_domain — use the surface_forms to map), ` +
    `span: the verbatim vignette substring that establishes it, type: "stated" if the span directly ` +
    `says it / "inferred" if you had to interpret, polarity: "affirmed" or "denied" }. Use value ` +
    `"normal" / "negative" / "absent" for explicitly-normal or absent findings. Do NOT invent a ` +
    `finding the vignette does not support, and do NOT emit a term outside the dictionary.\n` +
    `- discard: every clinical or contextual span you did NOT turn into a finding (demographics, ` +
    `unrelated history, medications, exposures), each with a short reason it is not a dispositive finding.\n` +
    `- inference_justifications: for each "inferred" finding, give the basis_span and whether the bytes ` +
    `ENTAIL it or it is a LEAP.\n\nEcho the case_id. Return the structured IR.`
  return agent(prompt, { label: `decompose:${id}`, phase: 'Decompose', schema: SCHEMA })
    .then((e) => (e ? { ...e, case_id: id } : null))
}

phase('Decompose')
const all = []
for (let i = 0; i < ids.length; i += BATCH) {
  const batch = ids.slice(i, i + BATCH)
  log(`Batch ${i / BATCH + 1}: decomposing ${batch.join(', ')} (${all.length} done).`)
  const res = await parallel(batch.map((id) => () => decompose(id)))
  all.push(...res.filter(Boolean))
}
log(`Collected ${all.length}/${ids.length} decompositions.`)
return all
