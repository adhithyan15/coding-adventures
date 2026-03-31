package com.codingadventures.waterwear

/**
 * NotificationSchedule — defines all 8 daily hydration reminders.
 *
 * DATA-DRIVEN DESIGN:
 *   Rather than 8 separate functions (scheduleNotification1, scheduleNotification2...),
 *   we define the schedule as a list of plain data objects. NotificationHelper
 *   iterates this list to schedule or cancel all reminders. Adding a new reminder
 *   only requires adding one entry here — no changes to scheduling logic.
 *
 * NOTIFICATION IDs:
 *   Every Android notification must have an integer ID. We derive it from a
 *   stable string tag so the ID is deterministic across app restarts. This lets
 *   AlarmManager replace (not duplicate) an existing alarm when rescheduling.
 *
 * WEAROS NOTIFICATION CONSTRAINTS:
 *   WearOS notification tiles show approximately 40 characters of body text before
 *   truncating. Bodies are kept concise — the key health fact comes first.
 *   Users can swipe to expand, but the primary message must land in line 1.
 *
 * FACT ROTATION:
 *   Body text is drawn from WATER_FACTS — see WaterFacts.kt (concise watch versions).
 *   8 daily slots use WATER_FACTS[0..7]. Facts 8–9 are reserved for future use.
 */

/** One scheduled hydration reminder. */
data class NotificationItem(
    /** Stable string identifier — used to derive the integer notification ID. */
    val tag: String,
    /** Hour of day (24-hour clock) when the notification fires. */
    val hour: Int,
    /** Short headline — fits on the notification tile header. */
    val title: String,
    /** 1-2 sentence body — front-loaded with the key fact. */
    val body: String
)

/**
 * The full 8-reminder schedule.
 *
 * Schedule rationale (why these hours?):
 *   07:00 — Wake-up: body is dehydrated after ~8h without water.
 *   09:00 — Mid-morning: focus and cognitive performance peak.
 *   11:00 — Pre-lunch: kidney efficiency reminder.
 *   13:00 — Lunch: digestive aid before eating.
 *   15:00 — Afternoon slump: most people feel energy dip around 3 pm.
 *   17:00 — Pre-workout window for many people.
 *   19:00 — Evening: liver processing begins overnight.
 *   21:00 — Last call: drinking after 9 pm fragments sleep.
 *
 * This schedule is identical to the iOS and Android stages for cross-platform
 * consistency — all devices remind the user at the same time.
 */
val NOTIFICATION_SCHEDULE = listOf(
    // Slot 0 — 07:00  overnight fluid loss
    NotificationItem(
        tag   = "waterwear_morning",
        hour  = 7,
        title = "Good morning!",
        body  = WATER_FACTS[0]
    ),
    // Slot 1 — 09:00  brain 75% water; dehydration reduces focus
    NotificationItem(
        tag   = "waterwear_mid_morning",
        hour  = 9,
        title = "Time to hydrate",
        body  = WATER_FACTS[1]
    ),
    // Slot 2 — 11:00  kidneys filter 180 L/day
    NotificationItem(
        tag   = "waterwear_late_morning",
        hour  = 11,
        title = "Nearly noon",
        body  = WATER_FACTS[2]
    ),
    // Slot 3 — 13:00  blood 90% water; dehydration strains the heart
    NotificationItem(
        tag   = "waterwear_lunch",
        hour  = 13,
        title = "Lunchtime",
        body  = WATER_FACTS[3]
    ),
    // Slot 4 — 15:00  3 pm slump = dehydration
    NotificationItem(
        tag   = "waterwear_afternoon",
        hour  = 15,
        title = "Afternoon slump?",
        body  = WATER_FACTS[4]
    ),
    // Slot 5 — 17:00  synovial fluid; joint lubrication
    NotificationItem(
        tag   = "waterwear_late_afternoon",
        hour  = 17,
        title = "Late afternoon",
        body  = WATER_FACTS[5]
    ),
    // Slot 6 — 19:00  water regulates temperature
    NotificationItem(
        tag   = "waterwear_evening",
        hour  = 19,
        title = "Evening reminder",
        body  = WATER_FACTS[6]
    ),
    // Slot 7 — 21:00  late drinking disrupts REM sleep
    NotificationItem(
        tag   = "waterwear_night",
        hour  = 21,
        title = "Last call",
        body  = WATER_FACTS[7]
    )
)
