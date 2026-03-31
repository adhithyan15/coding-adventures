// NotificationManager.swift
//
// Owns the full UNUserNotificationCenter lifecycle:
//   1. Request authorisation on first launch
//   2. Check current status on subsequent launches
//   3. Schedule the static daily reminder set
//   4. Deliver notifications even when the app is in the foreground
//
// TESTABILITY:
//   UNUserNotificationCenter.current() cannot be instantiated in unit tests
//   without a running app host. We inject a NotificationScheduling protocol
//   so tests can substitute MockNotificationCenter. The real centre satisfies
//   the protocol via a conformance extension — no wrapper class needed.
//
// IDEMPOTENT SCHEDULING:
//   scheduleAll() always calls removeAll() first. This means it is safe to
//   call on every app launch. Stable notification identifiers ensure the
//   system atomically replaces any existing request with the same ID.
//
// WALL-CLOCK TRIGGERS:
//   UNCalendarNotificationTrigger(dateMatching:repeats:) fires at a fixed
//   time every day regardless of when the app launches. This is correct for
//   a health app with a science-backed schedule. Do NOT use
//   UNTimeIntervalNotificationTrigger — it drifts relative to app launches.

import Foundation
import UserNotifications

// ── Protocol for dependency injection ─────────────────────────────────────────

/// The minimum surface of UNUserNotificationCenter that we use.
/// UNUserNotificationCenter satisfies this via the extension below.
/// Tests inject MockNotificationCenter instead.
protocol NotificationScheduling: AnyObject {
    func requestAuthorization(
        options: UNAuthorizationOptions,
        completionHandler: @escaping (Bool, Error?) -> Void
    )
    func add(
        _ request: UNNotificationRequest,
        withCompletionHandler completionHandler: ((Error?) -> Void)?
    )
    func removeAllPendingNotificationRequests()
    func getNotificationSettings(
        completionHandler: @escaping (UNNotificationSettings) -> Void
    )
}

// UNUserNotificationCenter already has all these methods — the empty
// extension just tells the compiler it conforms to our protocol.
extension UNUserNotificationCenter: NotificationScheduling {}

// ── Manager ───────────────────────────────────────────────────────────────────

@Observable
final class NotificationManager: NSObject {

    static let shared = NotificationManager()

    /// Current authorisation status. Observed by ContentView to show the
    /// "Enable in Settings" banner when the user has denied permission.
    private(set) var authStatus: UNAuthorizationStatus = .notDetermined

    /// The next scheduled notification fire time, computed from the static
    /// schedule relative to now. Used for the "Next reminder in Xh Ym" label.
    private(set) var nextReminderTime: Date?

    // The injected centre — real in production, mock in tests.
    // Private so callers always go through the public API.
    private var center: NotificationScheduling = UNUserNotificationCenter.current()

    // ── Setup ─────────────────────────────────────────────────────────────

    /// Called from WaterNotifyApp.init() before the first scene renders.
    ///
    /// Sets self as the UNUserNotificationCenterDelegate (so foreground
    /// notifications display as banners), then checks authorisation status:
    ///   • .notDetermined → show system permission dialog
    ///   • .authorized    → reschedule (idempotent, safe every launch)
    ///   • .denied        → update authStatus so UI can show Settings prompt
    func setup(center: NotificationScheduling = UNUserNotificationCenter.current()) {
        self.center = center

        // Set delegate on the real centre so foreground delivery works.
        // Guard: only set the delegate when using the production centre, not
        // when a mock is injected for testing. Avoids silently overwriting a
        // delegate registered by another component in a shared process.
        if center is UNUserNotificationCenter {
            UNUserNotificationCenter.current().delegate = self
        }

        // Check current status rather than blindly requesting auth every time.
        // requestAuthorization on a device where the user already tapped "Don't
        // Allow" is a no-op — it does not re-show the dialog.
        center.getNotificationSettings { [weak self] settings in
            DispatchQueue.main.async {
                self?.authStatus = settings.authorizationStatus
                switch settings.authorizationStatus {
                case .notDetermined:
                    self?.requestAuthorization()
                case .authorized, .provisional:
                    self?.scheduleAll()
                    self?.updateNextReminderTime()
                case .denied:
                    break   // ContentView shows a "go to Settings" banner
                @unknown default:
                    break
                }
            }
        }
    }

    // ── Authorisation ─────────────────────────────────────────────────────

    /// Shows the system permission dialog (first launch only).
    /// We request .alert and .sound — no badge (badges feel accusatory
    /// for a health app that is meant to encourage, not guilt).
    private func requestAuthorization() {
        center.requestAuthorization(options: [.alert, .sound]) { [weak self] granted, _ in
            DispatchQueue.main.async {
                self?.authStatus = granted ? .authorized : .denied
                if granted {
                    self?.scheduleAll()
                    self?.updateNextReminderTime()
                }
            }
        }
    }

    // ── Scheduling ────────────────────────────────────────────────────────

    /// Cancels all pending notifications then schedules the static daily set.
    ///
    /// Idempotent: safe to call on every app launch. The removeAll() ensures
    /// stale requests from previous versions of the schedule don't linger.
    /// The stable identifiers ensure the system replaces any existing request
    /// with the same ID atomically.
    ///
    /// We consume 8 of the 64-notification system limit. repeats: true means
    /// each calendar trigger is stored once (not once per future fire date),
    /// so we stay far under the limit.
    ///
    /// The `using` parameter is for dependency injection in unit tests.
    /// Production code always calls `scheduleAll()` (no argument) which uses
    /// the real `UNUserNotificationCenter`.
    func scheduleAll(using overrideCenter: NotificationScheduling? = nil) {
        let target = overrideCenter ?? center
        target.removeAllPendingNotificationRequests()

        for item in NotificationSchedule.all {
            let content       = UNMutableNotificationContent()
            content.title     = item.title
            content.body      = item.body
            content.sound     = .default
            // No content.badge — see module header

            // DateComponents with only hour set fires at that hour every day
            // in the device's local timezone. repeats: true = daily forever.
            var components    = DateComponents()
            components.hour   = item.hour
            components.minute = 0

            let trigger = UNCalendarNotificationTrigger(
                dateMatching: components,
                repeats: true
            )
            let request = UNNotificationRequest(
                identifier: item.id,
                content: content,
                trigger: trigger
            )
            target.add(request, withCompletionHandler: nil)
        }
    }

    // ── Next reminder time ────────────────────────────────────────────────

    /// Scans the static schedule to find the next notification time after now.
    /// Used by ContentView to show "Next reminder in Xh Ym".
    func updateNextReminderTime() {
        let calendar  = Calendar.current
        let now       = Date()
        let nowHour   = calendar.component(.hour, from: now)
        let nowMinute = calendar.component(.minute, from: now)

        // Find the next hour in the schedule that is after the current time.
        for item in NotificationSchedule.all {
            if item.hour > nowHour || (item.hour == nowHour && nowMinute == 0) {
                var components    = calendar.dateComponents([.year, .month, .day], from: now)
                components.hour   = item.hour
                components.minute = 0
                nextReminderTime  = calendar.date(from: components)
                return
            }
        }

        // All of today's reminders have passed — next is tomorrow's first slot.
        if let tomorrow = calendar.date(byAdding: .day, value: 1, to: now) {
            var components    = calendar.dateComponents([.year, .month, .day], from: tomorrow)
            components.hour   = NotificationSchedule.all.first?.hour ?? 7
            components.minute = 0
            nextReminderTime  = calendar.date(from: components)
        }
    }
}

// ── UNUserNotificationCenterDelegate ─────────────────────────────────────────

extension NotificationManager: UNUserNotificationCenterDelegate {

    /// Called when a notification fires while the app is in the foreground.
    ///
    /// Default iOS behaviour: suppress the banner when app is open.
    /// We override to show banner + play sound anyway — if the user has the
    /// app open, they still benefit from seeing the reminder cue.
    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        willPresent notification: UNNotification,
        withCompletionHandler completionHandler: @escaping (UNNotificationPresentationOptions) -> Void
    ) {
        completionHandler([.banner, .sound])
    }
}
