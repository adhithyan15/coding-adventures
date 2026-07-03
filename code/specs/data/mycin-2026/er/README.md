# The ER spine (C3) — someone walks in, speaks, gets triaged. Fully local.

The "someone comes into the ER" demo the project is aimed at, end to end and
entirely on-device:

```
voice / typed prose
   │  transcribe        (mlx-whisper, on-device — audio never leaves the machine)
   ▼  transcript
   │  decompose_text    (the ONE local model call — C2 backend selection)
   ▼  typed findings
   │  ir_to_adj → decide   (CPU engine, 0 answer-time model calls, byte-cited)
   ▼  differential
   │  triage            (grounded ESI acuity + immediate actions, 0 model calls)
   ▼  ACUITY + IMMEDIATE-ACTION CHECKLIST + audit trail
```

Everything after the single decompose is CPU-bound, so the patient's words never
leave the machine. **Decision support only** — the acuity and actions are a
grounded, overridable starting checklist the triage nurse / physician reviews; it
never replaces clinical judgment.

## Verified live, on-device

```
$ python3 run_er.py "Fever, neck stiffness, and a witnessed seizure. CSF shows neutrophil-predominant pleocytosis with low glucose."

[1] FINDINGS: csf_neutrophilic_pleocytosis(high), csf_glucose(low), fever(present), meningismus(present), seizure(present)
[2] bacterial_meningitis P = 0.978  <- leading   (0 answer-time model calls)
[3] TRIAGE — ESI acuity 1 (resuscitation)   [rule: red_flag:seizure(present)]
      - ABC + immediate physician; treat seizure (benzodiazepine), consider raised ICP / status epilepticus
answer-time model calls: 0   |   audio/data left the machine: none
```

The **seizure red flag escalated to ESI 1**, *above* the bacterial-meningitis
diagnosis acuity (ESI 2) — a high-risk presentation is emergent before the
diagnosis is even settled. That precedence is the point of the triage layer.

## Files

- `transcribe.py` — voice → transcript via `mlx-whisper` (on-device). Graceful: an
  audio file is transcribed if mlx-whisper is installed; a string that is already
  text passes straight through (the typed-transcript path CI uses). Raises only
  when given audio without mlx-whisper — never silently drops it.
- `triage_rules.json` — the grounded triage rules: red-flag findings, per-diagnosis
  ESI acuity + immediate-action bundles + time targets, and the undifferentiated
  default. Sourced from ESI v4 (Gilboy/AHRQ 2011) + IDSA Tunkel 2004 (door-to-
  antibiotic < 1 h). **Authored from standard references, not spider-grounded** —
  marked, and one CAS-style edit overridable, the same honesty boundary as the rest.
- `triage.py` — maps (leading diagnosis + determinacy, findings) → acuity + actions.
  Precedence: red flag (most acute) → sufficient-evidence diagnosis →
  undifferentiated default (urgent, never under-triaged).
- `run_er.py` — the spine (`python3 run_er.py "<prose>"` or `<recording.wav>`).
- `test_er.py` — guards the triage logic deterministically (red-flag precedence,
  bacterial = emergent + time target, viral = urgent, undifferentiated = urgent-
  not-low) + the transcribe passthrough. No audio / model required; CI runs it.

## Honesty / limits

- The triage acuities + action bundles are authored from standard guidance, not yet
  spider-grounded; one edit overridable.
- Live extraction quality tracks the local model (see `../bench/BENCH_FINDINGS.md`):
  llama3.1:8b extracts the findings reliably; the ~1 GB floor model is noisier and
  may under-extract — in which case the spine **safely abstains to "urgent"** rather
  than under-triage (it never fabricates findings or an acuity).
- Scope is the meningitis differential (the rest of the ER is the roadmap's next
  domains); the spine itself is domain-agnostic — point it at another rulebook +
  triage_rules and it works unchanged.
