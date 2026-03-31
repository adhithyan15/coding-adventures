# 02 — Water Counter (iOS / Swift)

Introduces `@State` — the foundation of all interactivity in SwiftUI.
Tap the button, the counter goes up. That's it. But the mechanics
behind it are everything.

## What it does

- Tap "Log a Drink" → adds 250ml to today's total
- Progress bar fills toward the 2,000ml daily goal
- Counter animates between values with a numeric transition
- Button springs on tap, disables when goal is reached
- Icon fills in when you hit the goal
- Reset button clears the counter

## What it teaches

- `@State` — view-owned mutable state that triggers re-renders
- `Button` — action closures
- `ProgressView` — built-in progress bar
- `withAnimation` — wrapping state changes to animate them
- `.contentTransition(.numericText())` — animating number changes
- `.disabled()` — conditionally disabling controls

## The key idea

```swift
@State private var totalMl: Int = 0

Button("Log a Drink") {
    totalMl += 250      // change the value
}

Text("\(totalMl) ml")   // UI updates automatically
```

SwiftUI watches `totalMl`. When it changes, it re-renders every view
that reads it. You never call reload/update/refresh manually.

## Running locally

```bash
cd code/programs/swift/02-water-counter
xcodegen generate
xcodebuild build \
  -scheme WaterCounter \
  -destination 'platform=iOS Simulator,name=iPhone 17,OS=26.4' \
  CODE_SIGNING_ALLOWED=NO
```
