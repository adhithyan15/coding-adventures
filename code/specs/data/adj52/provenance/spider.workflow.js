export const meta = {
  name: 'adj52-provenance-spider',
  description: 'Recursive byte-provenance spider for case-5: for each quantitative claim (prior/contributes/mechanism LR), start at the cited source, fetch it, byte-anchor the supporting span, and follow citations RECURSIVELY until primary data is reached — or terminate as direction-only / unsupported. Forces every magnitude to be traced to the paper/study it came from, or exposed as fabricated.',
  phases: [{ title: 'Spider' }],
}

// case-5 load-bearing claims (GU-TB vs bladder carcinoma). Embedded directly
// (args delivery is unreliable). Each is a magnitude the deriver asserted.
const CLAIMS = [
  { claim_id: 'c1', kind: 'prior', magnitude: 0.18, evidence: null, target: 'diagnosis(genitourinary_tuberculosis)', source: 'StatPearls NBK557558 Genitourinary Tuberculosis; F1000Research 9:1345' },
  { claim_id: 'c2', kind: 'prior', magnitude: 0.15, evidence: null, target: 'diagnosis(bladder_urothelial_carcinoma)', source: 'PMC9411696 Etiology/Epidemiology of Bladder Cancer; Cleveland Clinic Bladder Cancer' },
  { claim_id: 'c3', kind: 'contributes', magnitude: 3.0, evidence: 'tb_igra(low_positive)', target: 'diagnosis(genitourinary_tuberculosis)', source: 'StatPearls NBK557558 (IGRA helpful as initial screen; positive does not confirm active)' },
  { claim_id: 'c4', kind: 'contributes', magnitude: 4.0, evidence: 'course(refractory_relapsing_despite_fluoroquinolone)', target: 'diagnosis(genitourinary_tuberculosis)', source: 'StatPearls NBK557558 (chronic prostatitis not resolving with standard antibiotics should raise suspicion of GU-TB); PMC4549703 sterile pyuria' },
  { claim_id: 'c5', kind: 'contributes', magnitude: 2.0, evidence: 'urine_leukocytes(mildly_elevated)', target: 'diagnosis(genitourinary_tuberculosis)', source: 'StatPearls NBK557558 (persistent sterile pyuria); NEJM Sterile Pyuria review' },
  { claim_id: 'c6', kind: 'contributes', magnitude: 2.5, evidence: 'renal_finding(calcified_foci)', target: 'diagnosis(genitourinary_tuberculosis)', source: 'ScienceDirect Kidney Tuberculosis overview (calcification 40-70%); RadioGraphics 2021 GU-TB imaging' },
  { claim_id: 'c7', kind: 'mechanism', magnitude: 4.5, evidence: 'cystoscopy ulceration + congestion/edema + inflammatory granulation tissue + polypoid fibrovascular change (one TB bladder lesion described 4 ways)', target: 'diagnosis(genitourinary_tuberculosis)', source: 'StatPearls NBK557558 (urothelial erosions, ulcers, granulomas on cystoscopy); Pathology Outlines granulomatous cystitis' },
  { claim_id: 'c8', kind: 'contributes', magnitude: 3.0, evidence: 'cytology(atypical_clusters_worrisome_for_malignancy)', target: 'diagnosis(bladder_urothelial_carcinoma)', source: 'NYU Urology CIS case (suspicious cytology); PMID 3856982 urinary cytology of bladder TB' },
  { claim_id: 'c9', kind: 'contributes', magnitude: 0.4, evidence: 'cystoscopy_finding(no_discrete_tumor)', target: 'diagnosis(bladder_urothelial_carcinoma)', source: 'PMC8000909 Imaging of bladder cancer (papillary tumors are discrete masses); Pathology Outlines bladder CIS (CIS flat)' },
  { claim_id: 'c10', kind: 'contributes', magnitude: 0.4, evidence: 'ct_finding(no_mass_no_wall_thickening_no_nodes)', target: 'diagnosis(bladder_urothelial_carcinoma)', source: 'PMC8000909 / MDPI Cancers 13:1396 (bladder cancer shows wall thickening / mass / nodal disease on CT)' },
  { claim_id: 'c11', kind: 'contributes', magnitude: 0.6, evidence: 'smoking_status(lifelong_non_smoker)', target: 'diagnosis(bladder_urothelial_carcinoma)', source: 'Cleveland Clinic Bladder Cancer (smoking >2x risk, ~50% of cases); PMC9411696 epidemiology' },
  { claim_id: 'c12', kind: 'mechanism', magnitude: 2.2, evidence: 'cytology(atypical) + focal moderate-to-severe atypical hyperplasia, read as REACTIVE -> FOR TB', target: 'diagnosis(genitourinary_tuberculosis)', source: 'PMID 3856982 (urinary cytology of bladder TB: atypical urothelial cells correlate to benign reversible hyperplasia)' },
  { claim_id: 'c13', kind: 'mechanism', magnitude: 0.5, evidence: 'cytology(atypical) + focal moderate-to-severe atypical hyperplasia, read as REACTIVE -> AGAINST carcinoma. *** THE DECISIVE CLAUSE: it converts the cancer-defining findings into anti-cancer evidence ***', target: 'diagnosis(bladder_urothelial_carcinoma)', source: 'PMID 3856982 (type I/II atypia in TB are benign reversible hyperplasia); PMID 11186722 (post-TB cystitis follow-up)' },
  { claim_id: 'c14', kind: 'contributes', magnitude: 9.0, evidence: 'urine_tb(afb_or_culture_or_naat_positive)', target: 'diagnosis(genitourinary_tuberculosis)', source: 'StatPearls NBK557558 (urine culture/NAAT positive confirms GU-TB; PCR 95.6% sens / 98.1% spec)' },
  { claim_id: 'c15', kind: 'contributes', magnitude: 0.2, evidence: 'urine_tb(afb_and_culture_and_naat_negative)', target: 'diagnosis(genitourinary_tuberculosis)', source: 'StatPearls NBK557558 (repeatedly negative urine TB testing lowers GU-TB; sensitivity imperfect)' },
  { claim_id: 'c16', kind: 'contributes', magnitude: 8.0, evidence: 'tissue_tb(granuloma_afb_or_pcr_positive)', target: 'diagnosis(genitourinary_tuberculosis)', source: 'PMID 40176973 (caseating granuloma + AFB/PCR on bladder tissue confirms TB cystitis)' },
  { claim_id: 'c17', kind: 'contributes', magnitude: 0.15, evidence: 'tissue_tb(carcinoma_confirmed)', target: 'diagnosis(genitourinary_tuberculosis)', source: 'PMID 11186722 (confirmed carcinoma reclassifies the lesion away from pure TB)' },
  { claim_id: 'c18', kind: 'contributes', magnitude: 9.0, evidence: 'tissue_tb(carcinoma_confirmed)', target: 'diagnosis(bladder_urothelial_carcinoma)', source: 'Pathology Outlines bladder CIS (tissue confirmation of carcinoma)' },
  { claim_id: 'c19', kind: 'contributes', magnitude: 0.2, evidence: 'tissue_tb(granuloma_afb_or_pcr_positive)', target: 'diagnosis(bladder_urothelial_carcinoma)', source: 'PMID 3856982 (granulomatous TB with reactive atypia is not carcinoma)' },
]

const SCHEMA = {
  type: 'object',
  required: ['claim_id', 'cited_sources', 'chain', 'terminal', 'verdict', 'verdict_note'],
  properties: {
    claim_id: { type: 'string' },
    cited_sources: { type: 'array', items: { type: 'string' }, description: 'the source(s) the claim cites, parsed out' },
    chain: {
      type: 'array',
      description: 'the recursive provenance walk, one entry per hop, ordered from the cited source down toward primary data',
      items: {
        type: 'object',
        required: ['hop', 'source_queried', 'found', 'supporting_quote', 'gives_magnitude'],
        properties: {
          hop: { type: 'number' },
          source_queried: { type: 'string', description: 'what was fetched at this hop (PMID / URL / book section)' },
          resolved_url: { type: 'string', description: 'the actual URL/identifier reached' },
          found: { type: 'boolean', description: 'was the source located and readable' },
          supporting_quote: { type: 'string', description: 'the LITERAL text span from the fetched source that bears on the claim (byte-anchored; empty if none found)' },
          supports_direction: { type: 'boolean', description: 'does the span support the DIRECTION of the claim (raises/lowers)' },
          gives_magnitude: { type: 'boolean', description: 'does the span give an actual quantity (sensitivity/specificity/OR/RR/LR/prevalence/N)' },
          magnitude_found: { type: 'string', description: 'the actual quantity quoted, if any (e.g. "sens 90.6%, spec 88%, n=120")' },
          next_citation: { type: 'string', description: 'if the number is borrowed from a study this source cites, the citation to follow next (empty if terminal)' },
        },
      },
    },
    terminal: { type: 'string', enum: ['primary_data', 'secondary_only', 'direction_only', 'unsupported', 'source_not_found'], description: 'how the recursion ended' },
    verdict: { type: 'string', enum: ['grounded', 'direction_only', 'fabricated', 'unverifiable'], description: 'grounded = magnitude traces to primary data; direction_only = source supports the direction but NOT the magnitude (the number was invented to look precise); fabricated = source does not even support the direction, or cannot be found; unverifiable = could not complete the trace' },
    data_derived_magnitude: { type: 'string', description: 'what the LR/prior SHOULD be based on the primary data actually found (e.g. an LR computed from sens/spec), or empty if no data exists to ground it' },
    verdict_note: { type: 'string', description: 'concise: did the cited source actually justify THIS magnitude? if the same paper is cited for multiple contradictory claims, say so.' },
  },
}

const prompt = (c) => `You are a byte-provenance spider, like a search-engine crawler that recursively follows citations to their root. Your job: determine whether ONE quantitative claim in a clinical rulebook is actually grounded in published data, or was a number invented to look authoritative.

THE CLAIM (${c.claim_id}):
  ${c.kind === 'prior' ? 'PRIOR probability' : c.kind === 'mechanism' ? 'MECHANISM likelihood ratio' : 'likelihood ratio (contributes)'} = ${c.magnitude}
  ${c.evidence ? `evidence/findings: ${c.evidence}` : '(base rate)'}
  bearing on: ${c.target}
  CITED SOURCE(S): ${c.source}

RECURSIVE PROCEDURE (this is the whole point — do NOT stop at the first page):
1. Parse the cited source(s) into concrete handles (PMID, PMC id, book id, named guideline, URL).
2. For each, use WebSearch/WebFetch to LOCATE and READ it. Capture the LITERAL text span (a real quote, byte-anchored) that bears on this claim.
3. Decide: does that span give an actual MAGNITUDE (sensitivity, specificity, odds ratio, relative risk, likelihood ratio, prevalence %, or an N) — or only a DIRECTION ("raises suspicion", "is associated with")?
4. If the span states a number but BORROWS it from a study it cites, FOLLOW that citation (fetch it) and repeat — recurse down toward PRIMARY DATA. Go up to 3 hops.
5. Stop when you reach: primary data with the actual number (terminal=primary_data); a source that supports only the direction, never the magnitude (terminal=direction_only); a source that does not support even the direction, or that you cannot find (terminal=unsupported / source_not_found).

THEN JUDGE THE MAGNITUDE ${c.magnitude}:
- verdict=grounded  -> the magnitude traces to real data (give data_derived_magnitude — what the number SHOULD be).
- verdict=direction_only -> the source supports the direction but the SPECIFIC NUMBER ${c.magnitude} is not in any source; it was chosen to look precise.
- verdict=fabricated -> the source does not support even the direction, or cannot be located, or (critically) the same paper is being cited to justify several DIFFERENT magnitudes in different directions.
- Be skeptical and exact. A real-but-on-topic citation that does not state the number is direction_only, NOT grounded. Clinical narrative sources (StatPearls, Pathology Outlines, UpToDate) almost never publish likelihood ratios; if the LR is not literally in the text, it is not grounded by that source.

Return the full chain (every hop with its literal supporting_quote), the terminal, the verdict, data_derived_magnitude, and verdict_note.`

const results = await pipeline(
  CLAIMS,
  (c) => agent(prompt(c), { phase: 'Spider', label: `spider:${c.claim_id}`, agentType: 'general-purpose', schema: SCHEMA }).catch(() => null),
)
const done = results.filter(Boolean)
const tally = {}
for (const r of done) { tally[r.verdict] = (tally[r.verdict] || 0) + 1 }
log(`provenance spider done: ${done.length}/${CLAIMS.length}; verdicts ${JSON.stringify(tally)}`)
return { tally, per_claim: done }
