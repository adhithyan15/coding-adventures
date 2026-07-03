export const meta = {
  name: 'strep-grounded-corpus',
  description: 'Derive a GROUNDED streptococcal-pharyngitis corpus, case-blind. One agent per finding->GAS link crawls FORWARD to primary data (Centor/McIsaac derivation + validation, rapid-antigen-test meta-analyses, GAS prevalence studies), byte-anchors the sensitivity/specificity/OR/LR, and COMPUTES the likelihood ratio. Admitted only if it terminates in real numbers.',
  phases: [{ title: 'Ground' }],
}

const TARGET = 'diagnosis(group_a_strep_pharyngitis)'
const DOMAIN = 'streptococcal pharyngitis (group A strep, GAS) in a patient with acute sore throat'
const ANCHORS = 'Centor 1981 + McIsaac 1998/2004 derivation/validation (clinical-score LRs); rapid antigen detection test (RADT) meta-analyses (e.g. Cohen 2016 Cochrane, Stewart 2014) for sens/spec; GAS prevalence-in-sore-throat studies (Shaikh 2010 for children; adult pharyngitis cohorts); IDSA pharyngitis guideline'

const FINDINGS = [
  { id: 'f0', finding: 'prior', state: 'base_rate', desc: 'pretest prevalence of group A strep among patients (specify adult vs child) presenting with acute sore throat / pharyngitis', lr_type: 'prevalence' },
  { id: 'f1', finding: 'tonsillar_exudate', state: 'present', desc: 'tonsillar exudates (a Centor criterion)', lr_type: 'LR' },
  { id: 'f2', finding: 'tender_anterior_cervical_nodes', state: 'present', desc: 'tender/swollen anterior cervical lymph nodes (a Centor criterion)', lr_type: 'LR' },
  { id: 'f3', finding: 'history_of_fever', state: 'present', desc: 'history of fever / temperature > 38C (a Centor criterion)', lr_type: 'LR' },
  { id: 'f4', finding: 'cough', state: 'absent', desc: 'ABSENCE of cough (a Centor criterion — its absence raises GAS)', lr_type: 'LR' },
  { id: 'f5', finding: 'age', state: 'under_15', desc: 'age 3-14 (the McIsaac age modifier that raises GAS likelihood)', lr_type: 'LR' },
  { id: 'f6', finding: 'rapid_antigen_test', state: 'positive', desc: 'positive rapid antigen detection test (RADT) for GAS', lr_type: 'LR+' },
  { id: 'f7', finding: 'rapid_antigen_test', state: 'negative', desc: 'negative rapid antigen detection test (RADT) for GAS', lr_type: 'LR-' },
  { id: 'f8', finding: 'throat_culture', state: 'positive', desc: 'positive throat culture for GAS (the reference standard)', lr_type: 'LR+' },
]

const SCHEMA = {
  type: 'object',
  required: ['id', 'target', 'chain', 'verdict', 'primary_data', 'note'],
  properties: {
    id: { type: 'string' }, target: { type: 'string' },
    primary_data: {
      type: 'object',
      properties: {
        metric_type: { type: 'string' }, values: { type: 'string' }, n: { type: 'string' },
        population: { type: 'string' }, study: { type: 'string' }, byte_quote: { type: 'string' },
      },
    },
    chain: { type: 'array', items: { type: 'object', required: ['hop', 'source_queried', 'found', 'supporting_quote'], properties: {
      hop: { type: 'number' }, source_queried: { type: 'string' }, resolved_url: { type: 'string' },
      found: { type: 'boolean' }, supporting_quote: { type: 'string' }, gives_numbers: { type: 'boolean' }, next_citation: { type: 'string' } } } },
    computed_lr: { type: 'number', description: 'the LR computed FROM primary data; 0 if ungroundable' },
    lr_formula: { type: 'string' },
    verdict: { type: 'string', enum: ['grounded', 'direction_only', 'fabricated'] },
    note: { type: 'string' },
  },
}

const prompt = (f) => `You are a forward byte-provenance crawler BUILDING a grounded clinical corpus, case-blind, for ${DOMAIN}. Find the PRIMARY DATA for one diagnostic finding and COMPUTE its likelihood ratio. Do NOT invent a number.

FINDING (${f.id}): ${f.finding}(${f.state}) — ${f.desc}
  bearing on: ${TARGET}
  expected metric: ${f.lr_type}

PROCEDURE (WebSearch/WebFetch; recurse through citations to PRIMARY data):
1. Find the landmark study / meta-analysis reporting test characteristics for THIS finding. Likely anchors: ${ANCHORS}.
2. Capture the LITERAL sentence stating the numbers (byte_quote). Follow citations to the source of the number.
3. COMPUTE the LR: positive finding LR+ = sens/(1-spec); negative finding LR- = (1-sens)/spec; if only an OR or a published stratum-specific LR exists for a clinical criterion, use it and say so; for the prior, report prevalence as a probability. Show the arithmetic in lr_formula.
4. verdict=grounded ONLY if computed_lr comes from byte-anchored real numbers; direction_only if the source supports direction but states no usable quantity (computed_lr=0); fabricated if no support.

Return id, target=${TARGET}, primary_data, chain, computed_lr, lr_formula, verdict, note.`

const results = await pipeline(FINDINGS,
  (f) => agent(prompt(f), { phase: 'Ground', label: `ground:${f.id}:${f.finding}`, agentType: 'general-purpose', schema: SCHEMA }).catch(() => null))
const done = results.filter(Boolean)
const tally = {}; for (const r of done) tally[r.verdict] = (tally[r.verdict] || 0) + 1
log(`strep corpus grounding: ${done.length}/${FINDINGS.length}; verdicts ${JSON.stringify(tally)}`)
return { domain: 'streptococcal_pharyngitis', target: TARGET, tally, per_finding: done }
