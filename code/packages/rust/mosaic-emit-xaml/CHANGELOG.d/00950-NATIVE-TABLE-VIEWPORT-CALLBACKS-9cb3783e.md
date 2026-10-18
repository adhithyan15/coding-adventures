### Native table viewport callbacks

Validate row-window callback payloads and bindings, diagnose unsupported shapes,
and generate renamed typed dispatch handlers. Native tables report finite
capacity after layout and route fractional wheel movement to bounded row shifts.
Detach generations suppress stale callbacks; virtual tables keep one vertical
scroll owner and preserve shared horizontal header alignment. Exact flex: 1 now
reserves a star track, fixing content-sized scroll ports during resize.

Executed generated-C# probes cover arithmetic and queued callback lifecycle.
Native Debug interaction verified scrolling, selection/navigation, formula
commit/restart, shrinking/growing capacity and the last-row boundary. Performance
percentiles, full realization counts and remaining accessibility acceptance are
still open.
