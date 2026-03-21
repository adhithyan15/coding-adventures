/**
 * Progress Bar — a reusable text-based progress bar for tracking operations.
 *
 * This package provides a simple, dependency-free progress bar that renders
 * in the terminal using Unicode block characters. It supports both flat
 * (single-level) and hierarchical (parent/child) progress tracking.
 *
 * The public API consists of:
 *
 *   Tracker      — the main progress bar class
 *   NullTracker  — a no-op drop-in replacement (Null Object pattern)
 *   Event        — the message type sent to the tracker
 *   EventType    — enum of event kinds (Started, Finished, Skipped)
 *   Writable     — interface for output destinations (for testability)
 *   formatActivity — helper to format in-flight activity strings
 */

export { Tracker, NullTracker, formatActivity } from "./tracker.js";
export { EventType } from "./tracker.js";
export type { Event, Writable } from "./tracker.js";
