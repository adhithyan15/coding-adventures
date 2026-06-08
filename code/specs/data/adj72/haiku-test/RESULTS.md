# ADJ72 — Haiku-as-worker test

Can a small model (Haiku) run inside the byte-provenance + spider framework and beat its
own closed-book performance on hard HLE questions? Worker phases routed through Haiku
(Agent model=haiku); two questions with verified ground truth.

## Results

| question | Arm A: Haiku alone (closed-book) | Arm B: Haiku + framework (gate → spider) | ground truth |
|---|---|---|---|
| Palmyrene (RIB 1065) | "unable to provide reliable translation"; glossed BT=daughter, BR=son (the filiation trap), conf 0.15 | **"Regina, the freedwoman of Barates, alas"** — found RIB 1065, contradicted the daughter/son reading, cited the inscription | "Regina, the freedwoman of Barates, alas" |
| Hummingbird sesamoid | abstained: answer=null, exists=false, **confidence 0.0** | **UNDERDETERMINED** — found the right primary source (Zusi & Bentz 1984, Smithsonian), but could not extract the 80-page PDF; **refused the contaminated "2"** that casual sources parrot | 4 (Zusi & Bentz, primary source) |

## What worked

- **Haiku-as-worker runs the full pipeline end-to-end.** Closed-book gate → (ungrounded) → Haiku-driven open-book spider → grounded answer. The mechanism works on a small model.
- **Palmyrene: clear lift, wrong → right.** Haiku alone fell into the same filiation trap as Opus and committed nothing; Haiku + framework found RIB 1065 on the open web, overturned the misparse, and produced the correct, cited translation — matching the Opus-driven run. Where the grounding is web-accessible, Haiku fully succeeds.
- **No confident-wrong answers, in either arm, on either question.** This is the headline. Bare frontier models confidently answered the hummingbird question 2 (Gemini) / 3 (GPT-4o). Haiku + framework got Palmyrene **right** and, on hummingbird, **honestly abstained** ("underdetermined; found the source, can't extract it; the web's '2' is uncited"). The discipline degrades Haiku **gracefully to honest non-answer, never to confident fabrication.**

## What hit a ceiling (the honest part)

- **Hummingbird: Haiku found the right primary source but could not extract it.** Opus downloaded and text-extracted the full 80-page Smithsonian PDF to find the verbatim passage; Haiku identified the same source but couldn't pull the text, so it correctly landed at UNDERDETERMINED instead of grounding "4."
- **The capability gap is specifically document retrieval + extraction, not judgment.** Haiku's *judgment* was sound throughout: it identified the correct primary source, and it *refused* the contaminated "2" rather than laundering it (the discrimination gate working). What it lacked was the muscle to fetch-and-parse a large PDF.

## Architecture implication

The spider has a sub-step — fetch-and-extract a long primary document — that is the real capability floor for a small model, and it is **tool-shaped, not intelligence-shaped.** Pair Haiku with a deterministic PDF-fetch-and-extract tool (or route only that sub-step to a stronger model / the CAS cache from ADJ71) and Haiku would very likely clear the hummingbird too, since its judgment about *which* source and *whether* to trust it was already correct.

## Bottom line

Yes — we got it working on Haiku. On the web-groundable question (Palmyrene) Haiku + framework went from wrong to correct-and-cited, matching the frontier-model run. On the PDF-bound question (hummingbird) Haiku hit a retrieval-extraction ceiling and **correctly abstained rather than confabulating** — strictly better than the bare frontier models that confidently answered wrong. The framework's value on a small model is exactly its value on a large one: it converts confident error into either a grounded answer or an honest, auditable "I can't ground this yet — here's the source I found and what's missing."
