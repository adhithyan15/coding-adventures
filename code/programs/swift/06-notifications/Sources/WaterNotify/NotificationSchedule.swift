// NotificationSchedule.swift
//
// The static, science-backed daily notification schedule.
// All copy lives here — one file, one place to edit.
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
//   - Each body is 80–110 characters — readable in 3 seconds on the lock screen
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

        // 07:00 — special morning slot, encourages TWO glasses to compensate
        // for overnight loss (~500 ml via respiration and insensible perspiration)
        NotificationItem(
            id:    "watersync.morning",
            hour:  7,
            title: "Good morning! Start hydrated",
            body:  "You lose around 500 ml overnight just breathing. Two glasses now restores your baseline before the day begins."
        ),

        // 09:00 — cognitive function fact (highly relatable for desk workers)
        NotificationItem(
            id:    "watersync.mid-morning",
            hour:  9,
            title: "Mid-morning check-in",
            body:  "Your brain is 75% water. Even mild dehydration — just 1–2% fluid loss — reduces focus and reaction time."
        ),

        // 11:00 — kidney function (less well-known, genuinely interesting)
        NotificationItem(
            id:    "watersync.late-morning",
            hour:  11,
            title: "Nearly noon",
            body:  "Water helps your kidneys flush waste continuously. Staying topped up keeps them running at full efficiency."
        ),

        // 13:00 — digestion tie-in (relatable at mealtimes)
        NotificationItem(
            id:    "watersync.lunch",
            hour:  13,
            title: "Lunchtime hydration",
            body:  "A glass before meals aids digestion and gives your stomach a head start on breaking down food."
        ),

        // 15:00 — afternoon energy dip (the most relatable fact of the set)
        NotificationItem(
            id:    "watersync.afternoon",
            hour:  15,
            title: "Afternoon slump?",
            body:  "The 3pm energy dip is often dehydration in disguise. A glass of water works faster than another coffee."
        ),

        // 17:00 — muscles (relevant for post-work exercise crowd)
        NotificationItem(
            id:    "watersync.late-afternoon",
            hour:  17,
            title: "Late afternoon",
            body:  "Muscles are about 75% water. If you exercise after work, start hydrating now — not when you arrive."
        ),

        // 19:00 — liver/kidney overnight processing (useful, novel angle)
        NotificationItem(
            id:    "watersync.evening",
            hour:  19,
            title: "Evening reminder",
            body:  "Your liver and kidneys work through the night processing today. Keep them well supplied."
        ),

        // 21:00 — deliberate "stop here" cue. Reduces anxiety about hitting
        // the goal late at night. Also prevents sleep-disrupting late hydration.
        NotificationItem(
            id:    "watersync.night",
            hour:  21,
            title: "Last call for today",
            body:  "Good time to stop for the night. Late drinking can interrupt sleep. Log your final glass and you are done."
        ),
    ]
}
