### Fixed — deferred effect completion refreshes window status

Native-complete UserControl windows now display the status returned when they
refresh properties after a deferred effect completes. Save and cancellation no
longer leave a pending-effect persistence warning frozen in the window after the
host has recovered; remaining host warnings and refresh failures stay visible.
Native acceptance remains tracked separately under #14279.
