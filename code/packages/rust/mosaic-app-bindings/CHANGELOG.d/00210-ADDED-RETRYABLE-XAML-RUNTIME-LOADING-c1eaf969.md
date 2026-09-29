### Added — retryable XAML runtime loading

The XAML runtime host now records loader detail and can retry after `Close()`
instead of caching an unavailable runtime for the life of the process. Strict
generated WinUI startup can therefore report a missing DLL, restore it, and
recover in place (#16097).

