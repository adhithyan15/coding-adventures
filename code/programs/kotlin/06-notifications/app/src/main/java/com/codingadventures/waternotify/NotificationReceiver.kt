package com.codingadventures.waternotify

import android.Manifest
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
import androidx.core.content.getSystemService

/**
 * NotificationReceiver — BroadcastReceiver that fires when an alarm goes off.
 *
 * This is the "delivery" half of the notification system:
 *   - NotificationHelper.kt is the "setup" half (creates channel, schedules alarms)
 *   - NotificationReceiver.kt is the "delivery" half (shows the notification,
 *     reschedules for tomorrow)
 *
 * ═══════════════════════════════════════════════════════════════════
 * HOW BROADCASTRECEIVERS WORK
 * ═══════════════════════════════════════════════════════════════════
 * A BroadcastReceiver is Android's event bus subscriber. When the OS (or another
 * app with permission) sends a broadcast Intent, the OS wakes up all registered
 * receivers that match the Intent's action.
 *
 * Key constraints:
 *   1. onReceive() runs on the MAIN THREAD. You have ~10 seconds before ANR.
 *   2. The process may be killed immediately after onReceive() returns.
 *      You cannot start a coroutine here and expect it to complete.
 *   3. For anything requiring async work (network, DB), use goAsync() to get
 *      a PendingResult — or better, start a foreground Service.
 *
 * For our use case, NotificationManager.notify() is synchronous and fast
 * (~1ms), so plain onReceive() is fine. No async work needed.
 *
 * ═══════════════════════════════════════════════════════════════════
 * SELF-RESCHEDULING PATTERN
 * ═══════════════════════════════════════════════════════════════════
 * AlarmManager.setExact() fires ONCE. To get a daily alarm we reschedule
 * from inside the receiver:
 *
 *   [AlarmManager] → fires once → [NotificationReceiver.onReceive()]
 *                                         |
 *                                         ↓
 *                                  show notification
 *                                         |
 *                                         ↓
 *                                  schedule tomorrow's alarm
 *                                         |
 *                                         ↓
 *                          [AlarmManager] → fires again tomorrow
 *
 * This self-rescheduling chain continues indefinitely. The only way to stop
 * it is to cancel the PendingIntent via AlarmManager.cancel() (e.g., if the
 * user turns off notifications in app settings).
 *
 * This mirrors iOS UNCalendarNotificationTrigger(repeats: true) which iOS
 * handles automatically, but on Android we implement manually.
 *
 * ═══════════════════════════════════════════════════════════════════
 * TAP ACTION — OPENING THE APP
 * ═══════════════════════════════════════════════════════════════════
 * When the user taps the notification, we open MainActivity. This requires
 * a PendingIntent attached to the notification via .setContentIntent().
 * FLAG_IMMUTABLE and FLAG_UPDATE_CURRENT apply here too.
 */
class NotificationReceiver : BroadcastReceiver() {

    override fun onReceive(context: Context, intent: Intent) {
        // Extract the reminder data that NotificationHelper baked into the alarm intent.
        val notificationId = intent.getIntExtra(NotificationHelper.EXTRA_NOTIFICATION_ID, 0)
        val title = intent.getStringExtra(NotificationHelper.EXTRA_TITLE) ?: return
        val body = intent.getStringExtra(NotificationHelper.EXTRA_BODY) ?: return

        // On Android 13+ (API 33+), POST_NOTIFICATIONS is a runtime permission.
        // If the user denied it, we silently skip (no crash — just no notification).
        // The permission was requested at startup in MainActivity.
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            val granted = ContextCompat.checkSelfPermission(
                context,
                Manifest.permission.POST_NOTIFICATIONS
            ) == PackageManager.PERMISSION_GRANTED
            if (!granted) {
                // Re-schedule tomorrow even if we can't show today's notification.
                // The user might grant permission later and we don't want to lose
                // the alarm chain.
                reschedule(context, notificationId)
                return
            }
        }

        // Build the tap intent — opens MainActivity when user taps the notification.
        val tapIntent = Intent(context, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP
        }
        val tapPendingIntent = PendingIntent.getActivity(
            context,
            notificationId,
            tapIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        // Build the notification using NotificationCompat for backward compatibility.
        // NotificationCompat handles API differences automatically — the same code
        // works on API 26 (Oreo) and API 35 (Android 15).
        //
        // android.R.drawable.ic_dialog_info is a system icon available on all
        // Android versions. In a production app you'd use a custom water-drop icon.
        val notification = NotificationCompat.Builder(context, NotificationHelper.CHANNEL_ID)
            .setSmallIcon(android.R.drawable.ic_dialog_info)
            .setContentTitle(title)
            .setContentText(body)
            // BigTextStyle expands the notification to show the full body text.
            // Without this, long body text is truncated to ~1 line.
            .setStyle(NotificationCompat.BigTextStyle().bigText(body))
            .setContentIntent(tapPendingIntent)
            // AUTO_CANCEL = true: notification dismisses itself when tapped.
            // Without this, the notification stays in the drawer after the user opens the app.
            .setAutoCancel(true)
            // PRIORITY_DEFAULT maps to the channel's importance on API 26+.
            // On API 25 and below (pre-channel), this directly controls behaviour.
            .setPriority(NotificationCompat.PRIORITY_DEFAULT)
            .build()

        context.getSystemService<NotificationManager>()?.notify(notificationId, notification)

        // Reschedule for tomorrow — keeps the daily chain alive.
        reschedule(context, notificationId)
    }

    /**
     * Finds the reminder by id and schedules it for the next day.
     *
     * HYDRATION_REMINDERS is a constant list; finding by id is O(8) = O(1) in practice.
     * If the id doesn't match (should never happen), we silently skip — better than
     * crashing the receiver.
     */
    private fun reschedule(context: Context, notificationId: Int) {
        val reminder = HYDRATION_REMINDERS.find { it.id == notificationId } ?: return
        NotificationHelper.scheduleNextDay(context, reminder)
    }
}
