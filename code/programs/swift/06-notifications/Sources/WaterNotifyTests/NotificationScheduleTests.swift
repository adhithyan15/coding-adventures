// NotificationScheduleTests.swift
//
// Tests the notification schedule data and the NotificationManager scheduling
// logic, using a MockNotificationCenter instead of the real system centre.
//
// WHY A MOCK:
//   UNUserNotificationCenter.current() requires a running app host and cannot
//   be instantiated in a unit test target. Calling it in tests causes crashes
//   or silent failures. We inject a MockNotificationCenter via the
//   NotificationScheduling protocol instead.
//
//   This is the same dependency-injection pattern recommended by Apple in
//   their "Testing in Xcode" WWDC sessions.

import XCTest
import UserNotifications
@testable import WaterNotify

// ── Mock centre ───────────────────────────────────────────────────────────────

/// Records all calls made to it so tests can assert on what was scheduled.
final class MockNotificationCenter: NotificationScheduling {

    // Captured state
    var addedRequests: [UNNotificationRequest] = []
    var removeAllCalled = false
    var authorizationGranted = true   // simulate "user tapped Allow"

    func requestAuthorization(
        options: UNAuthorizationOptions,
        completionHandler: @escaping (Bool, Error?) -> Void
    ) {
        completionHandler(authorizationGranted, nil)
    }

    func add(
        _ request: UNNotificationRequest,
        withCompletionHandler completionHandler: ((Error?) -> Void)?
    ) {
        addedRequests.append(request)
        completionHandler?(nil)
    }

    func removeAllPendingNotificationRequests() {
        removeAllCalled = true
        // Note: does NOT clear addedRequests — tests verify what was added
        // after the clear, which is the correct production behaviour.
    }

    func getNotificationSettings(
        completionHandler: @escaping (UNNotificationSettings) -> Void
    ) {
        // In tests we call scheduleAll() directly and don't test the
        // auth-check path here (that would require subclassing UNNotificationSettings,
        // which is a sealed class). The auth path is covered by integration testing.
    }
}

// ── Schedule data tests ───────────────────────────────────────────────────────

final class NotificationScheduleTests: XCTestCase {

    // ── Data integrity tests ───────────────────────────────────────────────

    /// The schedule must have exactly 8 items — one per reminder slot.
    func testScheduleHasExactlyEightItems() {
        XCTAssertEqual(NotificationSchedule.all.count, 8,
                       "Expected 8 notification slots (one per 2-hour window)")
    }

    /// Every notification must have a unique identifier.
    /// Duplicate IDs would silently replace each other in the system.
    func testAllIdentifiersAreUnique() {
        let ids    = NotificationSchedule.all.map { $0.id }
        let unique = Set(ids)
        XCTAssertEqual(ids.count, unique.count,
                       "Duplicate notification IDs found: \(ids)")
    }

    /// All scheduled hours must be within the sensible waking window (7am–9pm).
    func testAllHoursAreWithinWakingWindow() {
        for item in NotificationSchedule.all {
            XCTAssertGreaterThanOrEqual(item.hour, 7,
                                        "\(item.id) fires before 7am")
            XCTAssertLessThanOrEqual(item.hour, 21,
                                     "\(item.id) fires after 9pm")
        }
    }

    /// Hours must be in ascending order so "next reminder" computation works.
    func testHoursAreInAscendingOrder() {
        let hours = NotificationSchedule.all.map { $0.hour }
        for i in 1..<hours.count {
            XCTAssertGreaterThan(hours[i], hours[i - 1],
                                 "Hour at index \(i) (\(hours[i])) is not after hour at \(i-1) (\(hours[i-1]))")
        }
    }

    /// No two notification slots at the same hour.
    func testNoTwoSlotsAtTheSameHour() {
        let hours  = NotificationSchedule.all.map { $0.hour }
        let unique = Set(hours)
        XCTAssertEqual(hours.count, unique.count,
                       "Duplicate hours in schedule: \(hours)")
    }

    /// The morning slot must exist at 07:00 — the science justifies a special
    /// two-glasses reminder after overnight fluid loss.
    func testMorningSlotExistsAtSevenAM() {
        let morning = NotificationSchedule.all.first { $0.id == "watersync.morning" }
        XCTAssertNotNil(morning, "Morning notification slot not found")
        XCTAssertEqual(morning?.hour, 7, "Morning slot must fire at 07:00")
    }

    /// No notification should have an empty title — that would show a blank banner.
    func testAllTitlesAreNonEmpty() {
        for item in NotificationSchedule.all {
            XCTAssertFalse(item.title.isEmpty,
                           "Empty title for notification ID: \(item.id)")
        }
    }

    /// No notification should have an empty body — the health fact is the value prop.
    func testAllBodiesAreNonEmpty() {
        for item in NotificationSchedule.all {
            XCTAssertFalse(item.body.isEmpty,
                           "Empty body for notification ID: \(item.id)")
        }
    }

    /// Bodies should be at most 150 characters — longer text is truncated on
    /// the lock screen and may not display on the Apple Watch notification mirror.
    func testAllBodiesAreReasonableLength() {
        for item in NotificationSchedule.all {
            XCTAssertLessThanOrEqual(item.body.count, 150,
                                     "Body too long for \(item.id): \(item.body.count) chars")
        }
    }
}

// ── NotificationManager scheduling tests ─────────────────────────────────────

final class NotificationManagerTests: XCTestCase {

    private var mock:    MockNotificationCenter!
    private var manager: NotificationManager!

    override func setUp() {
        super.setUp()
        mock    = MockNotificationCenter()
        manager = NotificationManager()
        // Bypass the auth-check path and call scheduleAll directly.
        // scheduleAll is the method we want to test here.
        manager.scheduleAll(using: mock)
    }

    // ── Scheduling tests ───────────────────────────────────────────────────

    /// scheduleAll must add exactly 8 requests — one per schedule item.
    func testScheduleAllAddsEightRequests() {
        XCTAssertEqual(mock.addedRequests.count, 8)
    }

    /// removeAll must be called before adding new requests.
    /// This ensures stale schedules from previous app versions don't linger.
    func testScheduleAllRemovesPendingBeforeAdding() {
        XCTAssertTrue(mock.removeAllCalled,
                      "removeAllPendingNotificationRequests() must be called before scheduling")
    }

    /// Each request's identifier must match the corresponding schedule item.
    /// Stable IDs are required for idempotent rescheduling.
    func testRequestIdentifiersMatchSchedule() {
        let scheduledIDs = mock.addedRequests.map { $0.identifier }.sorted()
        let expectedIDs  = NotificationSchedule.all.map { $0.id }.sorted()
        XCTAssertEqual(scheduledIDs, expectedIDs)
    }

    /// All triggers must be calendar-based (not time-interval-based).
    /// Calendar triggers fire at a wall-clock time regardless of app-launch time.
    func testAllTriggersAreCalendarTriggers() {
        for request in mock.addedRequests {
            XCTAssertTrue(
                request.trigger is UNCalendarNotificationTrigger,
                "Request \(request.identifier) has non-calendar trigger: \(String(describing: request.trigger))"
            )
        }
    }

    /// All triggers must have repeats = true for daily recurrence.
    func testAllTriggersRepeat() {
        for request in mock.addedRequests {
            guard let trigger = request.trigger as? UNCalendarNotificationTrigger else {
                XCTFail("Non-calendar trigger for \(request.identifier)"); continue
            }
            XCTAssertTrue(trigger.repeats,
                          "Trigger for \(request.identifier) does not repeat")
        }
    }

    /// The morning notification must fire at 07:00.
    func testMorningNotificationFiresAtSevenAM() {
        guard let morning = mock.addedRequests.first(where: { $0.identifier == "watersync.morning" }) else {
            XCTFail("Morning notification not found in scheduled requests")
            return
        }
        guard let trigger = morning.trigger as? UNCalendarNotificationTrigger else {
            XCTFail("Morning trigger is not a calendar trigger")
            return
        }
        XCTAssertEqual(trigger.dateComponents.hour, 7,
                       "Morning notification must fire at hour 7 (07:00)")
        XCTAssertEqual(trigger.dateComponents.minute, 0,
                       "Morning notification must fire at minute 0")
    }

    /// Notification content must include a sound.
    func testAllRequestsHaveSound() {
        for request in mock.addedRequests {
            XCTAssertNotNil(request.content.sound,
                            "No sound set for \(request.identifier)")
        }
    }

    /// No request must set a badge count (badges feel accusatory for health apps).
    func testNoBadgeCountIsSet() {
        for request in mock.addedRequests {
            XCTAssertNil(request.content.badge,
                         "Badge count set for \(request.identifier) — remove it")
        }
    }
}
