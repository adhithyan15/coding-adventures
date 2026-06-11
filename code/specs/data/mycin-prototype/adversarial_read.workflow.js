export const meta = {
  name: 'mycin-adversarial-read',
  description: 'Adversarial reading of a case IR at BOTH link types (FORWARD-DESIGN §1). N model-diverse readers (Opus + Sonnet + Haiku) per case: (1) INFERENCE READ — for each inferred finding, does the basis span ENTAIL it or is it a LEAP (over-read)? (2) DISCARD READ — for each discarded span, try to show it is actually a load-bearing dictionary finding that was WRONGLY dropped (the dangerous direction in medicine). Majority vote (>=2 of 3). The over-reads are dropped; the wrongly-dropped findings are surfaced for recovery. Sequential batches of 10 cases.',
  phases: [{ title: 'AdversarialRead', detail: '3 readers per case: inference read + discard read' }],
}

const DIR = '/Users/adhithya/Downloads/coding-adventures/code/specs/data/mycin-prototype'
const ids = Array.isArray(args) ? args : (typeof args === 'string' ? JSON.parse(args) : args)
const BATCH = 10
const READERS = [undefined, 'sonnet', 'haiku']

const SCHEMA = {
  type: 'object', additionalProperties: false,
  required: ['inference_leaps', 'discard_load_bearing'],
  properties: {
    inference_leaps: {
      type: 'array', description: 'terms of inferred findings whose basis span does NOT entail them',
      items: { type: 'string' },
    },
    discard_load_bearing: {
      type: 'array',
      description: 'discarded spans that are actually a load-bearing dictionary finding wrongly dropped',
      items: {
        type: 'object', additionalProperties: false, required: ['span', 'recovered_term'],
        properties: { span: { type: 'string' }, recovered_term: { type: 'string' } },
      },
    },
  },
}

function reader(id, model) {
  const irPath = `${DIR}/ir/${id}.json`
  const prompt =
    `You are an ADVERSARIAL reader auditing a clinical case decomposition. Read TWO files with the ` +
    `Read tool:\n1. the case IR: ${irPath} — { findings (each term/span/type/polarity), discard ` +
    `(spans not mapped, each with a reason), inference_justifications }.\n2. the dictionary: ` +
    `${DIR}/dictionary.json — the CLOSED set of canonical finding functors + value domains + ` +
    `surface_forms.\n\nDo TWO adversarial reads:\n` +
    `(A) INFERENCE READ — for every finding with type "inferred", decide whether its span genuinely ` +
    `ENTAILS the claimed term, or whether it is a LEAP (an over-read the bytes do not force). List the ` +
    `terms you judge LEAP.\n` +
    `(B) DISCARD READ — for every span in "discard", try to REFUTE the discard: does that span ` +
    `actually establish a dictionary finding (a functor+value, mappable via surface_forms) that was ` +
    `WRONGLY dropped? If yes, report the span and the recovered_term. Be strict: only flag a span if it ` +
    `truly maps to a dictionary finding value (not a symptom/demographic with no functor).\n\n` +
    `Return inference_leaps (list of terms) and discard_load_bearing (list of {span, recovered_term}).`
  return agent(prompt, { label: `advread:${id}:${model || 'opus'}`, phase: 'AdversarialRead', schema: SCHEMA, ...(model ? { model } : {}) })
}

function caseRead(id) {
  return parallel(READERS.map((m) => () => reader(id, m))).then((vs) => {
    const reads = vs.filter(Boolean)
    // majority (>=2 of the readers that returned)
    const need = Math.max(2, Math.ceil(reads.length / 2))
    const leapCount = {}
    const dbCount = {}
    const dbTerm = {}
    for (const r of reads) {
      for (const t of r.inference_leaps || []) leapCount[t] = (leapCount[t] || 0) + 1
      for (const d of r.discard_load_bearing || []) {
        dbCount[d.span] = (dbCount[d.span] || 0) + 1
        dbTerm[d.span] = d.recovered_term
      }
    }
    const inference_leaps = Object.keys(leapCount).filter((t) => leapCount[t] >= need)
    const discard_load_bearing = Object.keys(dbCount)
      .filter((s) => dbCount[s] >= need)
      .map((s) => ({ span: s, recovered_term: dbTerm[s] }))
    return { case_id: id, n_readers: reads.length, inference_leaps, discard_load_bearing }
  })
}

phase('AdversarialRead')
const all = []
for (let i = 0; i < ids.length; i += BATCH) {
  const batch = ids.slice(i, i + BATCH)
  log(`Batch ${i / BATCH + 1}: adversarial-reading ${batch.join(', ')} (${all.length} done).`)
  const res = await parallel(batch.map((id) => () => caseRead(id)))
  all.push(...res.filter(Boolean))
}
log(`Collected ${all.length}/${ids.length} adversarial reads.`)
return all
