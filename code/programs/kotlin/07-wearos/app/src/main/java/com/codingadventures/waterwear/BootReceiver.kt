package com.codingadventures.waterwear

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent

/**
 * BootReceiver — reschedules alarms after the watch reboots.
 *
 * WHY IS THIS NECESSARY?
 *   AlarmManager stores alarms in volatile memory. When the watch shuts down
 *   (to swap the band, for a software update, or because the battery died),
 *   ALL pending alarms are erased. Without a BootReceiver, the user would get
 *   zero notifications after any reboot until they manually reopened the app.
 *
 * HOW IT WORKS:
 *   1. The system broadcasts android.intent.action.BOOT_COMPLETED a few seconds
 *      after the watch finishes booting.
 *   2. Android matches the broadcast to this receiver (declared in AndroidManifest.xml).
 *   3. onReceive() calls NotificationHelper.scheduleAll(), which registers all
 *      8 daily alarms as if the app had just been opened fresh.
 *
 * MANIFEST DECLARATION:
 *   The receiver must declare an <intent-filter> for BOOT_COMPLETED in the manifest.
 *   It also requires the RECEIVE_BOOT_COMPLETED permission in the manifest.
 *   Without both, Android will not deliver the broadcast to this class.
 *
 * WEAROS BOOT TIMING:
 *   WearOS watches typically boot in 30-60 seconds. The BOOT_COMPLETED broadcast
 *   fires after the system is fully ready, including AlarmManager being available.
 *   It is safe to call scheduleAll() here without any delay.
 *
 * QUICK IMPLEMENTATION NOTE:
 *   This receiver does exactly one thing — call scheduleAll(). The simplicity
 *   is intentional: all scheduling logic lives in NotificationHelper.
 *   BootReceiver is just the hook that calls it at the right time.
 */
class BootReceiver : BroadcastReceiver() {

    override fun onReceive(context: Context, intent: Intent) {
        // Only act on BOOT_COMPLETED — ignore any other intents delivered here.
        if (intent.action == Intent.ACTION_BOOT_COMPLETED) {
            NotificationHelper.scheduleAll(context)
        }
    }
}
