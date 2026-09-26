### Fixed - Live native disabled state

Slot-backed `disabled` properties now lower to one-way WinUI `IsEnabled`
bindings. Generated buttons and inputs therefore observe runtime Mosaic state
changes instead of retaining the value captured when their XAML first loads.

