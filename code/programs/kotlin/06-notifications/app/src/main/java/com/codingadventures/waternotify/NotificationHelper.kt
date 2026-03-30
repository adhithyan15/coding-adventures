package com.codingadventures.waternotify

import android.app.AlarmManager
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.os.Build
import androidx.core.content.getSystemService
import java.util.Calendar

/**
 * NotificationHelper.kt — creates the notification channel and schedules alarms.
 *
 * This file is the "setup" half of the notification system. It handles:
 *   1. Creating the NotificationChannel (required since Android 8 / API 26)
 *   2. Building a PendingIntent for each reminder
 *   3. Scheduling each alarm with AlarmManager.setExact()
 *
 * The "delivery" half lives in NotificationReceiver.kt — that's where the
 * actual notification is posted when the alarm fires.
 *
 * ═══════════════════════════════════════════════════════════════════
 * HOW ANDROID NOTIFICATIONS WORK — THE FULL PIPELINE
 * ═══════════════════════════════════════════════════════════════════
 *
 *   Your code                  Android OS
 *   ──────────────────────     ───────────────────────────────────
 *   1. createChannel()    →    NotificationManager stores channel metadata
 *                              (sound, vibration, importance) in system settings.
 *                              Users can modify these in Settings → App → Notifications.
 *
 *   2. scheduleAll()      →    AlarmManager stores the alarm in the RTC
 *                              (Real-Time Clock) alarm table in the kernel.
 *                              This survives app process death. The alarm fires
 *                              even if your app is not running.
 *
 *   3. Alarm fires        →    OS creates a new process (if needed), calls
 *                              NotificationReceiver.onReceive() on the main thread.
 *
 *   4. onReceive()        →    Your code calls NotificationManager.notify() to
 *                              post the notification and AlarmManager.setExact()
 *                              to reschedule for tomorrow.
 *
 * ═══════════════════════════════════════════════════════════════════
 * WHY ALARMMANAGER INSTEAD OF WORKMANAGER?
 * ═══════════════════════════════════════════════════════════════════
 * WorkManager is the recommended solution for deferrable background tasks
 * (e.g., sync data, compress images). It respects battery optimisations,
 * Doze mode, and may delay work by minutes or hours.
 *
 * For time-sensitive reminders (hydration at 7:00 AM sharp), we need
 * AlarmManager.setExact() which bypasses Doze mode on most devices.
 * This mirrors iOS's UNCalendarNotificationTrigger which also fires at
 * exact calendar times regardless of battery state.
 *
 * ═══════════════════════════════════════════════════════════════════
 * NOTIFICATION CHANNEL (API 26+)
 * ═══════════════════════════════════════════════════════════════════
 * Android 8.0 Oreo introduced notification channels. Every notification must
 * belong to a channel. The channel defines the default sound, vibration,
 * importance level, and whether it appears on the lock screen.
 *
 * Crucially, users can disable individual channels in system Settings without
 * disabling all of an app's notifications. This is good UX: users who don't
 * want hydration reminders can turn off just that channel.
 *
 * IMPORTANCE_DEFAULT = shows in status bar, makes sound, does NOT use heads-up
 * (floating popup). Appropriate for non-urgent reminders.
 *
 * ═══════════════════════════════════════════════════════════════════
 * PENDINGINTENT — THE KEY TO ALARM DELIVERY
 * ═══════════════════════════════════════════════════════════════════
 * A PendingIntent is a token you give to the OS that says:
 *   "When the time comes, fire this Intent on my behalf, even if I'm not running."
 *
 * It contains:
 *   - The target (our NotificationReceiver)
 *   - The Intent extras (reminder id, title, body)
 *   - A requestCode (the reminder id) — makes each alarm unique so they
 *     don't overwrite each other
 *   - FLAG_UPDATE_CURRENT — if an identical PendingIntent exists, update its
 *     extras (important for rescheduling with new data)
 *   - FLAG_IMMUTABLE — required since Android 12 / API 31 for security;
 *     the OS cannot modify the intent after creation
 */
object NotificationHelper {

    /** Channel ID — must match in every NotificationCompat.Builder call. */
    const val CHANNEL_ID = "watersync_reminders"

    /** Intent extras keys for passing reminder data to the receiver. */
    const val EXTRA_NOTIFICATION_ID = "notification_id"
    const val EXTRA_TITLE = "title"
    const val EXTRA_BODY = "body"

    /**
     * Creates the notification channel if it doesn't exist.
     * Safe to call multiple times — NotificationManager is idempotent.
     *
     * Must be called before any notification is shown. We call it at app
     * startup in MainActivity.onCreate() so the channel always exists.
     */
    fun createChannel(context: Context) {
        val channel = NotificationChannel(
            CHANNEL_ID,
            "Hydration Reminders",
            NotificationManager.IMPORTANCE_DEFAULT
        ).apply {
            description = "Daily reminders to drink water throughout the day"
        }
        context.getSystemService<NotificationManager>()
            ?.createNotificationChannel(channel)
    }

    /**
     * Schedules all 8 daily reminders as exact alarms.
     *
     * Called at app startup and after every device reboot (BootReceiver).
     * For each reminder we:
     *   1. Compute the next occurrence of H:MM today (or tomorrow if past)
     *   2. Build a PendingIntent pointing at NotificationReceiver
     *   3. Call AlarmManager.setExact() with that time and intent
     */
    fun scheduleAll(context: Context) {
        HYDRATION_REMINDERS.forEach { reminder ->
            scheduleOne(context, reminder)
        }
    }

    /**
     * Schedules a single reminder for the next occurrence of its hour.
     *
     * If it's currently 14:00 and the reminder is at 9:00, we schedule it
     * for tomorrow 9:00. If it's 8:00 and the reminder is 9:00, we schedule
     * it for today 9:00.
     *
     * This logic runs at startup and after rescheduling from NotificationReceiver.
     */
    fun scheduleOne(context: Context, reminder: HydrationReminder) {
        val alarmManager = context.getSystemService<AlarmManager>() ?: return

        // Compute the trigger time as an epoch millisecond timestamp.
        // Calendar.getInstance() starts at "now"; we then set the hour and
        // minute, zeroing out seconds and milliseconds for a clean boundary.
        val triggerTime = nextOccurrence(reminder.hour, reminder.minute)

        val intent = buildIntent(context, reminder)

        // setExact() guarantees delivery within ~1 second of the trigger time.
        // On API 23+ in Doze mode, use setExactAndAllowWhileIdle() to pierce
        // Doze — critical so the 7 AM alarm fires even if the screen has been
        // off all night.
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            alarmManager.setExactAndAllowWhileIdle(
                AlarmManager.RTC_WAKEUP,
                triggerTime,
                intent
            )
        } else {
            alarmManager.setExact(
                AlarmManager.RTC_WAKEUP,
                triggerTime,
                intent
            )
        }
    }

    /**
     * Schedules the NEXT DAY occurrence of a reminder.
     *
     * Called from NotificationReceiver after firing a notification — the
     * alarm has fired, so we schedule for 24 hours later (tomorrow same time).
     * This creates the "repeating" behaviour without using setRepeating()
     * (which is inexact and deprecated for battery reasons).
     */
    fun scheduleNextDay(context: Context, reminder: HydrationReminder) {
        val alarmManager = context.getSystemService<AlarmManager>() ?: return
        val triggerTime = nextOccurrence(reminder.hour, reminder.minute, forceNextDay = true)
        val intent = buildIntent(context, reminder)

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            alarmManager.setExactAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, triggerTime, intent)
        } else {
            alarmManager.setExact(AlarmManager.RTC_WAKEUP, triggerTime, intent)
        }
    }

    /**
     * Builds the PendingIntent for a reminder.
     *
     * The Intent carries the reminder's id, title, and body as extras.
     * NotificationReceiver reads these in onReceive() to build the notification.
     *
     * requestCode = reminder.id ensures each alarm has a unique PendingIntent.
     * Without this, AlarmManager would overwrite alarm N with alarm N+1 because
     * the OS uses (action, requestCode) to differentiate PendingIntents.
     */
    private fun buildIntent(context: Context, reminder: HydrationReminder): PendingIntent {
        val intent = Intent(context, NotificationReceiver::class.java).apply {
            action = "com.codingadventures.waternotify.SHOW_NOTIFICATION"
            putExtra(EXTRA_NOTIFICATION_ID, reminder.id)
            putExtra(EXTRA_TITLE, reminder.title)
            putExtra(EXTRA_BODY, reminder.body)
        }

        // FLAG_IMMUTABLE: required since API 31. The OS cannot modify this intent.
        // FLAG_UPDATE_CURRENT: if a PendingIntent with this requestCode already
        //   exists, update its extras with the new values (important for
        //   rescheduling tomorrow with the same title/body).
        val flags = PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE

        return PendingIntent.getBroadcast(context, reminder.id, intent, flags)
    }

    /**
     * Computes the next epoch millisecond for [hour]:[minute].
     *
     * Logic:
     *   - Start with today's date at the given H:M:00.000
     *   - If that time is already in the past (or forceNextDay=true), add 1 day
     *   - Return the resulting timestamp
     *
     * Example (today is 2025-03-29 14:00):
     *   nextOccurrence(9, 0)  → 2025-03-30 09:00:00 (past → tomorrow)
     *   nextOccurrence(15, 0) → 2025-03-29 15:00:00 (future → today)
     *   nextOccurrence(9, 0, forceNextDay=true) → 2025-03-30 09:00:00
     */
    private fun nextOccurrence(hour: Int, minute: Int, forceNextDay: Boolean = false): Long {
        val cal = Calendar.getInstance().apply {
            set(Calendar.HOUR_OF_DAY, hour)
            set(Calendar.MINUTE, minute)
            set(Calendar.SECOND, 0)
            set(Calendar.MILLISECOND, 0)
        }
        val now = System.currentTimeMillis()
        if (forceNextDay || cal.timeInMillis <= now) {
            cal.add(Calendar.DAY_OF_YEAR, 1)
        }
        return cal.timeInMillis
    }
}
