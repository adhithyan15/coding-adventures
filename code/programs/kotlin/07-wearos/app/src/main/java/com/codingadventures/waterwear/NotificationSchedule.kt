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
    NotificationItem(
        tag   = "waterwear_morning",
        hour  = 7,
        title = "Good morning!",
        body  = "Two glasses now restores your baseline. You lose ~500ml overnight."
    ),
    NotificationItem(
        tag   = "waterwear_mid_morning",
        hour  = 9,
        title = "Time to hydrate",
        body  = "Brain is 75% water. Dehydration reduces focus."
    ),
    NotificationItem(
        tag   = "waterwear_late_morning",
        hour  = 11,
        title = "Nearly noon",
        body  = "Keep kidneys flushing waste efficiently."
    ),
    NotificationItem(
        tag   = "waterwear_lunch",
        hour  = 13,
        title = "Lunchtime",
        body  = "A glass before meals aids digestion."
    ),
    NotificationItem(
        tag   = "waterwear_afternoon",
        hour  = 15,
        title = "Afternoon slump?",
        body  = "Dehydration causes the 3pm energy dip."
    ),
    NotificationItem(
        tag   = "waterwear_late_afternoon",
        hour  = 17,
        title = "Late afternoon",
        body  = "Hydrate before your workout, not after."
    ),
    NotificationItem(
        tag   = "waterwear_evening",
        hour  = 19,
        title = "Evening reminder",
        body  = "Liver works overnight — keep it supplied."
    ),
    NotificationItem(
        tag   = "waterwear_night",
        hour  = 21,
        title = "Last call",
        body  = "Stop here. Late drinking interrupts sleep."
    )
)
