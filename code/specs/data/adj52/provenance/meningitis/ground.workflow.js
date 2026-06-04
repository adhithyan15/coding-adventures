export const meta = {
  name: 'meningitis-grounded-corpus',
  description: 'Derive a GROUNDED bacterial-meningitis corpus, case-blind. One agent per finding->bacterial-meningitis link crawls FORWARD to primary data (CSF-parameter meta-analyses, CSF lactate meta-analyses, Bacterial Meningitis Score, procalcitonin studies), byte-anchors sens/spec/LR, and COMPUTES the likelihood ratio. Admitted only if it terminates in real numbers.',
  phases: [{ title: 'Ground' }],
}

const TARGET = 'diagnosis(bacterial_meningitis)'
const DOMAIN = 'bacterial (vs viral/aseptic) meningitis in a patient with suspected acute meningitis undergoing lumbar puncture'
const ANCHORS = 'CSF-parameter diagnostic-accuracy reviews (Spanos JAMA 1989; Straus JAMA 2006 "Does this adult patient have acute meningitis?"); CSF lactate meta-analyses (Huy 2010 Crit Care; Sakushima 2011); the Bacterial Meningitis Score (Nigrovic 2002/2007); serum procalcitonin meta-analyses (Vikse 2015); IDSA meningitis guideline'

const FINDINGS = [
  { id: 'f0', finding: 'prior', state: 'base_rate', desc: 'pretest prevalence of BACTERIAL meningitis among patients with suspected acute meningitis / CSF pleocytosis undergoing LP', lr_type: 'prevalence' },
  { id: 'f1', finding: 'csf_gram_stain', state: 'positive', desc: 'positive CSF Gram stain', lr_type: 'LR+' },
  { id: 'f2', finding: 'csf_neutrophilic_pleocytosis', state: 'high', desc: 'high CSF neutrophil (PMN) count / pleocytosis above a bacterial threshold (e.g. >1000 cells or high % PMN)', lr_type: 'LR+' },
  { id: 'f3', finding: 'csf_glucose', state: 'low', desc: 'low CSF glucose or low CSF:serum glucose ratio (e.g. < 0.4)', lr_type: 'LR+' },
  { id: 'f4', finding: 'csf_protein', state: 'elevated', desc: 'elevated CSF protein (e.g. > 2 g/L)', lr_type: 'LR+' },
  { id: 'f5', finding: 'csf_lactate', state: 'elevated', desc: 'elevated CSF lactate (e.g. >= 3.5 mmol/L)', lr_type: 'LR+' },
  { id: 'f6', finding: 'serum_procalcitonin', state: 'elevated', desc: 'elevated serum procalcitonin', lr_type: 'LR+' },
  { id: 'f7', finding: 'seizure', state: 'present', desc: 'seizure at or before presentation (a Bacterial Meningitis Score criterion)', lr_type: 'LR' },
  { id: 'f8', finding: 'csf_culture', state: 'positive', desc: 'positive CSF culture (the reference standard)', lr_type: 'LR+' },
]

const SCHEMA = {
  type: 'object',
  required: ['id', 'target', 'chain', 'verdict', 'primary_data', 'note'],
  properties: {
    id: { type: 'string' }, target: { type: 'string' },
    primary_data: { type: 'object', properties: {
      metric_type: { type: 'string' }, values: { type: 'string' }, n: { type: 'string' },
      population: { type: 'string' }, study: { type: 'string' }, byte_quote: { type: 'string' } } },
    chain: { type: 'array', items: { type: 'object', required: ['hop', 'source_queried', 'found', 'supporting_quote'], properties: {
      hop: { type: 'number' }, source_queried: { type: 'string' }, resolved_url: { type: 'string' },
      found: { type: 'boolean' }, supporting_quote: { type: 'string' }, gives_numbers: { type: 'boolean' }, next_citation: { type: 'string' } } } },
    computed_lr: { type: 'number' }, lr_formula: { type: 'string' },
    verdict: { type: 'string', enum: ['grounded', 'direction_only', 'fabricated'] }, note: { type: 'string' },
  },
}

const prompt = (f) => `You are a forward byte-provenance crawler BUILDING a grounded clinical corpus, case-blind, for ${DOMAIN}. Find the PRIMARY DATA for one diagnostic finding and COMPUTE its likelihood ratio for bacterial (vs viral) meningitis. Do NOT invent a number.

FINDING (${f.id}): ${f.finding}(${f.state}) — ${f.desc}
  bearing on: ${TARGET}
  expected metric: ${f.lr_type}

PROCEDURE (WebSearch/WebFetch; recurse through citations to PRIMARY data):
1. Find the landmark study / meta-analysis reporting test characteristics for THIS finding distinguishing bacterial from viral meningitis. Likely anchors: ${ANCHORS}.
2. Capture the LITERAL sentence stating the numbers (byte_quote). Follow citations to the source.
3. COMPUTE the LR: LR+ = sens/(1-spec); LR- = (1-sens)/spec; use a published pooled LR directly if given (CSF parameter reviews often report LRs directly); for the prior, report prevalence as a probability. Show arithmetic in lr_formula.
4. verdict=grounded ONLY if computed_lr is from byte-anchored real numbers; direction_only if direction-only (computed_lr=0); fabricated if unsupported.

Return id, target=${TARGET}, primary_data, chain, computed_lr, lr_formula, verdict, note.`

const results = await pipeline(FINDINGS,
  (f) => agent(prompt(f), { phase: 'Ground', label: `ground:${f.id}:${f.finding}`, agentType: 'general-purpose', schema: SCHEMA }).catch(() => null))
const done = results.filter(Boolean)
const tally = {}; for (const r of done) tally[r.verdict] = (tally[r.verdict] || 0) + 1
log(`meningitis corpus grounding: ${done.length}/${FINDINGS.length}; verdicts ${JSON.stringify(tally)}`)
return { domain: 'bacterial_meningitis', target: TARGET, tally, per_finding: done }
