package com.codingadventures.waterwear

/**
 * WaterFacts.kt — 10 science-backed hydration facts, sized for WearOS tiles.
 *
 * WEAROS DISPLAY CONSTRAINT:
 * ──────────────────────────
 * WearOS notification tiles show ~40 characters before truncating on the watch
 * face. The most important word (the medical claim) must come first so it lands
 * in the visible portion. Users can tap to expand, but the hook must fit in line 1.
 *
 * These are concise versions of the same 10 facts used on iOS and Android.
 * The fact at index i is assigned to notification slot i. Facts 8 and 9 are
 * reserved for future slots or the watch tile UI.
 *
 * MEDICAL SOURCES (same as iOS / Android):
 *   IOM — Dietary Reference Intakes for Water (2004)
 *   Popkin et al., Nutrition Reviews (2010)
 *   Ganong's Medical Physiology, 25th ed.
 *   Guyton & Hall, Medical Physiology, 14th ed.
 *   Armstrong et al., Journal of Nutrition (2012)
 *   Mow & Huiskes, Basic Orthopaedic Biomechanics
 *   Armstrong, Exertional Heat Illnesses (2003)
 *   Tikkinen et al., European Urology (2016)
 *   Barrett, Gastrointestinal Physiology (2017)
 *   Verdier-Sévrain & Bonté, J. Cosmet. Dermatology (2007)
 */
val WATER_FACTS: List<String> = listOf(
    // 0 → 07:00 morning — overnight insensible fluid loss
    "~500 ml lost overnight breathing. Two glasses restores your baseline.",

    // 1 → 09:00 mid-morning — brain is 75% water; 1–2% dehydration harms focus
    "Brain is 75% water. Dehydration reduces focus and reaction time.",

    // 2 → 11:00 late morning — kidneys filter 180 L/day
    "Kidneys filter 180 L/day. Water keeps waste removal at full speed.",

    // 3 → 13:00 lunch — blood is 90% water; dehydration strains the heart
    "Blood is 90% water. Dehydration forces your heart to work harder.",

    // 4 → 15:00 afternoon — 3 pm slump is dehydration, not caffeine deficit
    "3 pm slump = dehydration. Water beats coffee for a quick recovery.",

    // 5 → 17:00 late afternoon — synovial fluid lubricates joints
    "Joint fluid is mostly water. Dehydration increases cartilage wear.",

    // 6 → 19:00 evening — thermoregulation; core temperature rises when dehydrated
    "Water regulates your temperature. Stay cool — keep drinking.",

    // 7 → 21:00 night — late drinking causes nocturia, fragments REM sleep
    "Late drinking disrupts REM sleep. Wind down water intake now.",

    // 8 — digestion (reserved for future slot or tile UI)
    "Water drives digestion. Dehydration is a top cause of constipation.",

    // 9 — skin elasticity (reserved for future slot or tile UI)
    "Hydrated skin stays elastic. Fine lines appear faster when dry."
)
