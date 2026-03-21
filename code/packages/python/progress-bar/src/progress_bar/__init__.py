"""Progress bar package — reusable text-based progress tracking.

This package provides a thread-safe progress bar for tracking concurrent
operations. It's the Python port of the Go progress-bar package from the
coding-adventures project.

Public API:
    Tracker     — the main progress bar engine
    Event       — a message sent to the tracker
    EventType   — enum of event kinds (STARTED, FINISHED, SKIPPED)
    NullTracker — a no-op tracker for when progress display is disabled
"""

from progress_bar.tracker import Event, EventType, NullTracker, Tracker

__all__ = ["Tracker", "Event", "EventType", "NullTracker"]
