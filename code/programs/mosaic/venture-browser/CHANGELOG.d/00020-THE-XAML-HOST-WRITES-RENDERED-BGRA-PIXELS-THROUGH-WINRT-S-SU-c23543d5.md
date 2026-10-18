- The XAML host writes rendered BGRA pixels through WinRT's supported
  `IBuffer.AsStream()` projection, avoiding a native COM projection crash in
  the emitted WinUI application.
