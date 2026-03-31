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
 * FACT ROTATION:
 * ──────────────
 * Body text is drawn from WATER_FACTS — see WaterFacts.kt.
 * Each slot uses WATER_FACTS[id % WATER_FACTS.size], giving 8 distinct facts
 * across the day. Facts 8–9 are reserved for future slots or UI use.
 *
 *   7 AM  — overnight fluid loss (~500 ml)
 *   9 AM  — brain 75% water; 1–2% dehydration reduces focus
 *   11 AM — kidneys filter ~180 L/day; hydration maintains GFR
 *   1 PM  — blood ~90% water; dehydration strains the heart
 *   3 PM  — afternoon slump is dehydration, not caffeine deficit
 *   5 PM  — synovial fluid; joint lubrication depends on hydration
 *   7 PM  — water as thermostat; core temp rises when dehydrated
 *   9 PM  — late drinking causes nocturia, fragments REM sleep
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
    // Slot 0 — 07:00  overnight fluid loss (~500 ml insensible loss)
    HydrationReminder(
        id = 0,
        tag = "watersync_morning",
        hour = 7,
        title = "Good morning! Start hydrated",
        body = WATER_FACTS[0]
    ),
    // Slot 1 — 09:00  brain is 75% water; 1–2% dehydration impairs cognition
    HydrationReminder(
        id = 1,
        tag = "watersync_mid_morning",
        hour = 9,
        title = "Mid-morning check-in",
        body = WATER_FACTS[1]
    ),
    // Slot 2 — 11:00  kidneys filter ~180 L/day; hydration maintains GFR
    HydrationReminder(
        id = 2,
        tag = "watersync_late_morning",
        hour = 11,
        title = "Nearly noon",
        body = WATER_FACTS[2]
    ),
    // Slot 3 — 13:00  blood ~90% water; dehydration thickens it
    HydrationReminder(
        id = 3,
        tag = "watersync_lunch",
        hour = 13,
        title = "Lunchtime hydration",
        body = WATER_FACTS[3]
    ),
    // Slot 4 — 15:00  3 pm slump = dehydration, not caffeine deficit
    HydrationReminder(
        id = 4,
        tag = "watersync_afternoon",
        hour = 15,
        title = "Afternoon slump?",
        body = WATER_FACTS[4]
    ),
    // Slot 5 — 17:00  synovial fluid; joint lubrication depends on hydration
    HydrationReminder(
        id = 5,
        tag = "watersync_late_afternoon",
        hour = 17,
        title = "Late afternoon",
        body = WATER_FACTS[5]
    ),
    // Slot 6 — 19:00  water as thermostat; core temp rises when dehydrated
    HydrationReminder(
        id = 6,
        tag = "watersync_evening",
        hour = 19,
        title = "Evening reminder",
        body = WATER_FACTS[6]
    ),
    // Slot 7 — 21:00  late drinking causes nocturia, fragments REM sleep
    HydrationReminder(
        id = 7,
        tag = "watersync_night",
        hour = 21,
        title = "Last call for today",
        body = WATER_FACTS[7]
    )
)
