# Changelog — 06-notifications

## [1.0.0] — 2026-03-29

### Added
- Glasses UI: 8 drop icons fill left-to-right as drinks are logged
- GlassesView: pure display component (no SwiftData dependency), independently testable
- NotificationManager: UNUserNotificationCenter wrapper with protocol injection for testability
- NotificationSchedule: 8 static daily slots, 07:00–21:00, each with a science-backed health fact
- UNCalendarNotificationTrigger: wall-clock daily repeating (not interval-based, no drift)
- Foreground delivery: willPresent delegate returns [.banner, .sound] so reminders show in-app
- "Notification denied" banner in ContentView linking directly to iOS Settings
- "Next reminder in Xh Ym" label using SwiftUI Text relative date style
- Unit tests: GlassesLogicTests (9 cases), NotificationScheduleTests (8 cases), NotificationManagerTests (7 cases)

### Architecture decisions
- Data stored as ml (not glasses) for future Apple Health HKQuantitySample compatibility
- Calendar triggers over time-interval triggers: schedule is anchored to wall clock, not app sessions
- Protocol injection (NotificationScheduling) enables unit testing without real UNUserNotificationCenter
- No badge count: badges feel accusatory for a health app designed to encourage
- scheduleAll(using:) overload accepts mock centre for tests; production path uses no argument
