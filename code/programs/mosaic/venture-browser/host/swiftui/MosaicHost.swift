import AppKit
import Darwin
import Foundation
import Metal
import QuartzCore

private final class VentureNativeLibrary {
  typealias New = @convention(c) (UnsafePointer<CChar>?, Double, Double) -> UnsafeMutableRawPointer?
  typealias Free = @convention(c) (UnsafeMutableRawPointer?) -> Void
  typealias ApplyProps = @convention(c) (UnsafeMutableRawPointer?) -> UnsafeMutablePointer<CChar>?
  typealias HandleEvent = @convention(c) (
    UnsafeMutableRawPointer?, UnsafePointer<CChar>?, UnsafePointer<CChar>?
  ) -> UnsafeMutablePointer<CChar>?
  typealias Scroll = @convention(c) (UnsafeMutableRawPointer?, Double) -> UInt8
  typealias ScrollCommand = @convention(c) (
    UnsafeMutableRawPointer?, UnsafePointer<CChar>?
  ) -> UInt8
  typealias ActivateLink = @convention(c) (UnsafeMutableRawPointer?, Double, Double) -> UInt8
  typealias Resize = @convention(c) (UnsafeMutableRawPointer?, Double, Double) -> UInt8
  typealias Render = @convention(c) (UnsafeMutableRawPointer?, UnsafeMutableRawPointer?) -> UInt8
  typealias StringFree = @convention(c) (UnsafeMutablePointer<CChar>?) -> Void

  let library: UnsafeMutableRawPointer
  let new: New
  let free: Free
  let applyProps: ApplyProps
  let handleEvent: HandleEvent
  let scroll: Scroll
  let scrollCommand: ScrollCommand
  let activateLink: ActivateLink
  let resize: Resize
  let render: Render
  let stringFree: StringFree

  init?() {
    let environment = ProcessInfo.processInfo.environment["VENTURE_BROWSER_LIBRARY"]
    let current = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
      .appendingPathComponent("libventure_browser_macos.dylib").path
    let executable = Bundle.main.executableURL?.deletingLastPathComponent()
      .appendingPathComponent("libventure_browser_macos.dylib").path
    let candidates = [environment, current, executable].compactMap { $0 }
    guard let library = candidates.lazy.compactMap({ dlopen($0, RTLD_NOW | RTLD_LOCAL) }).first else {
      return nil
    }

    func symbol<T>(_ name: String, as type: T.Type) -> T? {
      guard let raw = dlsym(library, name) else { return nil }
      return unsafeBitCast(raw, to: type)
    }

    guard
      let new = symbol("venture_browser_macos_new", as: New.self),
      let free = symbol("venture_browser_macos_free", as: Free.self),
      let applyProps = symbol("venture_browser_macos_apply_props", as: ApplyProps.self),
      let handleEvent = symbol("venture_browser_macos_handle_event", as: HandleEvent.self),
      let scroll = symbol("venture_browser_macos_scroll", as: Scroll.self),
      let scrollCommand = symbol(
        "venture_browser_macos_scroll_command", as: ScrollCommand.self
      ),
      let activateLink = symbol("venture_browser_macos_activate_link", as: ActivateLink.self),
      let resize = symbol("venture_browser_macos_resize", as: Resize.self),
      let render = symbol("venture_browser_macos_render", as: Render.self),
      let stringFree = symbol("venture_browser_string_free", as: StringFree.self)
    else {
      dlclose(library)
      return nil
    }

    self.library = library
    self.new = new
    self.free = free
    self.applyProps = applyProps
    self.handleEvent = handleEvent
    self.scroll = scroll
    self.scrollCommand = scrollCommand
    self.activateLink = activateLink
    self.resize = resize
    self.render = render
    self.stringFree = stringFree
  }

  deinit {
    dlclose(library)
  }

  func decode(_ value: UnsafeMutablePointer<CChar>?) -> NSDictionary? {
    guard let value else { return nil }
    defer { stringFree(value) }
    let data = Data(bytes: value, count: strlen(value))
    return try? JSONSerialization.jsonObject(with: data) as? NSDictionary
  }
}

@objc(MosaicHost)
final class MosaicHost: NSObject, MosaicHostBridgeObject {
  private let native: VentureNativeLibrary?
  private var browser: UnsafeMutableRawPointer?
  private var contentView: VentureContentView?
  private var propsChangedHandler: (() -> Void)?

  required override init() {
    let native = VentureNativeLibrary()
    self.native = native
    let startURL = ProcessInfo.processInfo.environment["VENTURE_START_URL"]
      ?? "http://info.cern.ch/"
    self.browser = startURL.withCString { native?.new($0, 1024, 640) }
    super.init()
  }

  deinit {
    native?.free(browser)
  }

  func applyProps() -> NSDictionary? {
    native?.decode(native?.applyProps(browser))
      ?? ["error": "Venture native bridge is unavailable"]
  }

  func handleEvent(_ envelope: NSDictionary, name: NSString) -> NSDictionary? {
    guard let native, let browser else { return applyProps() }
    let value = envelope["value"] as? String
    let response = name.utf8String.flatMap { eventName in
      if let value {
        return value.withCString { native.handleEvent(browser, eventName, $0) }
      }
      return native.handleEvent(browser, eventName, nil)
    }
    contentView?.renderPage()
    return native.decode(response)
  }

  func node(named name: NSString) -> NSObject? {
    guard name == "content-surface", native != nil, browser != nil else { return nil }
    if contentView == nil {
      contentView = VentureContentView(host: self)
    }
    return contentView
  }

  func setPropsChangedHandler(_ handler: @escaping () -> Void) {
    propsChangedHandler = handler
  }

  fileprivate func render(layer: CAMetalLayer) {
    guard let native, let browser else { return }
    let rawLayer = Unmanaged.passUnretained(layer).toOpaque()
    _ = native.render(browser, rawLayer)
  }

  fileprivate func scroll(by deltaY: Double) {
    guard let native, let browser, native.scroll(browser, deltaY) != 0 else { return }
    contentView?.renderPage()
  }

  fileprivate func scroll(command: String) {
    guard let native, let browser else { return }
    let changed = command.withCString { native.scrollCommand(browser, $0) }
    guard changed != 0 else { return }
    contentView?.renderPage()
  }

  fileprivate func navigateHistory(eventName: String) {
    _ = handleEvent([:], name: eventName as NSString)
    propsChangedHandler?()
  }

  fileprivate func activateLink(at point: NSPoint) {
    guard let native, let browser, native.activateLink(browser, point.x, point.y) != 0 else {
      return
    }
    contentView?.renderPage()
    propsChangedHandler?()
  }

  fileprivate func resize(width: Double, height: Double) {
    guard let native, let browser else { return }
    _ = native.resize(browser, width, height)
  }
}

private final class VentureContentView: NSView {
  private weak var host: MosaicHost?

  init(host: MosaicHost) {
    self.host = host
    super.init(frame: NSRect(x: 0, y: 0, width: 1024, height: 640))
    wantsLayer = true
  }

  required init?(coder: NSCoder) {
    nil
  }

  override var isFlipped: Bool { true }
  override var acceptsFirstResponder: Bool { true }

  override func makeBackingLayer() -> CALayer {
    let layer = CAMetalLayer()
    layer.device = MTLCreateSystemDefaultDevice()
    layer.pixelFormat = .bgra8Unorm
    layer.drawableSize = CGSize(width: 1024, height: 640)
    return layer
  }

  override func layout() {
    super.layout()
    guard let layer = layer as? CAMetalLayer else { return }
    layer.contentsScale = window?.backingScaleFactor ?? NSScreen.main?.backingScaleFactor ?? 1
    if bounds.width > 0, bounds.height > 0 {
      host?.resize(width: bounds.width, height: bounds.height)
    }
    layer.drawableSize = CGSize(
      width: max(1, bounds.width * layer.contentsScale),
      height: max(1, bounds.height * layer.contentsScale)
    )
    renderPage()
  }

  override func viewDidMoveToWindow() {
    super.viewDidMoveToWindow()
    renderPage()
  }

  override func scrollWheel(with event: NSEvent) {
    let scale = event.hasPreciseScrollingDeltas ? 1.0 : 40.0
    host?.scroll(by: -event.scrollingDeltaY * scale)
  }

  override func mouseDown(with event: NSEvent) {
    window?.makeFirstResponder(self)
    super.mouseDown(with: event)
  }

  override func mouseUp(with event: NSEvent) {
    host?.activateLink(at: convert(event.locationInWindow, from: nil))
  }

  override func keyDown(with event: NSEvent) {
    let modifiers = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
    if modifiers.contains(.command)
      && modifiers.intersection([.control, .option, .shift]).isEmpty
    {
      switch event.keyCode {
      case 123:
        host?.navigateHistory(eventName: "onBack")
        return
      case 124:
        host?.navigateHistory(eventName: "onForward")
        return
      default:
        break
      }
    }
    guard modifiers.intersection([.command, .control, .option]).isEmpty else {
      super.keyDown(with: event)
      return
    }
    let command: String?
    switch event.keyCode {
    case 126:
      command = "line-up"
    case 125:
      command = "line-down"
    case 116:
      command = "page-up"
    case 121:
      command = "page-down"
    case 49:
      command = modifiers.contains(.shift) ? "page-up" : "page-down"
    case 115:
      command = "document-start"
    case 119:
      command = "document-end"
    default:
      command = nil
    }
    guard let command else {
      super.keyDown(with: event)
      return
    }
    host?.scroll(command: command)
  }

  fileprivate func renderPage() {
    guard let layer = layer as? CAMetalLayer else { return }
    let scale = layer.contentsScale > 0 ? layer.contentsScale : 1
    layer.drawableSize = CGSize(
      width: max(1, bounds.width * scale),
      height: max(1, bounds.height * scale)
    )
    host?.render(layer: layer)
  }
}
