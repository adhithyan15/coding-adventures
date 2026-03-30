package com.codingadventures.waternotify

/**
 * NotificationSchedule.kt — the data model for daily hydration reminders.
 *
 * This file defines:
 *   1. `HydrationReminder` — a plain data class describing one notification
 *   2. `HYDRATION_REMINDERS` — the list of 8 reminders, one for each window
 *      of the day from morning (7 AM) through night (9 PM)
 *
 * WHY HARDCODE THE TIMES HERE?
 * ─────────────────────────────
 * iOS's UNCalendarNotificationTrigger(repeats: true) schedules a notification
 * to fire every day at a specific H:M:S, permanently, until cancelled. Android
 * has no exact equivalent — AlarmManager.setExact() fires once. We must
 * reschedule for "tomorrow same time" after each firing (in NotificationReceiver)
 * and after every reboot (in BootReceiver).
 *
 * Centralising all 8 schedules here means NotificationHelper, NotificationReceiver,
 * and BootReceiver all share a single source of truth. If you change a time or
 * body text, you change it in one place.
 *
 * NOTIFICATION IDs:
 * ─────────────────
 * Each notification needs a stable Int ID for two purposes:
 *   1. NotificationManager.notify(id, ...) — so the OS can replace a duplicate
 *      if the same notification fires twice (e.g., device was offline).
 *   2. AlarmManager PendingIntent — each alarm needs a unique requestCode so
 *      Android doesn't overwrite one alarm with another.
 *
 * We use the index (0–7) as the id. String identifiers like "watersync_morning"
 * are kept as `tag` for readability in logs.
 *
 * SCIENCE BEHIND THE BODY TEXTS:
 * ───────────────────────────────
 * Each reminder body references a concrete physiological fact:
 *   7 AM  — you lose ~500 ml overnight through respiration (insensible loss)
 *   9 AM  — brain is 75% water; 1–2% dehydration reduces focus
 *   11 AM — kidneys continuously filter ~180 L/day; hydration is critical
 *   1 PM  — water before meals aids gastric enzyme activity
 *   3 PM  — afternoon energy dip is often mild dehydration, not caffeine deficit
 *   5 PM  — muscles are ~75% water; pre-exercise hydration improves performance
 *   7 PM  — liver + kidneys metabolise overnight; keep supply up
 *   9 PM  — late water disrupts REM sleep; wrap up intake for the night
 */
data class HydrationReminder(
    /** Numeric ID (0–7). Used as NotificationManager.notify() id and PendingIntent requestCode. */
    val id: Int,

    /** Human-readable tag for logging. Mirrors the iOS notification identifier. */
    val tag: String,

    /** Hour of day (24-hour clock) when this alarm should fire. */
    val hour: Int,

    /** Minute within the hour. All reminders fire on the hour (:00). */
    val minute: Int = 0,

    /** Notification title — short, shown in bold on the lock screen. */
    val title: String,

    /** Notification body — one sentence of hydration science. */
    val body: String
)

/**
 * The complete daily hydration reminder schedule.
 *
 * 8 reminders × every day = a gentle nudge every ~2 hours during waking hours.
 * The night reminder (9 PM) intentionally encourages users to *stop* drinking
 * to protect sleep quality — good UX acknowledges biological limits.
 */
val HYDRATION_REMINDERS: List<HydrationReminder> = listOf(
    HydrationReminder(
        id = 0,
        tag = "watersync_morning",
        hour = 7,
        title = "Good morning! Start hydrated",
        body = "You lose around 500 ml overnight just breathing. Two glasses now restores your baseline before the day begins."
    ),
    HydrationReminder(
        id = 1,
        tag = "watersync_mid_morning",
        hour = 9,
        title = "Mid-morning check-in",
        body = "Your brain is 75% water. Even mild dehydration — just 1–2% fluid loss — reduces focus and reaction time."
    ),
    HydrationReminder(
        id = 2,
        tag = "watersync_late_morning",
        hour = 11,
        title = "Nearly noon",
        body = "Water helps your kidneys flush waste continuously. Staying topped up keeps them running at full efficiency."
    ),
    HydrationReminder(
        id = 3,
        tag = "watersync_lunch",
        hour = 13,
        title = "Lunchtime hydration",
        body = "A glass before meals aids digestion and gives your stomach a head start on breaking down food."
    ),
    HydrationReminder(
        id = 4,
        tag = "watersync_afternoon",
        hour = 15,
        title = "Afternoon slump?",
        body = "The 3pm energy dip is often dehydration in disguise. A glass of water works faster than another coffee."
    ),
    HydrationReminder(
        id = 5,
        tag = "watersync_late_afternoon",
        hour = 17,
        title = "Late afternoon",
        body = "Muscles are about 75% water. If you exercise after work, start hydrating now — not when you arrive."
    ),
    HydrationReminder(
        id = 6,
        tag = "watersync_evening",
        hour = 19,
        title = "Evening reminder",
        body = "Your liver and kidneys work through the night processing today. Keep them well supplied."
    ),
    HydrationReminder(
        id = 7,
        tag = "watersync_night",
        hour = 21,
        title = "Last call for today",
        body = "Good time to stop for the night. Late drinking can interrupt sleep. Log your final glass and you are done."
    )
)
