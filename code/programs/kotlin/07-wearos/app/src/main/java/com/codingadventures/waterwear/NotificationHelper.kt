package com.codingadventures.waterwear

import android.app.AlarmManager
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.os.Build
import java.util.Calendar

/**
 * NotificationHelper — creates the notification channel and schedules alarms.
 *
 * TWO RESPONSIBILITIES:
 *   1. createChannel(): must be called once on app start to register the
 *      notification channel with Android. Without a channel, no notifications
 *      can be shown on API 26+ (Android 8 / WearOS 3+).
 *
 *   2. scheduleAll(): iterates NOTIFICATION_SCHEDULE and uses AlarmManager
 *      to fire an Intent at each target hour. The Intent triggers
 *      NotificationReceiver, which shows the notification.
 *
 * ALARMMANAGER APPROACH (vs WorkManager):
 *   WorkManager is the modern recommendation for background tasks, but it has
 *   ~15-minute minimum intervals and flexible timing. For exact-hour notifications
 *   (e.g., exactly 07:00), AlarmManager with setExactAndAllowWhileIdle() is more
 *   reliable on WearOS, which aggressively sleeps to conserve battery.
 *
 * EXACT ALARM PERMISSION (API 31+):
 *   setExactAndAllowWhileIdle() requires SCHEDULE_EXACT_ALARM permission on
 *   Android 12+ (API 31+). We check canScheduleExactAlarms() at runtime and
 *   fall back to setAndAllowWhileIdle() (inexact, ±minutes) if denied.
 *   The user can grant the permission in Settings > Apps > WaterWear > Alarms.
 *
 * WEAROS AMBIENT MODE:
 *   Watches spend most of their time in ambient (low-power display) mode.
 *   setExactAndAllowWhileIdle() fires even during Doze mode, which is equivalent
 *   to ambient on WearOS. The WAKE_LOCK permission ensures the CPU wakes briefly
 *   to execute the BroadcastReceiver before sleeping again.
 */
object NotificationHelper {

    /** The notification channel ID — must match strings.xml. */
    private const val CHANNEL_ID = "waterwear_reminders"

    /**
     * Creates the notification channel.
     *
     * NOTIFICATION CHANNELS (API 26+):
     *   Android 8 introduced notification channels as a user-control mechanism.
     *   Each channel has its own settings (sound, vibration, importance) that
     *   the user can override in Settings. The channel only needs to be created
     *   once — subsequent calls with the same ID are no-ops.
     *
     * IMPORTANCE LEVELS:
     *   IMPORTANCE_DEFAULT: shows in the shade with sound and heads-up on the watch.
     *   IMPORTANCE_LOW: silent, no heads-up (bad for health reminders).
     *   IMPORTANCE_HIGH: persistent heads-up — too aggressive for reminders.
     */
    fun createChannel(context: Context) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                context.getString(R.string.notification_channel_name),
                NotificationManager.IMPORTANCE_DEFAULT
            ).apply {
                description = context.getString(R.string.notification_channel_description)
            }
            val manager = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
            manager.createNotificationChannel(channel)
        }
    }

    /**
     * Schedules all 8 daily hydration reminders using AlarmManager.
     *
     * For each NotificationItem in NOTIFICATION_SCHEDULE:
     *   1. Compute the next occurrence of item.hour:00:00 local time.
     *   2. Wrap a NotificationReceiver Intent in a PendingIntent.
     *   3. Register the alarm with AlarmManager.
     *
     * CALL SITES:
     *   - MainActivity.onCreate() → first-time setup
     *   - BootReceiver.onReceive() → after watch restarts (alarms are erased on boot)
     */
    fun scheduleAll(context: Context) {
        val alarmManager = context.getSystemService(Context.ALARM_SERVICE) as AlarmManager
        for (item in NOTIFICATION_SCHEDULE) {
            scheduleOne(context, alarmManager, item)
        }
    }

    /**
     * Schedules a single notification alarm.
     *
     * NEXT-OCCURRENCE LOGIC:
     *   We always schedule for TODAY at item.hour. If that time has already passed
     *   (i.e., it's past 7am and we're scheduling the morning alarm), we add one
     *   day so it fires TOMORROW morning — never in the past.
     *
     * PENDING INTENT EXTRAS:
     *   The Intent carries the notification title and body as string extras.
     *   NotificationReceiver reads these to build the notification without needing
     *   to look up which slot fired — any receiver instance can handle any slot.
     *
     * REQUEST CODE:
     *   Each alarm must have a unique request code, otherwise a new PendingIntent
     *   with the same request code REPLACES the previous alarm rather than adding
     *   a second one. We use tag.hashCode() which is stable and unique per tag.
     *
     * FLAG_IMMUTABLE:
     *   Required on API 23+ for security. Prevents other apps from modifying
     *   the PendingIntent's extras after it is created.
     */
    private fun scheduleOne(
        context: Context,
        alarmManager: AlarmManager,
        item: NotificationItem
    ) {
        val cal = Calendar.getInstance().apply {
            set(Calendar.HOUR_OF_DAY, item.hour)
            set(Calendar.MINUTE, 0)
            set(Calendar.SECOND, 0)
            set(Calendar.MILLISECOND, 0)
            // If the target time is in the past today, schedule for tomorrow.
            if (timeInMillis <= System.currentTimeMillis()) {
                add(Calendar.DAY_OF_YEAR, 1)
            }
        }

        // Build the Intent that NotificationReceiver will receive.
        // Extras carry the notification content so the receiver is stateless.
        val intent = Intent(context, NotificationReceiver::class.java).apply {
            putExtra("tag", item.tag)
            putExtra("title", item.title)
            putExtra("body", item.body)
            putExtra("hour", item.hour)
        }

        // PendingIntent wraps the Intent so AlarmManager can fire it later.
        // FLAG_IMMUTABLE: the Intent extras cannot be changed after creation.
        // FLAG_UPDATE_CURRENT: if an identical PendingIntent exists (same request
        //   code), update its extras rather than failing silently.
        val pendingIntent = PendingIntent.getBroadcast(
            context,
            item.tag.hashCode(),   // unique request code per notification slot
            intent,
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        )

        // Schedule the alarm.
        // setExactAndAllowWhileIdle() fires at the exact time even in Doze mode.
        // On API 31+ (Android 12 / WearOS 4), this requires SCHEDULE_EXACT_ALARM.
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            if (alarmManager.canScheduleExactAlarms()) {
                alarmManager.setExactAndAllowWhileIdle(
                    AlarmManager.RTC_WAKEUP,
                    cal.timeInMillis,
                    pendingIntent
                )
            } else {
                // Fallback: inexact alarm (±minutes). Acceptable but not ideal.
                // The user can grant SCHEDULE_EXACT_ALARM in Settings > Apps.
                alarmManager.setAndAllowWhileIdle(
                    AlarmManager.RTC_WAKEUP,
                    cal.timeInMillis,
                    pendingIntent
                )
            }
        } else {
            // API 30 (WearOS 3): setExactAndAllowWhileIdle() does not require
            // the SCHEDULE_EXACT_ALARM permission — use it directly.
            alarmManager.setExactAndAllowWhileIdle(
                AlarmManager.RTC_WAKEUP,
                cal.timeInMillis,
                pendingIntent
            )
        }
    }
}
