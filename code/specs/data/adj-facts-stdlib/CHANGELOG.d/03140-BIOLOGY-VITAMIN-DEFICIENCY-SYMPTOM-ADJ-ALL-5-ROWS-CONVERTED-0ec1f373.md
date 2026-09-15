- `biology/vitamin-deficiency-symptom.adj` — all 5 rows converted to per-row provenance
  (RS-5e, #14986), which is also the fix for the **#14124 note this file's own test already
  carried**: *"The table ships ONE envelope for five rows, and that envelope is the
  xerophthalmia/night-blindness sentence — which grounds vitamin_a and NOT vitamin_d. Pinning
  vitamin_d would pair an answer about bone deformity with a citation about vision, and freeze
  it in a test."* Five rows, five NIH ODS fact sheets, one vitamin each; the other four spans
  were already here as untiered `cites`, in row order.

  **One span was widened, and the pairing check is what found it.** The shipped vitamin-A span
  — *"Xerophthalmia is the inability to see in low light…"* — states the **symptom** and never
  names vitamin A, so on its own it did not warrant `(vitamin_a,
  inability_to_see_in_low_light)`; the link came from the page it sat on rather than from the
  quoted run. The page's preceding sentence supplies it and is contiguous. **The pin moved with
  the span**, rather than the span being kept to satisfy the pin.

  **The vitamin names differ from the atom labels on two pages** — `vitamin_b1` is "thiamin"
  and `vitamin_b9` is "folate" — so the pairing check uses a stated mapping rather than the row
  key. A key-in-span test would have called both unwarranted, the same trap as `xray` being
  "X-rays" in `wave-types`.

  The span carries a **curly apostrophe (U+2019)** in "isn't"; ASCII-ifying it is a mutant.

  `facts_vitamindeficiencysymptom_e2e.rs`: 7 tests, including the #15193 structural check (five
  distinct spans, five distinct fact sheets, no table-level `cites`). **13 of 13 mutants
  killed**, baseline green — including re-narrowing the vitamin-A span and ASCII-ifying the
  apostrophe.
