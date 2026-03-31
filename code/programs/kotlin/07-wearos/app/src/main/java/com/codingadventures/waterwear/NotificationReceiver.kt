package com.codingadventures.waterwear

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

/**
 * NotificationReceiver — BroadcastReceiver that fires when an AlarmManager alarm triggers.
 *
 * BROADCAST RECEIVER LIFECYCLE:
 *   A BroadcastReceiver is instantiated, has onReceive() called, and is then
 *   garbage-collected — all within a few hundred milliseconds. There is no
 *   persistent state. Everything this receiver needs must arrive via the Intent
 *   extras (title, body, hour) set by NotificationHelper.
 *
 * SELF-RESCHEDULING PATTERN:
 *   AlarmManager alarms fire ONCE. To get a daily reminder, each alarm must
 *   reschedule itself for the same hour tomorrow. This receiver:
 *     1. Shows the notification.
 *     2. Creates a new alarm for tomorrow at the same hour.
 *
 *   This is simpler than using setRepeating() because WearOS can cancel
 *   repeating alarms during battery optimisation. One-shot + self-reschedule
 *   is more resilient (and is the pattern recommended by Google for exact alarms).
 *
 * NOTIFICATION PERMISSION CHECK:
 *   On API 33+ (WearOS 4+), POST_NOTIFICATIONS requires a runtime grant.
 *   We check the permission before calling notify(). If denied, we silently
 *   skip the notification (the alarm still reschedules for next time).
 *
 * WEAROS-SPECIFIC CONSIDERATION:
 *   WearOS notifications appear as notification tiles that the user swipes to
 *   from the main watch face. The notification includes a content intent that
 *   opens MainActivity when tapped — letting the user log a drink immediately.
 */
class NotificationReceiver : BroadcastReceiver() {

    override fun onReceive(context: Context, intent: Intent) {
        // Extract the notification content from the Intent extras.
        // These were set by NotificationHelper.scheduleOne().
        val tag   = intent.getStringExtra("tag")   ?: return
        val title = intent.getStringExtra("title") ?: return
        val body  = intent.getStringExtra("body")  ?: return
        val hour  = intent.getIntExtra("hour", -1)

        // ── Show the notification ─────────────────────────────────────────────
        showNotification(context, tag, title, body)

        // ── Reschedule for tomorrow ───────────────────────────────────────────
        // Find the matching NotificationItem by tag and reschedule.
        // This ensures the alarm fires again at the same hour tomorrow.
        if (hour >= 0) {
            val item = NOTIFICATION_SCHEDULE.find { it.tag == tag }
            if (item != null) {
                // Re-use NotificationHelper.scheduleAll() would reschedule ALL 8.
                // Instead, re-schedule this single item by creating a new
                // NotificationItem with the same data and delegating to the helper.
                // We call scheduleAll() for simplicity — idempotent due to
                // FLAG_UPDATE_CURRENT on each PendingIntent.
                NotificationHelper.scheduleAll(context)
            }
        }
    }

    /**
     * Builds and shows the notification.
     *
     * NOTIFICATIONCOMPAT:
     *   NotificationCompat.Builder works on all API levels (it back-ports features
     *   to older APIs). We always use it instead of the platform Notification.Builder.
     *
     * CONTENT INTENT:
     *   Tapping the notification on the watch opens MainActivity.
     *   This is important for WearOS UX — the user sees the reminder, taps it,
     *   and is immediately in the app to log a drink.
     *
     * SMALL ICON:
     *   Android requires a small icon for notifications. We reuse the app launcher
     *   icon here. In a production app you'd use a dedicated monochrome icon.
     *   Note: ic_launcher is typically not a valid notification icon (must be
     *   monochrome); for this learning project it compiles fine.
     */
    private fun showNotification(
        context: Context,
        tag: String,
        title: String,
        body: String
    ) {
        // Check POST_NOTIFICATIONS permission on API 33+.
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            val granted = ContextCompat.checkSelfPermission(
                context,
                Manifest.permission.POST_NOTIFICATIONS
            ) == PackageManager.PERMISSION_GRANTED
            if (!granted) return
        }

        // Tapping the notification opens MainActivity.
        val openAppIntent = Intent(context, MainActivity::class.java)
        val contentPendingIntent = PendingIntent.getActivity(
            context,
            tag.hashCode(),
            openAppIntent,
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        )

        val notification = NotificationCompat.Builder(context, "waterwear_reminders")
            .setSmallIcon(R.mipmap.ic_launcher)
            .setContentTitle(title)
            .setContentText(body)
            .setStyle(NotificationCompat.BigTextStyle().bigText(body))
            .setContentIntent(contentPendingIntent)
            // AUTO_CANCEL: dismiss the notification when the user taps it.
            .setAutoCancel(true)
            // Priority for pre-channel devices (API < 26). On API 26+ the
            // channel's importance setting overrides this.
            .setPriority(NotificationCompat.PRIORITY_DEFAULT)
            .build()

        val manager = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        // Use tag.hashCode() as the notification ID.
        // Using the same ID for the same slot ensures a new "morning" notification
        // replaces the previous one instead of stacking 30 identical tiles.
        manager.notify(tag.hashCode(), notification)
    }
}
