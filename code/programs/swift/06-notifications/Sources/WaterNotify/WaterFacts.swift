// WaterFacts.swift
//
// The canonical list of 10 science-backed facts about water and the human body.
//
// WHY A SEPARATE FILE?
//   The facts are data, not scheduling logic. Separating them means:
//   • NotificationSchedule stays focused on timing and identifiers.
//   • Future stages (onboarding cards, "did you know?" UI) can use WaterFacts
//     without depending on UserNotifications.
//   • Translations and editorial changes happen in one place.
//
// MEDICAL SOURCES:
//   Institute of Medicine — Dietary Reference Intakes for Water (2004)
//   Popkin et al. — "Water, Hydration and Health", Nutrition Reviews (2010)
//   Ganong's Review of Medical Physiology, 25th ed. — blood & synovial fluid
//   Guyton & Hall — Textbook of Medical Physiology, 14th ed. — kidney GFR
//   Armstrong et al. — Journal of Nutrition (2012) — mild dehydration & cognition
//   Mow & Huiskes — Basic Orthopaedic Biomechanics — synovial fluid
//   Armstrong — Exertional Heat Illnesses, Human Kinetics (2003) — thermoregulation
//   Tikkinen et al. — European Urology (2016) — nocturia & sleep fragmentation
//   Barrett — Gastrointestinal Physiology, Lange (2017) — GI secretions
//   Verdier-Sévrain & Bonté — J. Cosmet. Dermatology (2007) — skin elasticity
//
// LENGTH CONSTRAINT:
//   Every fact is ≤ 150 characters — the maximum that fits on the iOS lock screen
//   and Apple Watch notification mirror without truncation.
//
// FACT INDEX → NOTIFICATION SLOT:
//   NotificationSchedule assigns WaterFacts.all[i] to slot i.
//   With 10 facts and 8 daily slots, every slot shows a distinct fact.
//   Facts at index 8 and 9 are reserved for future notification slots or UI use.

import Foundation

enum WaterFacts {

    /// 10 medically verified hydration facts, each ≤ 150 characters.
    static let all: [String] = [

        // 0 — Overnight fluid loss  →  07:00 morning slot
        // The body loses 300–500 ml nightly through respiration and skin evaporation
        // (insensible fluid loss). Source: Ganong's Medical Physiology.
        "You lose around 500 ml overnight just breathing. Two glasses now restores your baseline before the day begins.",

        // 1 — Brain and cognitive performance  →  09:00 mid-morning slot
        // 1–2% dehydration degrades working memory, reaction time, and mood.
        // Source: Popkin et al., Nutrition Reviews (2010).
        "Your brain is 75% water. Losing just 1–2% of body fluids measurably reduces concentration, short-term memory, and reaction time.",

        // 2 — Kidney filtration  →  11:00 late-morning slot
        // The kidneys filter ~180 L of plasma per day; glomerular filtration rate
        // drops measurably when fluid intake is insufficient.
        // Source: Guyton & Hall, Textbook of Medical Physiology, 14th ed.
        "Your kidneys filter around 180 litres of blood every day. Adequate water keeps this continuous waste removal working at full speed.",

        // 3 — Blood viscosity  →  13:00 lunch slot
        // Plasma is ~90% water; dehydration raises viscosity and increases cardiac
        // afterload. Source: Ganong's Review of Medical Physiology, 25th ed.
        "Blood is roughly 90% water. Dehydration thickens it, forcing your heart to work harder to push oxygen to every organ in your body.",

        // 4 — Afternoon energy dip  →  15:00 afternoon slot
        // Mild dehydration lowers plasma volume, reducing cerebral oxygen delivery.
        // Source: Armstrong et al., Journal of Nutrition (2012).
        "The 3 pm energy dip is often mild dehydration, not a caffeine deficit. Water restores plasma volume and oxygen delivery faster.",

        // 5 — Joint lubrication  →  17:00 late-afternoon slot
        // Synovial fluid is an ultrafiltrate of blood plasma; its volume and
        // lubricity depend on adequate hydration.
        // Source: Mow & Huiskes, Basic Orthopaedic Biomechanics.
        "Synovial fluid — the cushion inside your joints — is mostly water. Dehydration thickens it, increasing friction and cartilage wear.",

        // 6 — Thermoregulation  →  19:00 evening slot
        // Sweat rate can reach 0.5–2 L/hr in the heat; a 2% fluid deficit raises
        // core temperature by ~0.3°C. Source: Armstrong, Exertional Heat Illnesses.
        "Water is your body's thermostat. Without enough, core temperature rises faster during activity and heat exhaustion sets in sooner.",

        // 7 — Sleep quality  →  21:00 night slot
        // Nocturia (waking to urinate at night) is a recognised disruptor of
        // slow-wave and REM sleep. Source: Tikkinen et al., European Urology (2016).
        "Drinking too much close to bedtime increases night waking and fragments REM sleep. Wind down your water intake for the night.",

        // 8 — Digestion  (reserved for future notification slot or UI use)
        // The GI tract requires ~1–2 L/day for gastric and intestinal secretions.
        // Source: Barrett, Gastrointestinal Physiology, Lange (2017).
        "Water is essential for stomach acid, breaking down food, and moving waste through your intestines. Dehydration is a leading cause of constipation.",

        // 9 — Skin elasticity  (reserved for future notification slot or UI use)
        // Stratum corneum water content directly correlates with elasticity and turgor.
        // Source: Verdier-Sévrain & Bonté, J. Cosmet. Dermatology (2007).
        "Skin cells are plumped by water. Studies show chronically dehydrated skin has measurably lower elasticity and shows fine lines earlier.",
    ]
}
