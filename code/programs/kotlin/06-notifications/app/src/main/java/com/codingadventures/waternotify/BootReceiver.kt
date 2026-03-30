package com.codingadventures.waternotify

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent

/**
 * BootReceiver — reschedules all daily alarms after device reboot.
 *
 * ═══════════════════════════════════════════════════════════════════
 * WHY ALARMS DON'T SURVIVE REBOOTS
 * ═══════════════════════════════════════════════════════════════════
 * AlarmManager stores pending alarms in memory (in the alarm_manager system
 * service process). When the device powers off, that memory is lost. On boot,
 * the alarm service starts fresh with an empty alarm table.
 *
 * This means: if you schedule 8 alarms and the user reboots their phone,
 * ALL 8 alarms are silently dropped. The user would never receive another
 * hydration reminder until they opened the app again.
 *
 * The fix: register a BroadcastReceiver for android.intent.action.BOOT_COMPLETED.
 * The OS sends this broadcast to all apps ~30-60 seconds after boot finishes.
 * We use it as a trigger to reschedule all alarms.
 *
 * ═══════════════════════════════════════════════════════════════════
 * MANIFEST REQUIREMENTS
 * ═══════════════════════════════════════════════════════════════════
 * Two things must be in AndroidManifest.xml for this to work:
 *
 *   1. Permission:
 *      <uses-permission android:name="android.permission.RECEIVE_BOOT_COMPLETED" />
 *      Without this, the OS won't deliver BOOT_COMPLETED to our app.
 *
 *   2. Receiver declaration with exported=true:
 *      <receiver android:name=".BootReceiver" android:exported="true">
 *          <intent-filter>
 *              <action android:name="android.intent.action.BOOT_COMPLETED" />
 *          </intent-filter>
 *      </receiver>
 *      exported=true is required because the sender (the Android OS) is a
 *      different process. If exported=false, only our own app could trigger
 *      this receiver.
 *
 * ═══════════════════════════════════════════════════════════════════
 * STOPPED STATE CAVEAT
 * ═══════════════════════════════════════════════════════════════════
 * Since Android 3.1, apps are in a "stopped state" immediately after
 * installation. In stopped state, broadcast receivers (including boot receivers)
 * are NOT invoked. The app leaves stopped state the first time the user
 * manually opens it.
 *
 * This means: on a fresh install + reboot (without ever opening the app),
 * BootReceiver won't run. This is acceptable — the user hasn't set up the
 * app yet anyway. Once they open the app, MainActivity schedules all alarms,
 * and BootReceiver will handle all future reboots.
 *
 * ═══════════════════════════════════════════════════════════════════
 * COMPARISON WITH IOS
 * ═══════════════════════════════════════════════════════════════════
 * iOS's UNCalendarNotificationTrigger(repeats: true) is persistent across
 * reboots — iOS handles this transparently. Android requires us to do it
 * manually. BootReceiver + RECEIVE_BOOT_COMPLETED is the standard pattern
 * that all Android apps with repeating alarms must implement.
 */
class BootReceiver : BroadcastReceiver() {

    override fun onReceive(context: Context, intent: Intent) {
        // Guard: only respond to boot completed, not other intents.
        // This is defensive programming — the manifest should ensure only
        // BOOT_COMPLETED reaches us, but an explicit check is safer.
        if (intent.action != Intent.ACTION_BOOT_COMPLETED) return

        // Reschedule all 8 daily hydration reminders.
        // NotificationHelper.scheduleAll() computes the next occurrence for each
        // reminder (today if in the future, tomorrow if already past) and sets
        // an exact alarm. The self-rescheduling chain in NotificationReceiver
        // keeps the alarms going indefinitely from this point forward.
        NotificationHelper.scheduleAll(context)
    }
}
