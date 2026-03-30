// WaterFacts.swift  (Shared)
//
// The canonical list of 10 science-backed facts about water and the human body.
// Lives in Sources/Shared so both the iPhone target (WaterSync) and the
// Apple Watch target (WaterSyncWatch) can access it without duplication.
//
// CURRENT USE:
//   Stage 06 (iOS, code/programs/swift/06-notifications) uses an identical copy
//   of these facts to populate notification bodies. This Shared copy means the
//   Watch face can show the same facts in a future "Did you know?" tile or in
//   standalone watchOS reminders (planned for a later stage).
//
// WHY SHARED AND NOT WATCH-ONLY?
//   WatchConnectivity already uses this Shared target for SyncPayload. Adding
//   WaterFacts here follows the same pattern: one definition, both targets.
//   If the iPhone app ever surfaces a facts carousel or onboarding screen, it
//   reads from the same source as the Watch.
//
// MEDICAL SOURCES:
//   Institute of Medicine — Dietary Reference Intakes for Water (2004)
//   Popkin et al. — "Water, Hydration and Health", Nutrition Reviews (2010)
//   Ganong's Review of Medical Physiology, 25th ed.
//   Guyton & Hall — Textbook of Medical Physiology, 14th ed.
//   Armstrong et al. — Journal of Nutrition (2012)
//   Mow & Huiskes — Basic Orthopaedic Biomechanics
//   Armstrong — Exertional Heat Illnesses, Human Kinetics (2003)
//   Tikkinen et al. — European Urology (2016)
//   Barrett — Gastrointestinal Physiology, Lange (2017)
//   Verdier-Sévrain & Bonté — Journal of Cosmetic Dermatology (2007)
//
// LENGTH CONSTRAINT:
//   Every fact is ≤ 150 characters — the maximum that fits on the Apple Watch
//   notification face and iOS lock screen without truncation.

import Foundation

enum WaterFacts {

    /// 10 medically verified hydration facts, each ≤ 150 characters.
    static let all: [String] = [

        // 0 — Overnight fluid loss
        // ~300–500 ml lost nightly via respiration and skin evaporation.
        // Source: Ganong's Medical Physiology.
        "You lose around 500 ml overnight just breathing. Two glasses now restores your baseline before the day begins.",

        // 1 — Brain and cognitive performance
        // 1–2% dehydration degrades working memory, reaction time, and mood.
        // Source: Popkin et al., Nutrition Reviews (2010).
        "Your brain is 75% water. Losing just 1–2% of body fluids measurably reduces concentration, short-term memory, and reaction time.",

        // 2 — Kidney filtration
        // Kidneys filter ~180 L of plasma/day; GFR drops with low fluid intake.
        // Source: Guyton & Hall, Textbook of Medical Physiology, 14th ed.
        "Your kidneys filter around 180 litres of blood every day. Adequate water keeps this continuous waste removal working at full speed.",

        // 3 — Blood viscosity
        // Plasma is ~90% water; dehydration raises viscosity and cardiac afterload.
        // Source: Ganong's Review of Medical Physiology, 25th ed.
        "Blood is roughly 90% water. Dehydration thickens it, forcing your heart to work harder to push oxygen to every organ in your body.",

        // 4 — Afternoon energy dip
        // Mild dehydration lowers plasma volume and cerebral oxygen delivery.
        // Source: Armstrong et al., Journal of Nutrition (2012).
        "The 3 pm energy dip is often mild dehydration, not a caffeine deficit. Water restores plasma volume and oxygen delivery faster.",

        // 5 — Joint lubrication
        // Synovial fluid is an ultrafiltrate of plasma; its lubricity depends on hydration.
        // Source: Mow & Huiskes, Basic Orthopaedic Biomechanics.
        "Synovial fluid — the cushion inside your joints — is mostly water. Dehydration thickens it, increasing friction and cartilage wear.",

        // 6 — Thermoregulation
        // A 2% fluid deficit raises core temperature ~0.3°C during activity.
        // Source: Armstrong, Exertional Heat Illnesses, Human Kinetics (2003).
        "Water is your body's thermostat. Without enough, core temperature rises faster during activity and heat exhaustion sets in sooner.",

        // 7 — Sleep quality
        // Nocturia (waking to urinate at night) disrupts slow-wave and REM sleep.
        // Source: Tikkinen et al., European Urology (2016).
        "Drinking too much close to bedtime increases night waking and fragments REM sleep. Wind down your water intake for the night.",

        // 8 — Digestion (reserved for future notification slot or UI use)
        // GI tract requires ~1–2 L/day for gastric and intestinal secretions.
        // Source: Barrett, Gastrointestinal Physiology, Lange (2017).
        "Water is essential for stomach acid, breaking down food, and moving waste through your intestines. Dehydration is a leading cause of constipation.",

        // 9 — Skin elasticity (reserved for future notification slot or UI use)
        // Stratum corneum water content correlates with elasticity and turgor.
        // Source: Verdier-Sévrain & Bonté, J. Cosmet. Dermatology (2007).
        "Skin cells are plumped by water. Studies show chronically dehydrated skin has measurably lower elasticity and shows fine lines earlier.",
    ]
}
