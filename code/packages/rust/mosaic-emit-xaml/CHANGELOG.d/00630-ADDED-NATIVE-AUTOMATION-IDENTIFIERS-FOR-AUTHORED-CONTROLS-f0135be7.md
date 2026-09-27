### Added - Native automation identifiers for authored controls

`HostInput` and `HostButton` now preserve their MLL part names as WinUI
`AutomationProperties.AutomationId` values. Generated applications can locate
the same Mosaic-authored control deterministically for accessibility and direct
native interaction acceptance without adding a parallel Win32 control tree.

