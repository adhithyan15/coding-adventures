### Added - HostSurface native composition

WinUI output now lowers Mosaic `HostSurface ( content: slot: ... )` to a
`ContentPresenter` bound to the host-supplied `UIElement`, wrapped by the
shared MSL-styled `Border`. This gives Direct2D and other native renderers a
typed mount point inside Mosaic-authored application chrome.

