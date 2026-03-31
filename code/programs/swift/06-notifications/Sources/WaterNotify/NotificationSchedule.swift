// NotificationSchedule.swift
//
// The static, science-backed daily notification schedule.
// Timing, identifiers, and titles live here.
// Body copy is drawn from WaterFacts — see WaterFacts.swift.
//
// FACT ROTATION:
//   Each slot body is WaterFacts.all[slotIndex]. With 10 facts and 8 slots,
//   every daily reminder shows a distinct medical fact:
//     slot 0 (07:00) → WaterFacts.all[0]  overnight fluid loss
//     slot 1 (09:00) → WaterFacts.all[1]  brain & cognition
//     slot 2 (11:00) → WaterFacts.all[2]  kidney filtration
//     slot 3 (13:00) → WaterFacts.all[3]  blood viscosity
//     slot 4 (15:00) → WaterFacts.all[4]  afternoon energy dip
//     slot 5 (17:00) → WaterFacts.all[5]  joint lubrication
//     slot 6 (19:00) → WaterFacts.all[6]  thermoregulation
//     slot 7 (21:00) → WaterFacts.all[7]  sleep quality
//   Facts 8–9 are reserved for future slots or UI (onboarding, "did you know?").
//
// SCIENCE BASIS:
//   Adults need ~2–2.5 L/day (IOM Dietary Reference Intakes, 2004).
//   Kidneys process at most ~1 L/hour — small frequent sips beat large boluses.
//   Overnight fluid loss is ~500 ml — the 07:00 slot addresses this directly.
//   2-hour spacing across a 7–21 waking window gives 8 reminders = 8 glasses.
//
// COPY GUIDELINES:
//   - 07:00 is the only slot that mentions "two glasses" (overnight loss)
//   - 21:00 explicitly cues the user to stop (no late-night bathroom trips)
//   - No emoji — they render inconsistently in notification banners
//   - Every body is ≤ 150 characters (enforced by NotificationScheduleTests)
//   - Every fact is independently interesting; no slot repeats another's point

import Foundation
import UserNotifications

// ── Data type ─────────────────────────────────────────────────────────────────

/// One scheduled notification slot.
struct NotificationItem {
    /// Stable identifier used as the UNNotificationRequest identifier.
    /// Changing an ID cancels the old notification and creates a new one.
    let id:    String

    /// Wall-clock hour (24-hour format) at which this fires daily.
    /// Minute is always 0 — reminders fire on the hour.
    let hour:  Int

    let title: String
    let body:  String
}

// ── Static schedule ───────────────────────────────────────────────────────────

enum NotificationSchedule {

    /// All 8 daily notification slots, in chronological order.
    /// The order matters for display in settings UIs (future stage) and for
    /// computing "next reminder" — keep it ascending by hour.
    static let all: [NotificationItem] = [

        // Slot 0 — 07:00 morning
        // Special: encourages TWO glasses to compensate for overnight fluid loss.
        // Body: WaterFacts[0] — insensible overnight loss (~500 ml).
        NotificationItem(
            id:    "watersync.morning",
            hour:  7,
            title: "Good morning! Start hydrated",
            body:  WaterFacts.all[0]
        ),

        // Slot 1 — 09:00 mid-morning
        // Body: WaterFacts[1] — brain is 75% water; 1–2% fluid loss impairs focus.
        NotificationItem(
            id:    "watersync.mid-morning",
            hour:  9,
            title: "Mid-morning check-in",
            body:  WaterFacts.all[1]
        ),

        // Slot 2 — 11:00 late morning
        // Body: WaterFacts[2] — kidneys filter ~180 L/day; hydration maintains GFR.
        NotificationItem(
            id:    "watersync.late-morning",
            hour:  11,
            title: "Nearly noon",
            body:  WaterFacts.all[2]
        ),

        // Slot 3 — 13:00 lunch
        // Body: WaterFacts[3] — blood is ~90% water; dehydration thickens it.
        NotificationItem(
            id:    "watersync.lunch",
            hour:  13,
            title: "Lunchtime hydration",
            body:  WaterFacts.all[3]
        ),

        // Slot 4 — 15:00 afternoon
        // Body: WaterFacts[4] — 3 pm slump is dehydration, not caffeine deficit.
        NotificationItem(
            id:    "watersync.afternoon",
            hour:  15,
            title: "Afternoon slump?",
            body:  WaterFacts.all[4]
        ),

        // Slot 5 — 17:00 late afternoon
        // Body: WaterFacts[5] — synovial fluid; joint lubrication depends on hydration.
        NotificationItem(
            id:    "watersync.late-afternoon",
            hour:  17,
            title: "Late afternoon",
            body:  WaterFacts.all[5]
        ),

        // Slot 6 — 19:00 evening
        // Body: WaterFacts[6] — water as thermostat; core temp rises when dehydrated.
        NotificationItem(
            id:    "watersync.evening",
            hour:  19,
            title: "Evening reminder",
            body:  WaterFacts.all[6]
        ),

        // Slot 7 — 21:00 night
        // Deliberate "stop here" cue — late drinking causes nocturia, fragments REM.
        // Body: WaterFacts[7] — drinking late increases night waking.
        NotificationItem(
            id:    "watersync.night",
            hour:  21,
            title: "Last call for today",
            body:  WaterFacts.all[7]
        ),
    ]
}
