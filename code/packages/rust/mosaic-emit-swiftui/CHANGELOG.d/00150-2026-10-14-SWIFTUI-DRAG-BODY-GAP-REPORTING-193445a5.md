## 2026-10-14 (SwiftUI drag-body gap reporting)

Recognize the gap already consumed by HostDraggable and HostDropTarget VStack
bodies. Unsupported values and shared parts with a non-spacing occurrence
remain reported. TaskApp's measured SwiftUI style inventory falls from 63 to
59 without changing emitted drag behavior (#17481).
