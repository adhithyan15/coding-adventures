export const meta = {
  name: 'pe-grounded-corpus',
  description: 'Phase 1 of the provenance-first proof: derive a GROUNDED pulmonary-embolism corpus, case-blind. One agent per finding->PE link crawls FORWARD to primary data (PIOPED II, Christopher, Wells, D-dimer meta-analyses), byte-anchors the sensitivity/specificity/OR, and COMPUTES the likelihood ratio from that data. A magnitude is admitted only if it terminates in real numbers; otherwise it is a flagged data-gap, never an invented LR.',
  phases: [{ title: 'Ground' }],
}

// Case-BLIND PE diagnostic skeleton (standard differential discriminators; no case yet).
const FINDINGS = [
  { id: 'f0', finding: 'prior', state: 'base_rate', desc: 'pretest prevalence of PE among ED/outpatients formally worked up for SUSPECTED PE (the base rate before any test)', lr_type: 'prevalence' },
  { id: 'f1', finding: 'd_dimer', state: 'elevated', desc: 'D-dimer above the assay cutoff (highly sensitive quantitative ELISA)', lr_type: 'LR+' },
  { id: 'f2', finding: 'd_dimer', state: 'normal', desc: 'D-dimer below the assay cutoff', lr_type: 'LR-' },
  { id: 'f3', finding: 'clinical_signs_of_dvt', state: 'present', desc: 'objective clinical signs/symptoms of DVT (leg swelling + pain on deep-vein palpation) — a Wells criterion', lr_type: 'OR_or_LR' },
  { id: 'f4', finding: 'pe_is_leading_diagnosis', state: 'present', desc: 'PE judged as likely as or more likely than any alternative — the Wells gestalt criterion', lr_type: 'OR_or_LR' },
  { id: 'f5', finding: 'heart_rate', state: 'over_100', desc: 'tachycardia, heart rate > 100/min — a Wells criterion', lr_type: 'OR_or_LR' },
  { id: 'f6', finding: 'recent_immobilization_or_surgery', state: 'present', desc: 'immobilization >=3 days or surgery within 4 weeks — a Wells criterion', lr_type: 'OR_or_LR' },
  { id: 'f7', finding: 'previous_vte', state: 'present', desc: 'previously objectively diagnosed DVT or PE — a Wells criterion', lr_type: 'OR_or_LR' },
  { id: 'f8', finding: 'hemoptysis', state: 'present', desc: 'hemoptysis — a Wells criterion', lr_type: 'OR_or_LR' },
  { id: 'f9', finding: 'active_malignancy', state: 'present', desc: 'malignancy on treatment / within 6 months / palliative — a Wells criterion', lr_type: 'OR_or_LR' },
  { id: 'f10', finding: 'ctpa', state: 'filling_defect_positive', desc: 'CT pulmonary angiography with an intraluminal filling defect (the confirmatory test)', lr_type: 'LR+' },
  { id: 'f11', finding: 'ctpa', state: 'negative', desc: 'CT pulmonary angiography negative for PE on an adequate study', lr_type: 'LR-' },
]

const SCHEMA = {
  type: 'object',
  required: ['id', 'target', 'chain', 'verdict', 'primary_data', 'note'],
  properties: {
    id: { type: 'string' },
    target: { type: 'string' },
    primary_data: {
      type: 'object',
      description: 'the actual numbers found in primary literature for THIS finding in PE diagnosis',
      properties: {
        metric_type: { type: 'string', description: 'sensitivity_specificity | odds_ratio | likelihood_ratio | prevalence | none' },
        values: { type: 'string', description: 'the literal quantities, e.g. "sens 96.5%, spec 40.6%" or "OR 2.4 (95% CI 1.4-4.1)" or "prevalence ~20%"' },
        n: { type: 'string', description: 'study/pooled sample size' },
        population: { type: 'string', description: 'the population the numbers come from (matters for prevalence/pretest)' },
        study: { type: 'string', description: 'the primary study or meta-analysis (named, with year/PMID/DOI)' },
        byte_quote: { type: 'string', description: 'the LITERAL sentence(s) from the source stating the numbers' },
      },
    },
    chain: {
      type: 'array',
      items: {
        type: 'object',
        required: ['hop', 'source_queried', 'found', 'supporting_quote'],
        properties: {
          hop: { type: 'number' },
          source_queried: { type: 'string' },
          resolved_url: { type: 'string' },
          found: { type: 'boolean' },
          supporting_quote: { type: 'string' },
          gives_numbers: { type: 'boolean' },
          next_citation: { type: 'string' },
        },
      },
    },
    computed_lr: { type: 'number', description: 'the likelihood ratio computed FROM the primary data. LR+ = sens/(1-spec); LR- = (1-sens)/spec; or use the reported OR/LR directly. 0 if ungroundable.' },
    lr_formula: { type: 'string', description: 'show the arithmetic, e.g. "LR+ = 0.965/(1-0.406) = 1.63"' },
    verdict: { type: 'string', enum: ['grounded', 'direction_only', 'fabricated'], description: 'grounded = computed_lr derives from primary numbers byte-anchored above; direction_only = literature supports direction but gives no usable number; fabricated = no support' },
    note: { type: 'string' },
  },
}

const prompt = (f) => `You are a forward byte-provenance crawler BUILDING a grounded clinical corpus — case-blind. Your job: find the PRIMARY DATA for one diagnostic finding and COMPUTE its likelihood ratio from that data. Do NOT invent a number; either it traces to published sensitivity/specificity/OR/prevalence, or you report it as ungrounded.

FINDING (${f.id}): ${f.finding}(${f.state})
  ${f.desc}
  bearing on: diagnosis(pulmonary_embolism)
  expected metric: ${f.lr_type}

PROCEDURE (use WebSearch/WebFetch; recurse through citations to PRIMARY data):
1. Find the landmark study or meta-analysis that reports the test characteristics for THIS finding in PE diagnosis. Anchors you will likely need: PIOPED II (Stein 2006 NEJM, CTPA sens/spec); the Christopher study (JAMA 2006, D-dimer+Wells algorithm, PE prevalence); Wells et al 2000/2001 (score derivation); Cochrane / meta-analyses of D-dimer (e.g. Crawford 2016) and of individual Wells items (e.g. Lucassen 2011 Annals Intern Med). Prefer pooled/meta numbers.
2. Capture the LITERAL sentence stating the numbers (byte_quote). If the number is quoted FROM another study, follow that citation to the source.
3. COMPUTE the likelihood ratio:
   - positive finding: LR+ = sensitivity / (1 - specificity)
   - negative finding: LR- = (1 - sensitivity) / specificity
   - if only an odds ratio / adjusted OR is published for a clinical criterion, use it as the LR proxy and say so.
   - for the prior: report the prevalence of PE in the suspected-PE workup population as a probability (e.g. 0.20).
   Show the arithmetic in lr_formula.
4. Be honest about grounding: verdict=grounded ONLY if computed_lr comes from real numbers you byte-anchored. If the literature supports the direction but you cannot find a usable quantity, verdict=direction_only and computed_lr=0. If no support at all, verdict=fabricated.

Return id, target=diagnosis(pulmonary_embolism), primary_data, chain, computed_lr, lr_formula, verdict, note.`

const results = await pipeline(
  FINDINGS,
  (f) => agent(prompt(f), { phase: 'Ground', label: `ground:${f.id}:${f.finding}`, agentType: 'general-purpose', schema: SCHEMA }).catch(() => null),
)
const done = results.filter(Boolean)
const tally = {}
for (const r of done) { tally[r.verdict] = (tally[r.verdict] || 0) + 1 }
log(`PE corpus grounding done: ${done.length}/${FINDINGS.length}; verdicts ${JSON.stringify(tally)}`)
return { tally, per_finding: done }
