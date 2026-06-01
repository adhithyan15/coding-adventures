// mosaic_flux.dart — public surface of mosaic_flux_flutter.

export 'src/action.dart';
export 'src/store.dart';
export 'src/middleware.dart';
export 'src/selector.dart';
export 'src/devtools.dart';

// MosaicBuilder widget — deferred to v0.2.0. v0.1.0 ships only the
// pure-Dart core so the package builds without the Flutter SDK.
// Consumers wanting the widget can use the imperative subscribe()
// API to trigger setState manually in their StatefulWidget.
