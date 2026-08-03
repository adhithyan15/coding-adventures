import 'dart:async';
import 'dart:convert';
import 'dart:ffi';
import 'dart:io';
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';

typedef _CreateNative = Pointer<Void> Function(Pointer<Char>, Double, Double);
typedef _CreateDart = Pointer<Void> Function(Pointer<Char>, double, double);
typedef _FreeNative = Void Function(Pointer<Void>);
typedef _FreeDart = void Function(Pointer<Void>);
typedef _PropsNative = Pointer<Char> Function(Pointer<Void>);
typedef _PropsDart = Pointer<Char> Function(Pointer<Void>);
typedef _EventNative =
    Pointer<Char> Function(Pointer<Void>, Pointer<Char>, Pointer<Char>);
typedef _EventDart =
    Pointer<Char> Function(Pointer<Void>, Pointer<Char>, Pointer<Char>);
typedef _PointNative = Uint8 Function(Pointer<Void>, Double, Double);
typedef _PointDart = int Function(Pointer<Void>, double, double);
typedef _ScalarNative = Uint8 Function(Pointer<Void>, Double);
typedef _ScalarDart = int Function(Pointer<Void>, double);
typedef _MetricsNative =
    Uint8 Function(
      Pointer<Void>,
      Pointer<Double>,
      Pointer<Double>,
      Pointer<Double>,
      Pointer<Double>,
    );
typedef _MetricsDart =
    int Function(
      Pointer<Void>,
      Pointer<Double>,
      Pointer<Double>,
      Pointer<Double>,
      Pointer<Double>,
    );
typedef _RenderNative =
    IntPtr Function(
      Pointer<Void>,
      Pointer<Uint8>,
      IntPtr,
      Pointer<Uint32>,
      Pointer<Uint32>,
    );
typedef _RenderDart =
    int Function(
      Pointer<Void>,
      Pointer<Uint8>,
      int,
      Pointer<Uint32>,
      Pointer<Uint32>,
    );
typedef _StringFreeNative = Void Function(Pointer<Char>);
typedef _StringFreeDart = void Function(Pointer<Char>);
typedef _MallocNative = Pointer<Void> Function(IntPtr);
typedef _MallocDart = Pointer<Void> Function(int);
typedef _SystemFreeNative = Void Function(Pointer<Void>);
typedef _SystemFreeDart = void Function(Pointer<Void>);

class _NativeMemory {
  _NativeMemory() {
    final library = Platform.isWindows
        ? DynamicLibrary.open('msvcrt.dll')
        : DynamicLibrary.process();
    malloc = library.lookupFunction<_MallocNative, _MallocDart>('malloc');
    free = library.lookupFunction<_SystemFreeNative, _SystemFreeDart>('free');
  }

  late final _MallocDart malloc;
  late final _SystemFreeDart free;

  Pointer<T> allocate<T extends NativeType>(int bytes) {
    final pointer = malloc(bytes).cast<T>();
    if (pointer == nullptr) {
      throw StateError('native allocation of $bytes bytes failed');
    }
    return pointer;
  }
}

final _NativeMemory _memory = _NativeMemory();

class _NativeString {
  _NativeString(String value) {
    final bytes = utf8.encode(value);
    pointer = _memory.allocate<Uint8>(bytes.length + 1).cast<Char>();
    final view = pointer.cast<Uint8>().asTypedList(bytes.length + 1);
    view.setRange(0, bytes.length, bytes);
    view[bytes.length] = 0;
  }

  late final Pointer<Char> pointer;

  void dispose() => _memory.free(pointer.cast<Void>());
}

class _VentureBindings {
  _VentureBindings(DynamicLibrary library)
    : create = library.lookupFunction<_CreateNative, _CreateDart>(
        'venture_browser_flutter_new',
      ),
      free = library.lookupFunction<_FreeNative, _FreeDart>(
        'venture_browser_flutter_free',
      ),
      props = library.lookupFunction<_PropsNative, _PropsDart>(
        'venture_browser_flutter_apply_props',
      ),
      event = library.lookupFunction<_EventNative, _EventDart>(
        'venture_browser_flutter_handle_event',
      ),
      scroll = library.lookupFunction<_ScalarNative, _ScalarDart>(
        'venture_browser_flutter_scroll',
      ),
      activateLink = library.lookupFunction<_PointNative, _PointDart>(
        'venture_browser_flutter_activate_link',
      ),
      updateHover = library.lookupFunction<_PointNative, _PointDart>(
        'venture_browser_flutter_update_hover',
      ),
      metrics = library.lookupFunction<_MetricsNative, _MetricsDart>(
        'venture_browser_flutter_scroll_metrics',
      ),
      resize = library.lookupFunction<_PointNative, _PointDart>(
        'venture_browser_flutter_resize',
      ),
      render = library.lookupFunction<_RenderNative, _RenderDart>(
        'venture_browser_flutter_render_rgba',
      ),
      stringFree = library.lookupFunction<_StringFreeNative, _StringFreeDart>(
        'venture_browser_flutter_string_free',
      );

  final _CreateDart create;
  final _FreeDart free;
  final _PropsDart props;
  final _EventDart event;
  final _ScalarDart scroll;
  final _PointDart activateLink;
  final _PointDart updateHover;
  final _MetricsDart metrics;
  final _PointDart resize;
  final _RenderDart render;
  final _StringFreeDart stringFree;
}

class _RgbaFrame {
  const _RgbaFrame(this.width, this.height, this.pixels);

  final int width;
  final int height;
  final Uint8List pixels;

  Future<ui.Image> decode() async {
    final buffer = await ui.ImmutableBuffer.fromUint8List(pixels);
    final descriptor = ui.ImageDescriptor.raw(
      buffer,
      width: width,
      height: height,
      rowBytes: width * 4,
      pixelFormat: ui.PixelFormat.rgba8888,
    );
    final codec = await descriptor.instantiateCodec();
    final frame = await codec.getNextFrame();
    codec.dispose();
    descriptor.dispose();
    buffer.dispose();
    return frame.image;
  }
}

class MosaicHost {
  MosaicHost._(this._bindings, this._host) {
    _contentSurface = VentureContentSurface(
      key: const Key('venture-content-surface'),
      host: this,
    );
  }

  static const double viewportWidth = 1024;
  static const double viewportHeight = 640;

  static MosaicHost? load() {
    final libraryPath =
        Platform.environment['VENTURE_BROWSER_FLUTTER_LIBRARY'] ??
        _defaultLibraryName();
    final startUrl =
        Platform.environment['VENTURE_BROWSER_START_URL'] ??
        'http://info.cern.ch/';
    try {
      return open(libraryPath: libraryPath, startUrl: startUrl);
    } on Object catch (error) {
      debugPrint('Venture Flutter host unavailable: $error');
      return null;
    }
  }

  static MosaicHost open({
    required String libraryPath,
    required String startUrl,
  }) {
    final bindings = _VentureBindings(DynamicLibrary.open(libraryPath));
    final url = _NativeString(startUrl);
    try {
      final host = bindings.create(url.pointer, viewportWidth, viewportHeight);
      if (host == nullptr) {
        throw StateError(
          'shared Venture browser session failed to load $startUrl',
        );
      }
      return MosaicHost._(bindings, host);
    } finally {
      url.dispose();
    }
  }

  static String _defaultLibraryName() {
    if (Platform.isMacOS) return 'libventure_browser_flutter.dylib';
    if (Platform.isWindows) return 'venture_browser_flutter.dll';
    return 'libventure_browser_flutter.so';
  }

  final _VentureBindings _bindings;
  final Pointer<Void> _host;
  final ValueNotifier<int> _surfaceRevision = ValueNotifier<int>(0);
  Completer<void> _surfaceReady = Completer<void>();
  late final Widget _contentSurface;
  void Function()? _propsChangedHandler;
  bool _disposed = false;

  FutureOr<Map<String, Object?>?> props() {
    return _decorate(_decodeResponse(_bindings.props(_host)));
  }

  FutureOr<Map<String, Object?>?> handleEvent(Map<String, Object?> event) {
    final name = _NativeString(event['event']?.toString() ?? '');
    final value = _NativeString(event['value']?.toString() ?? '');
    try {
      final response = _decorate(
        _decodeResponse(_bindings.event(_host, name.pointer, value.pointer)),
      );
      _surfaceChanged();
      return response;
    } finally {
      value.dispose();
      name.dispose();
    }
  }

  void setPropsChangedHandler(void Function()? handler) {
    _propsChangedHandler = handler;
  }

  Future<void> get surfaceReady => _surfaceReady.future;

  Map<String, double>? get scrollMetrics {
    final values = List<Pointer<Double>>.generate(
      4,
      (_) => _memory.allocate<Double>(sizeOf<Double>()),
    );
    try {
      if (_bindings.metrics(
            _host,
            values[0],
            values[1],
            values[2],
            values[3],
          ) ==
          0) {
        return null;
      }
      return <String, double>{
        'offset': values[0].value,
        'viewport': values[1].value,
        'content': values[2].value,
        'max': values[3].value,
      };
    } finally {
      for (final value in values) {
        _memory.free(value.cast<Void>());
      }
    }
  }

  String get statusText {
    final response = _decorate(_decodeResponse(_bindings.props(_host)));
    final props = response['props'];
    if (props is Map) return props['status-text']?.toString() ?? '';
    return '';
  }

  void scrollBy(double deltaY) {
    if (_bindings.scroll(_host, deltaY) != 0) _surfaceChanged();
  }

  void updateHover(double x, double y) {
    if (_bindings.updateHover(_host, x, y) != 0) _surfaceChanged();
  }

  void activateLink(double x, double y) {
    if (_bindings.activateLink(_host, x, y) != 0) _surfaceChanged();
  }

  void resize(double width, double height) {
    if (_bindings.resize(_host, width, height) != 0) _surfaceChanged();
  }

  _RgbaFrame renderFrame() {
    final width = _memory.allocate<Uint32>(sizeOf<Uint32>());
    final height = _memory.allocate<Uint32>(sizeOf<Uint32>());
    try {
      final length = _bindings.render(
        _host,
        nullptr.cast<Uint8>(),
        0,
        width,
        height,
      );
      if (length <= 0 || width.value == 0 || height.value == 0) {
        throw StateError('shared Venture Cairo renderer returned no pixels');
      }
      final output = _memory.allocate<Uint8>(length);
      try {
        final written = _bindings.render(_host, output, length, width, height);
        if (written != length) {
          throw StateError(
            'shared Venture Cairo render changed size during copy',
          );
        }
        return _RgbaFrame(
          width.value,
          height.value,
          Uint8List.fromList(output.asTypedList(length)),
        );
      } finally {
        _memory.free(output.cast<Void>());
      }
    } finally {
      _memory.free(height.cast<Void>());
      _memory.free(width.cast<Void>());
    }
  }

  Map<String, Object?> _decodeResponse(Pointer<Char> value) {
    if (value == nullptr) {
      throw StateError('shared Venture host returned a null response');
    }
    try {
      var length = 0;
      while (value.cast<Uint8>()[length] != 0) {
        length += 1;
      }
      final decoded = jsonDecode(
        utf8.decode(value.cast<Uint8>().asTypedList(length)),
      );
      return Map<String, Object?>.from(decoded as Map);
    } finally {
      _bindings.stringFree(value);
    }
  }

  Map<String, Object?> _decorate(Map<String, Object?> response) {
    final props = Map<String, Object?>.from(
      (response['props'] as Map?) ?? const <String, Object?>{},
    );
    props['content-surface'] = _contentSurface;
    return <String, Object?>{...response, 'props': props};
  }

  void _surfaceChanged() {
    if (_surfaceReady.isCompleted) {
      _surfaceReady = Completer<void>();
    }
    _surfaceRevision.value += 1;
    _propsChangedHandler?.call();
  }

  void _surfaceRendered() {
    if (!_surfaceReady.isCompleted) {
      _surfaceReady.complete();
    }
  }

  void dispose() {
    if (_disposed) return;
    _disposed = true;
    _propsChangedHandler = null;
    _bindings.free(_host);
    _surfaceRevision.dispose();
  }
}

class VentureContentSurface extends StatefulWidget {
  const VentureContentSurface({super.key, required this.host});

  final MosaicHost host;

  @override
  State<VentureContentSurface> createState() => _VentureContentSurfaceState();
}

class _VentureContentSurfaceState extends State<VentureContentSurface> {
  ui.Image? _image;
  bool _linkHover = false;
  int _renderGeneration = 0;

  @override
  void initState() {
    super.initState();
    widget.host._surfaceRevision.addListener(_scheduleRefresh);
    unawaited(_refresh());
  }

  @override
  void didUpdateWidget(VentureContentSurface oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.host != widget.host) {
      oldWidget.host._surfaceRevision.removeListener(_scheduleRefresh);
      widget.host._surfaceRevision.addListener(_scheduleRefresh);
      unawaited(_refresh());
    }
  }

  void _scheduleRefresh() => unawaited(_refresh());

  Future<void> _refresh() async {
    final generation = ++_renderGeneration;
    final rendered = widget.host.renderFrame();
    final nextImage = await rendered.decode();
    if (!mounted || generation != _renderGeneration) {
      nextImage.dispose();
      return;
    }
    final previous = _image;
    setState(() {
      _image = nextImage;
      _linkHover = widget.host.statusText.isNotEmpty;
    });
    previous?.dispose();
    widget.host._surfaceRendered();
  }

  void _handleHover(PointerHoverEvent event) {
    widget.host.updateHover(event.localPosition.dx, event.localPosition.dy);
    final linkHover = widget.host.statusText.isNotEmpty;
    if (linkHover != _linkHover && mounted) {
      setState(() => _linkHover = linkHover);
    }
  }

  void _handlePointerSignal(PointerSignalEvent event) {
    if (event is PointerScrollEvent) {
      widget.host.scrollBy(event.scrollDelta.dy);
    }
  }

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: MosaicHost.viewportWidth,
      height: MosaicHost.viewportHeight,
      child: MouseRegion(
        cursor: _linkHover
            ? SystemMouseCursors.click
            : SystemMouseCursors.basic,
        onHover: _handleHover,
        child: Listener(
          behavior: HitTestBehavior.opaque,
          onPointerSignal: _handlePointerSignal,
          child: GestureDetector(
            behavior: HitTestBehavior.opaque,
            onTapUp: (details) => widget.host.activateLink(
              details.localPosition.dx,
              details.localPosition.dy,
            ),
            child: _image == null
                ? const ColoredBox(color: Colors.white)
                : RawImage(
                    key: const Key('venture-content-image'),
                    image: _image,
                    fit: BoxFit.fill,
                  ),
          ),
        ),
      ),
    );
  }

  @override
  void dispose() {
    widget.host._surfaceRevision.removeListener(_scheduleRefresh);
    _renderGeneration += 1;
    _image?.dispose();
    super.dispose();
  }
}
