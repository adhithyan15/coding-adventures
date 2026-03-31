package com.codingadventures.waternotify

/**
 * WaterFacts.kt — canonical list of 10 science-backed hydration facts.
 *
 * WHY A SEPARATE FILE?
 * ────────────────────
 * Facts are data, not scheduling logic. Keeping them here means:
 *   • NotificationSchedule stays focused on timing and IDs.
 *   • A future "Did you know?" UI screen can read from this list without
 *     depending on AlarmManager or notification plumbing.
 *   • Translations and editorial changes happen in one place.
 *
 * MEDICAL SOURCES:
 *   Institute of Medicine — Dietary Reference Intakes for Water (2004)
 *   Popkin et al. — "Water, Hydration and Health", Nutrition Reviews (2010)
 *   Ganong's Review of Medical Physiology, 25th ed. — blood & synovial fluid
 *   Guyton & Hall — Textbook of Medical Physiology, 14th ed. — kidney GFR
 *   Armstrong et al. — Journal of Nutrition (2012) — mild dehydration & cognition
 *   Mow & Huiskes — Basic Orthopaedic Biomechanics — synovial fluid
 *   Armstrong — Exertional Heat Illnesses, Human Kinetics (2003) — thermoregulation
 *   Tikkinen et al. — European Urology (2016) — nocturia & sleep fragmentation
 *   Barrett — Gastrointestinal Physiology, Lange (2017) — GI secretions
 *   Verdier-Sévrain & Bonté — J. Cosmet. Dermatology (2007) — skin elasticity
 *
 * USAGE:
 *   HYDRATION_REMINDERS assigns WATER_FACTS[i % WATER_FACTS.size] to slot i.
 *   With 8 daily slots and 10 facts, every slot shows a distinct fact.
 *   Facts at index 8 and 9 are available for future slots or UI use.
 */
val WATER_FACTS: List<String> = listOf(

    // 0 — overnight fluid loss  →  07:00 morning slot
    // The body loses 300–500 ml nightly via respiration and skin evaporation.
    "You lose around 500 ml overnight just breathing. Two glasses now restores your baseline before the day begins.",

    // 1 — brain & cognitive performance  →  09:00 mid-morning slot
    // 1–2% dehydration degrades working memory, reaction time, and mood.
    "Your brain is 75% water. Losing just 1–2% of body fluids measurably reduces concentration, short-term memory, and reaction time.",

    // 2 — kidney filtration  →  11:00 late-morning slot
    // Kidneys filter ~180 L of plasma per day; GFR drops with low fluid intake.
    "Your kidneys filter around 180 litres of blood every day. Adequate water keeps this continuous waste removal working at full speed.",

    // 3 — blood viscosity  →  13:00 lunch slot
    // Plasma is ~90% water; dehydration raises viscosity and cardiac afterload.
    "Blood is roughly 90% water. Dehydration thickens it, forcing your heart to work harder to push oxygen to every organ in your body.",

    // 4 — afternoon energy dip  →  15:00 afternoon slot
    // Mild dehydration lowers plasma volume and cerebral oxygen delivery.
    "The 3 pm energy dip is often mild dehydration, not a caffeine deficit. Water restores plasma volume and oxygen delivery faster.",

    // 5 — joint lubrication  →  17:00 late-afternoon slot
    // Synovial fluid is an ultrafiltrate of plasma; its lubricity depends on hydration.
    "Synovial fluid — the cushion inside your joints — is mostly water. Dehydration thickens it, increasing friction and cartilage wear.",

    // 6 — thermoregulation  →  19:00 evening slot
    // A 2% fluid deficit raises core temperature ~0.3°C during activity.
    "Water is your body's thermostat. Without enough, core temperature rises faster during activity and heat exhaustion sets in sooner.",

    // 7 — sleep quality  →  21:00 night slot
    // Nocturia is a recognised disruptor of slow-wave and REM sleep.
    "Drinking too much close to bedtime increases night waking and fragments REM sleep. Wind down your water intake for the night.",

    // 8 — digestion  (reserved for future slot or UI use)
    // GI tract requires ~1–2 L/day for gastric and intestinal secretions.
    "Water is essential for stomach acid, breaking down food, and moving waste through your intestines. Dehydration is a leading cause of constipation.",

    // 9 — skin elasticity  (reserved for future slot or UI use)
    // Stratum corneum water content correlates with elasticity and turgor.
    "Skin cells are plumped by water. Studies show chronically dehydrated skin has measurably lower elasticity and shows fine lines earlier."
)
