- `biology/vitamin-deficiency-symptom.adj` (new) — a sibling to the already-shipped `vitamins.adj`
  (`deficiency_disease(vitamin, disease)`, the classic deficiency DISEASE each vitamin's lack
  causes — vitamin_c → scurvy, vitamin_d → rickets, etc.). That table's own header already quotes,
  verbatim, the NIH ODS sentence that states each disease, and for five of the seven vitamins the
  SAME sentence also states the disease's defining SYMPTOM: "the bones become soft, weak,
  deformed, and painful" (vitamin_d), "tingling and numbness in the feet and hands..." (vitamin_b1),
  "makes people tired and weak" (vitamin_b12), and so on. New `vitamin_deficiency_symptom(vitamin,
  symptom)` table: vitamin_a → inability_to_see_in_low_light, vitamin_d →
  soft_weak_deformed_painful_bones, vitamin_b1 → tingling_and_numbness_in_feet_and_hands, vitamin_b9
  → weakness_and_fatigue, vitamin_b12 → tired_and_weak. Honestly narrower than its parent: honest
  abstention on `vitamin_c` and `vitamin_b3`, whose already-cited spans name the disease but state
  no symptom. New e2e test file `facts_vitamindeficiencysymptom_e2e.rs` (3 tests: forward recall
  with citation, backward recall of the tired-and-weak vitamin, honest abstention on vitamin_c). No
  manifest objective, matching `vitamins.adj`'s own precedent of not having one. First slice of a
  fresh, narrower biology/ pass (revisiting a candidate the prior sweep round rated MODERATE and
  deprioritized; closer inspection showed 5 of 7 rows have a fully groundable symptom in the
  already-quoted spans).
