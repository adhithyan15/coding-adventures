# Arm A — plain Claude (no framework), verbatim

The PMC11999957 case given to plain Claude with no rulebook, no corpus, no forced
structure, no web lookup — just the clinical question. This is the control arm
ADJ55 §5 flagged as not-yet-run. **Ground truth: PE was present** (CTPA filling
defect, DVT confirmed; pretest scores were low but PE was real).

## Prompt

> You are an experienced physician. A colleague hands you the following case and
> asks for your read. Reason it through and answer naturally — there is no template
> to fill in. Do NOT search the web or look up any published case; reason from your
> own clinical knowledge.
>
> CASE: "A 55-year-old man presented with exertional chest pain, palpitations, and
> dyspnea. HR 80, no leg edema. ECG showed ST-elevation mimicking ACS. D-dimer
> elevated. Wells score 0 (low clinical probability). Angiography clean."
>
> 1. Most likely diagnosis + differential? 2. Specifically, probability of PE? 3.
> What would you do next? 4. How confident, and what would change your mind?

(Identical case prose to the one the framework arms received.)

## Response (verbatim)

**Most likely diagnosis:** a STEMI-mimic / MINOCA-type presentation; leading
diagnosis **acute pericarditis / myopericarditis**, then myocarditis, Takotsubo,
vasospasm/SCAD, aortic pathology. PE is "on the list because of the elevated
D-dimer and dyspnea, but … a poor fit."

**Probability of PE:** *"Low. I'd put it around **3–5%**, roughly at or below the
population baseline."* Reasoning: Wells 0, HR 80 (not tachycardic), no leg edema,
clean angiogram; *"The elevated D-dimer is almost certainly explained by the
inflammatory/myocardial process … not by clot. A positive D-dimer on top of a low
Wells score does not raise PE probability much … PE is a distractor here."*

**What next:** troponin, echo, careful ECG re-read, inflammatory markers. On PE
specifically: *"rather than reflexively CT every such patient … If a coherent
pericarditis/myocarditis picture consolidates and there's no RV strain or hypoxia,
I'd be comfortable not doing CTPA. If anything points to RV strain, hypoxia, or
syncope, I'd get a CTPA promptly."*

**Confidence:** *"high confidence that PE is not the answer."* Bottom line: *"don't
let the elevated D-dimer hijack the case … reserve CTPA for actual signs of RV
strain or hypoxia rather than the D-dimer alone."*

## Verdict

**Incorrect.** Sophisticated reasoning — it correctly reframed the ECG/clean-angio
picture as a cardiac mimic and even named the anchoring trap — but its operative
conclusion was to put PE at 3–5% and **be comfortable not imaging**, on a patient
who had a PE. The two errors are quantified in
[`three-arm-comparison.md`](three-arm-comparison.md): it under-anchored the base
rate (~3–5% vs the grounded 0.192) and let the better-fitting narrative override
the positive-D-dimer's standing mandate to image.
