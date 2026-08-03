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
  private var acceptanceReported = false
  private var interactionAcceptanceStarted = false
  private var lastSurfaceWheelDelta: Double?
  private var lastSurfaceKeyboardCommand: String?
  private var lastSurfaceHistoryEvent: String?
  private var lastSurfaceFocusState: String?
  private var lastSurfacePointerPoint: NSPoint?
  private var surfaceResizeBaseline: NSSize?
  private var lastSurfaceResizeSize: NSSize?
  private var surfaceRenderBaseline: CGSize?
  private var lastSurfaceRenderSize: CGSize?
  private var chromeEventCounts: [String: Int] = [:]

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
    chromeEventCounts[name as String, default: 0] += 1
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

  func runInteractionAcceptance() {
    let environment = ProcessInfo.processInfo.environment
    guard !interactionAcceptanceStarted,
      let markerPath = environment["VENTURE_BROWSER_INTERACTION_ACCEPTANCE_PATH"],
      let targetURL = environment["VENTURE_BROWSER_INTERACTION_URL"],
      environment["VENTURE_BROWSER_INTERACTION_LINK_URL"] != nil,
      let startURL = environment["VENTURE_START_URL"]
    else { return }
    interactionAcceptanceStarted = true
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.25) { [weak self] in
      self?.attemptInteraction(
        startURL: startURL, targetURL: targetURL, markerPath: markerPath, remaining: 50)
    }
  }

  private func attemptInteraction(
    startURL: String, targetURL: String, markerPath: String, remaining: Int
  ) {
    guard !NSApp.windows.isEmpty else {
      retryInteraction(
        startURL: startURL, targetURL: targetURL, markerPath: markerPath,
        remaining: remaining)
      return
    }
    var visited = Set<ObjectIdentifier>()
    guard let address = findEditableTextField(in: NSApp, visited: &visited) else {
      retryInteraction(
        startURL: startURL, targetURL: targetURL, markerPath: markerPath,
        remaining: remaining)
      return
    }
    let response = applyProps()
    let props = response?["props"] as? NSDictionary
    let currentAddress = props?["address"] as? String ?? ""
    let backDisabled = props?["back-disabled"] as? Bool
    let forwardDisabled = props?["forward-disabled"] as? Bool
    guard currentAddress == startURL, backDisabled == true, forwardDisabled == true else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error", "address": currentAddress,
          "error": "initial native navigation control state did not match",
        ],
        to: markerPath)
      return
    }
    let backEventCount = chromeEventCounts["onBack", default: 0]
    let forwardEventCount = chromeEventCounts["onForward", default: 0]
    guard performNativeButtonClick(identifier: "back-button"),
      performNativeButtonClick(identifier: "forward-button")
    else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error",
          "error": "initial native navigation controls not found",
        ],
        to: markerPath)
      return
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.15) { [weak self, weak address] in
      self?.verifyInitialDisabledControls(
        address: address, startURL: startURL, targetURL: targetURL, markerPath: markerPath,
        backEventCount: backEventCount, forwardEventCount: forwardEventCount)
    }
  }

  private func verifyInitialDisabledControls(
    address: NSTextField?, startURL: String, targetURL: String, markerPath: String,
    backEventCount: Int, forwardEventCount: Int
  ) {
    guard chromeEventCounts["onBack", default: 0] == backEventCount,
      chromeEventCounts["onForward", default: 0] == forwardEventCount
    else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error",
          "error": "initial native disabled navigation control dispatched",
        ],
        to: markerPath)
      return
    }
    guard let address else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error",
          "error": "address-input unavailable after initial navigation state check",
        ],
        to: markerPath)
      return
    }
    address.selectText(nil)
    guard let editor = address.currentEditor() as? NSTextView else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error",
          "error": "address-input native editor unavailable",
        ],
        to: markerPath)
      return
    }
    editor.selectAll(nil)
    editor.insertText(targetURL, replacementRange: editor.selectedRange())
    if let window = address.window {
      let addressFrame = address.convert(address.bounds, to: nil)
      sendPrimaryClick(
        at: NSPoint(x: addressFrame.maxX + 24, y: addressFrame.midY),
        to: window)
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) { [weak self] in
      self?.verifyInitialNavigation(
        startURL: startURL, targetURL: targetURL, markerPath: markerPath, remaining: 50)
    }
  }

  private func retryInteraction(
    startURL: String, targetURL: String, markerPath: String, remaining: Int
  ) {
    guard remaining > 0 else {
      writeInteractionResult(
        ["backend": "swiftui", "status": "error", "error": "native controls not found"],
        to: markerPath)
      return
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) { [weak self] in
      self?.attemptInteraction(
        startURL: startURL, targetURL: targetURL, markerPath: markerPath,
        remaining: remaining - 1)
    }
  }

  private func verifyInitialNavigation(
    startURL: String, targetURL: String, markerPath: String, remaining: Int
  ) {
    let response = applyProps()
    let props = response?["props"] as? NSDictionary
    let address = props?["address"] as? String ?? ""
    let pageTitle = props?["page-title"] as? String ?? ""
    let backDisabled = props?["back-disabled"] as? Bool
    let forwardDisabled = props?["forward-disabled"] as? Bool
    if address == targetURL, pageTitle == "Venture interaction acceptance",
      backDisabled == false, forwardDisabled == true
    {
      let eventCount = chromeEventCounts["onForward", default: 0]
      guard performNativeButtonClick(identifier: "forward-button") else {
        writeInteractionResult(
          ["backend": "swiftui", "status": "error", "error": "forward-button not found"],
          to: markerPath)
        return
      }
      DispatchQueue.main.asyncAfter(deadline: .now() + 0.15) { [weak self] in
        self?.verifyForwardDisabledAtTarget(
          startURL: startURL, targetURL: targetURL, markerPath: markerPath,
          eventCount: eventCount)
      }
      return
    }
    guard remaining > 0 else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error", "address": address,
          "pageTitle": pageTitle, "error": "navigation state did not update",
        ],
        to: markerPath)
      return
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) { [weak self] in
      self?.verifyInitialNavigation(
        startURL: startURL, targetURL: targetURL, markerPath: markerPath,
        remaining: remaining - 1)
    }
  }

  private func verifyForwardDisabledAtTarget(
    startURL: String, targetURL: String, markerPath: String, eventCount: Int
  ) {
    guard chromeEventCounts["onForward", default: 0] == eventCount else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error",
          "error": "native disabled Forward button dispatched",
        ],
        to: markerPath)
      return
    }
    guard performNativeButtonClick(identifier: "back-button") else {
      writeInteractionResult(
        ["backend": "swiftui", "status": "error", "error": "back-button not found"],
        to: markerPath)
      return
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.25) { [weak self] in
      self?.verifyBackNavigation(
        startURL: startURL, targetURL: targetURL, markerPath: markerPath, remaining: 50)
    }
  }

  private func verifyBackNavigation(
    startURL: String, targetURL: String, markerPath: String, remaining: Int
  ) {
    let response = applyProps()
    let props = response?["props"] as? NSDictionary
    let address = props?["address"] as? String ?? ""
    let pageTitle = props?["page-title"] as? String ?? ""
    let backDisabled = props?["back-disabled"] as? Bool
    let forwardDisabled = props?["forward-disabled"] as? Bool
    if address == startURL, pageTitle == "Venture launch acceptance",
      backDisabled == true, forwardDisabled == false
    {
      let eventCount = chromeEventCounts["onBack", default: 0]
      guard performNativeButtonClick(identifier: "back-button") else {
        writeInteractionResult(
          ["backend": "swiftui", "status": "error", "error": "back-button not found"],
          to: markerPath)
        return
      }
      DispatchQueue.main.asyncAfter(deadline: .now() + 0.15) { [weak self] in
        self?.verifyBackDisabledAtStart(
          startURL: startURL, targetURL: targetURL, markerPath: markerPath,
          eventCount: eventCount)
      }
      return
    }
    guard remaining > 0 else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error", "address": address,
          "pageTitle": pageTitle, "error": "back navigation state did not update",
        ],
        to: markerPath)
      return
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) { [weak self] in
      self?.verifyBackNavigation(
        startURL: startURL, targetURL: targetURL, markerPath: markerPath,
        remaining: remaining - 1)
    }
  }

  private func verifyBackDisabledAtStart(
    startURL: String, targetURL: String, markerPath: String, eventCount: Int
  ) {
    guard chromeEventCounts["onBack", default: 0] == eventCount else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error",
          "error": "native disabled Back button dispatched",
        ],
        to: markerPath)
      return
    }
    guard performNativeButtonClick(identifier: "forward-button") else {
      writeInteractionResult(
        ["backend": "swiftui", "status": "error", "error": "forward-button not found"],
        to: markerPath)
      return
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.25) { [weak self] in
      self?.verifyForwardNavigation(
        startURL: startURL, targetURL: targetURL, markerPath: markerPath, remaining: 50)
    }
  }

  private func verifyForwardNavigation(
    startURL: String, targetURL: String, markerPath: String, remaining: Int
  ) {
    let response = applyProps()
    let props = response?["props"] as? NSDictionary
    let address = props?["address"] as? String ?? ""
    let pageTitle = props?["page-title"] as? String ?? ""
    let backDisabled = props?["back-disabled"] as? Bool
    let forwardDisabled = props?["forward-disabled"] as? Bool
    if address == targetURL, pageTitle == "Venture interaction acceptance",
      backDisabled == false, forwardDisabled == true
    {
      let eventCount = chromeEventCounts["onForward", default: 0]
      guard performNativeButtonClick(identifier: "forward-button") else {
        writeInteractionResult(
          ["backend": "swiftui", "status": "error", "error": "forward-button not found"],
          to: markerPath)
        return
      }
      DispatchQueue.main.asyncAfter(deadline: .now() + 0.15) { [weak self] in
        self?.verifyForwardDisabledAfterForward(
          startURL: startURL, targetURL: targetURL, markerPath: markerPath,
          eventCount: eventCount)
      }
      return
    }
    guard remaining > 0 else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error", "address": address,
          "pageTitle": pageTitle, "error": "forward navigation state did not update",
        ],
        to: markerPath)
      return
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) { [weak self] in
      self?.verifyForwardNavigation(
        startURL: startURL, targetURL: targetURL, markerPath: markerPath,
        remaining: remaining - 1)
    }
  }

  private func verifyForwardDisabledAfterForward(
    startURL: String, targetURL: String, markerPath: String, eventCount: Int
  ) {
    guard chromeEventCounts["onForward", default: 0] == eventCount else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error",
          "error": "native disabled Forward button dispatched after Forward",
        ],
        to: markerPath)
      return
    }
    guard performNativeButtonClick(identifier: "reload-button") else {
      writeInteractionResult(
        ["backend": "swiftui", "status": "error", "error": "reload-button not found"],
        to: markerPath)
      return
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.25) { [weak self] in
      self?.verifyReload(
        startURL: startURL, targetURL: targetURL, markerPath: markerPath, remaining: 50)
    }
  }

  private func verifyReload(
    startURL: String, targetURL: String, markerPath: String, remaining: Int
  ) {
    let response = applyProps()
    let props = response?["props"] as? NSDictionary
    let address = props?["address"] as? String ?? ""
    let pageTitle = props?["page-title"] as? String ?? ""
    if address == targetURL, pageTitle == "Venture reload acceptance" {
      guard performNativeButtonClick(identifier: "home-button") else {
        writeInteractionResult(
          ["backend": "swiftui", "status": "error", "error": "home-button not found"],
          to: markerPath)
        return
      }
      DispatchQueue.main.asyncAfter(deadline: .now() + 0.25) { [weak self] in
        self?.verifyHome(
          startURL: startURL, targetURL: targetURL, markerPath: markerPath, remaining: 50)
      }
      return
    }
    guard remaining > 0 else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error", "address": address,
          "pageTitle": pageTitle, "error": "reload state did not update",
        ],
        to: markerPath)
      return
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) { [weak self] in
      self?.verifyReload(
        startURL: startURL, targetURL: targetURL, markerPath: markerPath,
        remaining: remaining - 1)
    }
  }

  private func verifyHome(
    startURL: String, targetURL: String, markerPath: String, remaining: Int
  ) {
    let response = applyProps()
    let props = response?["props"] as? NSDictionary
    let address = props?["address"] as? String ?? ""
    let pageTitle = props?["page-title"] as? String ?? ""
    if address == startURL, pageTitle == "Venture launch acceptance" {
      guard performNativeAddressCommit(value: targetURL) else {
        writeInteractionResult(
          [
            "backend": "swiftui", "status": "error",
            "error": "address-input native Return event unavailable",
          ],
          to: markerPath)
        return
      }
      DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) { [weak self] in
        self?.verifyAddressCommit(
          startURL: startURL, targetURL: targetURL, markerPath: markerPath, remaining: 50)
      }
      return
    }
    guard remaining > 0 else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error", "address": address,
          "pageTitle": pageTitle, "error": "home navigation state did not update",
        ],
        to: markerPath)
      return
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) { [weak self] in
      self?.verifyHome(
        startURL: startURL, targetURL: targetURL, markerPath: markerPath,
        remaining: remaining - 1)
    }
  }

  private func verifyAddressCommit(
    startURL: String, targetURL: String, markerPath: String, remaining: Int
  ) {
    let response = applyProps()
    let props = response?["props"] as? NSDictionary
    let address = props?["address"] as? String ?? ""
    let pageTitle = props?["page-title"] as? String ?? ""
    if address == targetURL, pageTitle == "Venture commit acceptance" {
      guard performNativeButtonClick(identifier: "home-button") else {
        writeInteractionResult(
          ["backend": "swiftui", "status": "error", "error": "home-button not found"],
          to: markerPath)
        return
      }
      DispatchQueue.main.asyncAfter(deadline: .now() + 0.25) { [weak self] in
        self?.verifyCommittedHome(
          startURL: startURL, targetURL: targetURL, markerPath: markerPath, remaining: 50)
      }
      return
    }
    guard remaining > 0 else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error", "address": address,
          "pageTitle": pageTitle, "error": "native address commit did not navigate",
        ],
        to: markerPath)
      return
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) { [weak self] in
      self?.verifyAddressCommit(
        startURL: startURL, targetURL: targetURL, markerPath: markerPath,
        remaining: remaining - 1)
    }
  }

  private func verifyCommittedHome(
    startURL: String, targetURL: String, markerPath: String, remaining: Int
  ) {
    let response = applyProps()
    let props = response?["props"] as? NSDictionary
    let address = props?["address"] as? String ?? ""
    let pageTitle = props?["page-title"] as? String ?? ""
    if address == startURL, pageTitle == "Venture launch acceptance" {
      guard performNativeSurfaceWheel() else {
        writeInteractionResult(
          ["backend": "swiftui", "status": "error", "error": "content surface unavailable"],
          to: markerPath)
        return
      }
      DispatchQueue.main.asyncAfter(deadline: .now() + 0.25) { [weak self] in
        self?.verifySurfaceWheel(
          startURL: startURL, targetURL: targetURL, markerPath: markerPath)
      }
      return
    }
    guard remaining > 0 else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error", "address": address,
          "pageTitle": pageTitle, "error": "home after address commit did not update",
        ],
        to: markerPath)
      return
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) { [weak self] in
      self?.verifyCommittedHome(
        startURL: startURL, targetURL: targetURL, markerPath: markerPath,
        remaining: remaining - 1)
    }
  }

  private func verifySurfaceWheel(
    startURL: String, targetURL: String, markerPath: String
  ) {
    guard let delta = lastSurfaceWheelDelta, delta > 0 else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error",
          "error": "native wheel did not scroll the shared viewport",
        ],
        to: markerPath)
      return
    }
    guard performNativeSurfaceKey(keyCode: 119) else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error",
          "error": "content surface unavailable for native End key",
        ],
        to: markerPath)
      return
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.25) { [weak self] in
      self?.verifySurfaceKeyboard(
        startURL: startURL, targetURL: targetURL, markerPath: markerPath)
    }
  }

  private func verifySurfaceKeyboard(
    startURL: String, targetURL: String, markerPath: String
  ) {
    guard lastSurfaceKeyboardCommand == "document-end" else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error",
          "error": "native End key did not scroll the shared viewport",
        ],
        to: markerPath)
      return
    }
    guard performNativeSurfaceKey(keyCode: 115) else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error",
          "error": "native Home key could not reset the shared viewport",
        ],
        to: markerPath)
      return
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.25) { [weak self] in
      self?.activateSurfaceLink(
        startURL: startURL, targetURL: targetURL, markerPath: markerPath)
    }
  }

  private func activateSurfaceLink(
    startURL: String, targetURL: String, markerPath: String
  ) {
    guard lastSurfaceKeyboardCommand == "document-start" else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error",
          "error": "native Home key did not reset the shared viewport",
        ],
        to: markerPath)
      return
    }
    guard performNativeSurfaceClick(at: NSPoint(x: 32, y: 26)) else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error",
          "error": "content surface unavailable for native link activation",
        ],
        to: markerPath)
      return
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.25) { [weak self] in
      self?.verifySurfaceLink(
        startURL: startURL, targetURL: targetURL, markerPath: markerPath, remaining: 50)
    }
  }

  private func verifySurfaceLink(
    startURL: String, targetURL: String, markerPath: String, remaining: Int
  ) {
    let linkURL = ProcessInfo.processInfo.environment["VENTURE_BROWSER_INTERACTION_LINK_URL"] ?? ""
    let response = applyProps()
    let props = response?["props"] as? NSDictionary
    let address = props?["address"] as? String ?? ""
    let pageTitle = props?["page-title"] as? String ?? ""
    if address == linkURL, pageTitle == "Venture link acceptance" {
      lastSurfaceHistoryEvent = nil
      guard performNativeSurfaceKey(keyCode: 123, modifiers: [.command]) else {
        writeInteractionResult(
          [
            "backend": "swiftui", "status": "error",
            "error": "content surface unavailable for Command-Left history",
          ],
          to: markerPath)
        return
      }
      DispatchQueue.main.asyncAfter(deadline: .now() + 0.25) { [weak self] in
        self?.verifySurfaceHistoryBack(
          startURL: startURL, targetURL: targetURL, linkURL: linkURL,
          markerPath: markerPath, remaining: 50)
      }
      return
    }
    guard remaining > 0 else {
      let pointer = lastSurfacePointerPoint.map { "\($0.x),\($0.y)" } ?? "unhandled"
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error", "address": address,
          "pageTitle": pageTitle, "surfacePoint": pointer,
          "error": "native surface link did not navigate",
        ],
        to: markerPath)
      return
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) { [weak self] in
      self?.verifySurfaceLink(
        startURL: startURL, targetURL: targetURL, markerPath: markerPath,
        remaining: remaining - 1)
    }
  }

  private func verifySurfaceHistoryBack(
    startURL: String, targetURL: String, linkURL: String, markerPath: String, remaining: Int
  ) {
    let response = applyProps()
    let props = response?["props"] as? NSDictionary
    let address = props?["address"] as? String ?? ""
    let pageTitle = props?["page-title"] as? String ?? ""
    if lastSurfaceHistoryEvent == "onBack", address == startURL,
      pageTitle == "Venture launch acceptance"
    {
      lastSurfaceHistoryEvent = nil
      guard performNativeSurfaceKey(keyCode: 124, modifiers: [.command]) else {
        writeInteractionResult(
          [
            "backend": "swiftui", "status": "error",
            "error": "content surface unavailable for Command-Right history",
          ],
          to: markerPath)
        return
      }
      DispatchQueue.main.asyncAfter(deadline: .now() + 0.25) { [weak self] in
        self?.verifySurfaceHistoryForward(
          startURL: startURL, targetURL: targetURL, linkURL: linkURL,
          markerPath: markerPath, remaining: 50)
      }
      return
    }
    guard remaining > 0 else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error", "address": address,
          "pageTitle": pageTitle,
          "error": "Command-Left did not navigate shared history",
        ],
        to: markerPath)
      return
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) { [weak self] in
      self?.verifySurfaceHistoryBack(
        startURL: startURL, targetURL: targetURL, linkURL: linkURL,
        markerPath: markerPath, remaining: remaining - 1)
    }
  }

  private func verifySurfaceHistoryForward(
    startURL: String, targetURL: String, linkURL: String, markerPath: String, remaining: Int
  ) {
    let response = applyProps()
    let props = response?["props"] as? NSDictionary
    let address = props?["address"] as? String ?? ""
    let pageTitle = props?["page-title"] as? String ?? ""
    if lastSurfaceHistoryEvent == "onForward", address == linkURL,
      pageTitle == "Venture link acceptance"
    {
      guard performNativeSurfaceResize() else {
        writeInteractionResult(
          [
            "backend": "swiftui", "status": "error",
            "error": "content surface unavailable for native resize",
          ],
          to: markerPath)
        return
      }
      DispatchQueue.main.asyncAfter(deadline: .now() + 0.25) { [weak self] in
        self?.verifySurfaceResize(
          startURL: startURL, targetURL: targetURL, linkURL: linkURL,
          markerPath: markerPath, remaining: 50)
      }
      return
    }
    guard remaining > 0 else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error", "address": address,
          "pageTitle": pageTitle,
          "error": "Command-Right did not navigate shared history",
        ],
        to: markerPath)
      return
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) { [weak self] in
      self?.verifySurfaceHistoryForward(
        startURL: startURL, targetURL: targetURL, linkURL: linkURL,
        markerPath: markerPath, remaining: remaining - 1)
    }
  }

  private func verifySurfaceResize(
    startURL: String, targetURL: String, linkURL: String, markerPath: String, remaining: Int
  ) {
    if lastSurfaceFocusState == "first-responder",
      let baseline = surfaceResizeBaseline, let resized = lastSurfaceResizeSize,
      let renderBaseline = surfaceRenderBaseline, let rendered = lastSurfaceRenderSize,
      (abs(resized.width - baseline.width) > 0.5
        || abs(resized.height - baseline.height) > 0.5),
      (abs(rendered.width - renderBaseline.width) > 0.5
        || abs(rendered.height - renderBaseline.height) > 0.5)
    {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "interacted",
          "controls": "back-forward-reload-home", "addressCommit": "native-return",
          "navigationState": "native-disabled-transitions",
          "surfaceWheel": "scroll",
          "surfaceFocus": "native",
          "surfaceKeyboard": "document-end", "surfaceHistory": "back-forward",
          "surfacePointer": "link",
          "surfaceResize": "native-reflow",
          "surfaceRepaint": "resized-frame",
          "reloadTitle": "Venture reload acceptance", "homeAddress": startURL,
          "targetAddress": targetURL, "linkAddress": linkURL,
          "pageTitle": "Venture link acceptance",
        ],
        to: markerPath)
      return
    }
    guard remaining > 0 else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error",
          "error": "native surface resize did not produce a resized shared frame",
        ],
        to: markerPath)
      return
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) { [weak self] in
      self?.verifySurfaceResize(
        startURL: startURL, targetURL: targetURL, linkURL: linkURL,
        markerPath: markerPath, remaining: remaining - 1)
    }
  }

  private func findEditableTextField(
    in object: NSObject, visited: inout Set<ObjectIdentifier>
  ) -> NSTextField? {
    let objectIdentifier = ObjectIdentifier(object)
    guard visited.insert(objectIdentifier).inserted else { return nil }
    if let textField = object as? NSTextField, textField.isEditable { return textField }
    for child in nativeChildren(of: object) {
      if let found = findEditableTextField(in: child, visited: &visited) {
        return found
      }
    }
    return nil
  }

  private func nativeToolbarPoint(identifier: String) -> (NSPoint, NSWindow)? {
    let position: CGFloat
    switch identifier {
    case "back-button": position = 0.12
    case "forward-button": position = 0.34
    case "home-button": position = 0.56
    case "reload-button": position = 0.78
    default: return nil
    }
    var visited = Set<ObjectIdentifier>()
    guard let address = findEditableTextField(in: NSApp, visited: &visited),
      let window = address.window, let contentView = window.contentView
    else { return nil }
    let addressFrame = address.convert(address.bounds, to: contentView)
    let leadingChromeWidth = addressFrame.minX - contentView.bounds.minX
    let point = contentView.convert(
      NSPoint(
        x: contentView.bounds.minX + leadingChromeWidth * position,
        y: addressFrame.midY),
      to: nil)
    return (point, window)
  }

  private func performNativeButtonClick(identifier: String) -> Bool {
    NSApp.activate(ignoringOtherApps: true)
    guard let (point, window) = nativeToolbarPoint(identifier: identifier) else { return false }
    // SwiftUI owns these buttons inside its hosting view. Target their stable Mosaic toolbar
    // positions relative to the real native address field, then send ordinary AppKit events.
    sendPrimaryClick(at: point, to: window)
    return true
  }

  private func performNativeAddressCommit(value: String) -> Bool {
    var visited = Set<ObjectIdentifier>()
    guard let address = findEditableTextField(in: NSApp, visited: &visited) else {
      return false
    }
    address.selectText(nil)
    guard let editor = address.currentEditor() as? NSTextView,
      let window = address.window
    else { return false }
    editor.selectAll(nil)
    editor.insertText(value, replacementRange: editor.selectedRange())
    NSApp.activate(ignoringOtherApps: true)
    guard window.makeFirstResponder(editor) else { return false }
    guard let event = NSEvent.keyEvent(
      with: .keyDown,
      location: .zero,
      modifierFlags: [],
      timestamp: ProcessInfo.processInfo.systemUptime,
      windowNumber: window.windowNumber,
      context: nil,
      characters: "\r",
      charactersIgnoringModifiers: "\r",
      isARepeat: false,
      keyCode: 36)
    else { return false }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
      editor.interpretKeyEvents([event])
    }
    return true
  }

  private func performNativeSurfaceKey(
    keyCode: UInt16, modifiers: NSEvent.ModifierFlags = []
  ) -> Bool {
    guard let contentView, let window = contentView.window else { return false }
    NSApp.activate(ignoringOtherApps: true)
    guard focusNativeSurface(),
      let event = NSEvent.keyEvent(
        with: .keyDown,
        location: .zero,
        modifierFlags: modifiers,
        timestamp: ProcessInfo.processInfo.systemUptime,
        windowNumber: window.windowNumber,
        context: nil,
        characters: "",
        charactersIgnoringModifiers: "",
        isARepeat: false,
        keyCode: keyCode)
    else { return false }
    if modifiers.isEmpty {
      NSApp.sendEvent(event)
    } else {
      // Command-key events are consumed by AppKit's key-equivalent pass before the
      // first responder sees them. Deliver the real NSEvent to the production
      // content-surface override so this gate exercises its shortcut reducer.
      contentView.keyDown(with: event)
    }
    return true
  }

  private func performNativeSurfaceWheel() -> Bool {
    guard let contentView, let window = contentView.window,
      focusNativeSurface(),
      let cgEvent = CGEvent(
        scrollWheelEvent2Source: nil, units: .line, wheelCount: 1,
        wheel1: -3, wheel2: 0, wheel3: 0),
      let event = NSEvent(cgEvent: cgEvent)
    else { return false }
    NSApp.activate(ignoringOtherApps: true)
    lastSurfaceWheelDelta = nil
    contentView.scrollWheel(with: event)
    return true
  }

  private func focusNativeSurface() -> Bool {
    guard let contentView, let window = contentView.window,
      window.makeFirstResponder(contentView), window.firstResponder === contentView
    else { return false }
    lastSurfaceFocusState = "first-responder"
    return true
  }

  private func performNativeSurfaceClick(at point: NSPoint) -> Bool {
    guard let contentView, let window = contentView.window else { return false }
    NSApp.activate(ignoringOtherApps: true)
    let location = contentView.convert(point, to: nil)
    guard
      let mouseDown = primaryMouseEvent(type: .leftMouseDown, at: location, in: window),
      let mouseUp = primaryMouseEvent(type: .leftMouseUp, at: location, in: window)
    else { return false }
    contentView.mouseDown(with: mouseDown)
    contentView.mouseUp(with: mouseUp)
    return true
  }

  private func performNativeSurfaceResize() -> Bool {
    guard let contentView, let window = contentView.window else { return false }
    let baseline = contentView.bounds.size
    guard let layer = contentView.layer as? CAMetalLayer else { return false }
    let renderBaseline = layer.drawableSize
    guard baseline.width > 0, baseline.height > 0,
      renderBaseline.width > 0, renderBaseline.height > 0
    else { return false }
    surfaceResizeBaseline = baseline
    lastSurfaceResizeSize = nil
    surfaceRenderBaseline = renderBaseline
    lastSurfaceRenderSize = nil
    let contentSize = window.contentView?.bounds.size ?? window.frame.size
    window.setContentSize(
      NSSize(width: contentSize.width + 80, height: contentSize.height + 60))
    window.contentView?.needsLayout = true
    window.contentView?.layoutSubtreeIfNeeded()
    return true
  }

  private func sendPrimaryClick(at location: NSPoint, to window: NSWindow) {
    for eventType in [NSEvent.EventType.leftMouseDown, .leftMouseUp] {
      guard let event = primaryMouseEvent(type: eventType, at: location, in: window)
      else { continue }
      NSApp.sendEvent(event)
    }
  }

  private func primaryMouseEvent(
    type: NSEvent.EventType, at location: NSPoint, in window: NSWindow
  ) -> NSEvent? {
    NSEvent.mouseEvent(
      with: type,
      location: location,
      modifierFlags: [],
      timestamp: ProcessInfo.processInfo.systemUptime,
      windowNumber: window.windowNumber,
      context: nil,
      eventNumber: 0,
      clickCount: 1,
      pressure: type == .leftMouseDown ? 1 : 0)
  }

  private func nativeChildren(of object: NSObject) -> [NSObject] {
    var children: [NSObject] = []
    if let application = object as? NSApplication {
      children.append(contentsOf: application.windows)
    } else if let window = object as? NSWindow, let contentView = window.contentView {
      children.append(contentView)
    } else if let view = object as? NSView {
      children.append(contentsOf: view.subviews)
    }
    return children
  }

  private func writeInteractionResult(_ result: [String: String], to path: String) {
    guard let data = try? JSONSerialization.data(withJSONObject: result) else { return }
    try? data.write(to: URL(fileURLWithPath: path), options: .atomic)
  }

  fileprivate func render(layer: CAMetalLayer) {
    guard let native, let browser else { return }
    let rawLayer = Unmanaged.passUnretained(layer).toOpaque()
    guard native.render(browser, rawLayer) != 0 else { return }
    lastSurfaceRenderSize = layer.drawableSize
    reportAcceptanceIfRequested()
  }

  private func reportAcceptanceIfRequested() {
    guard !acceptanceReported else { return }
    guard let path = ProcessInfo.processInfo.environment["VENTURE_BROWSER_ACCEPTANCE_PATH"]
    else { return }
    acceptanceReported = true
    let result = ["backend": "swiftui", "status": "ready"]
    guard let data = try? JSONSerialization.data(withJSONObject: result) else { return }
    try? data.write(to: URL(fileURLWithPath: path), options: .atomic)
  }

  fileprivate func scroll(by deltaY: Double) {
    guard let native, let browser, native.scroll(browser, deltaY) != 0 else { return }
    lastSurfaceWheelDelta = deltaY
    contentView?.renderPage()
  }

  fileprivate func scroll(command: String) {
    guard let native, let browser else { return }
    let changed = command.withCString { native.scrollCommand(browser, $0) }
    guard changed != 0 else { return }
    lastSurfaceKeyboardCommand = command
    contentView?.renderPage()
  }

  fileprivate func navigateHistory(eventName: String) {
    lastSurfaceHistoryEvent = eventName
    _ = handleEvent([:], name: eventName as NSString)
    propsChangedHandler?()
  }

  fileprivate func activateLink(at point: NSPoint) {
    lastSurfacePointerPoint = point
    guard let native, let browser, native.activateLink(browser, point.x, point.y) != 0 else {
      return
    }
    contentView?.renderPage()
    propsChangedHandler?()
  }

  fileprivate func resize(width: Double, height: Double) {
    guard let native, let browser else { return }
    if native.resize(browser, width, height) != 0 {
      lastSurfaceResizeSize = NSSize(width: width, height: height)
    }
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
    guard bounds.width > 0, bounds.height > 0 else { return }
    host?.resize(width: bounds.width, height: bounds.height)
    let scale = layer.contentsScale > 0 ? layer.contentsScale : 1
    layer.drawableSize = CGSize(
      width: max(1, bounds.width * scale),
      height: max(1, bounds.height * scale)
    )
    host?.render(layer: layer)
  }
}
