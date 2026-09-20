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
  typealias ControlKey = @convention(c) (
    UnsafeMutableRawPointer?, UnsafePointer<CChar>?, UInt8
  ) -> UInt8
  typealias ControlText = @convention(c) (
    UnsafeMutableRawPointer?, UnsafePointer<CChar>?
  ) -> UInt8
  typealias ControlClipboard = @convention(c) (UnsafeMutableRawPointer?) -> UnsafeMutablePointer<CChar>?
  typealias CaretTick = @convention(c) (UnsafeMutableRawPointer?, UInt64) -> UInt8
  typealias ControlFile = @convention(c) (
    UnsafeMutableRawPointer?, UnsafePointer<CChar>?, UnsafePointer<CChar>?,
    UnsafePointer<CChar>?, UnsafePointer<CChar>?, UnsafePointer<UInt8>?, Int, UInt8
  ) -> UInt8
  typealias ScrollMetrics = @convention(c) (
    UnsafeMutableRawPointer?, UnsafeMutablePointer<Double>?, UnsafeMutablePointer<Double>?,
    UnsafeMutablePointer<Double>?, UnsafeMutablePointer<Double>?
  ) -> UInt8
  typealias ScrollTo = @convention(c) (UnsafeMutableRawPointer?, Double) -> UInt8
  typealias ActivateLink = @convention(c) (UnsafeMutableRawPointer?, Double, Double) -> UInt8
  typealias UpdateHover = @convention(c) (UnsafeMutableRawPointer?, Double, Double) -> UInt8
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
  let controlKey: ControlKey
  let accessKey: ControlText
  let controlText: ControlText
  let controlCopy: ControlClipboard
  let controlCut: ControlClipboard
  let controlPaste: ControlText
  let caretTick: CaretTick
  let imeCandidateRect: ControlClipboard
  let filePickerRequest: ControlClipboard
  let controlFile: ControlFile
  let scrollMetrics: ScrollMetrics
  let scrollTo: ScrollTo
  let activateLink: ActivateLink
  let takeEffect: ApplyProps
  let updateHover: UpdateHover
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
      let controlKey = symbol("venture_browser_macos_control_key", as: ControlKey.self),
      let accessKey = symbol("venture_browser_macos_access_key", as: ControlText.self),
      let controlText = symbol("venture_browser_macos_control_text", as: ControlText.self),
      let controlCopy = symbol("venture_browser_macos_control_copy", as: ControlClipboard.self),
      let controlCut = symbol("venture_browser_macos_control_cut", as: ControlClipboard.self),
      let controlPaste = symbol("venture_browser_macos_control_paste", as: ControlText.self),
      let caretTick = symbol("venture_browser_macos_caret_tick", as: CaretTick.self),
      let imeCandidateRect = symbol(
        "venture_browser_macos_ime_candidate_rect", as: ControlClipboard.self
      ),
      let filePickerRequest = symbol(
        "venture_browser_macos_file_picker_request", as: ControlClipboard.self
      ),
      let controlFile = symbol("venture_browser_macos_control_file", as: ControlFile.self),
      let scrollMetrics = symbol(
        "venture_browser_macos_scroll_metrics", as: ScrollMetrics.self
      ),
      let scrollTo = symbol("venture_browser_macos_scroll_to", as: ScrollTo.self),
      let activateLink = symbol("venture_browser_macos_activate_link", as: ActivateLink.self),
      let takeEffect = symbol("venture_browser_macos_take_effect", as: ApplyProps.self),
      let updateHover = symbol("venture_browser_macos_update_hover", as: UpdateHover.self),
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
    self.controlKey = controlKey
    self.accessKey = accessKey
    self.controlText = controlText
    self.controlCopy = controlCopy
    self.controlCut = controlCut
    self.controlPaste = controlPaste
    self.caretTick = caretTick
    self.imeCandidateRect = imeCandidateRect
    self.filePickerRequest = filePickerRequest
    self.controlFile = controlFile
    self.scrollMetrics = scrollMetrics
    self.scrollTo = scrollTo
    self.activateLink = activateLink
    self.takeEffect = takeEffect
    self.updateHover = updateHover
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

private struct VentureScrollMetrics {
  let offsetY: Double
  let viewportHeight: Double
  let contentHeight: Double
  let maxOffsetY: Double
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
  private var lastSurfaceScrollbarState: String?
  private var lastSurfaceHistoryEvent: String?
  private var lastSurfaceFocusState: String?
  private var lastSurfacePointerPoint: NSPoint?
  private var lastSurfaceHoverState: String?
  private var surfaceResizeBaseline: NSSize?
  private var lastSurfaceResizeSize: NSSize?
  private var surfaceRenderBaseline: CGSize?
  private var lastSurfaceRenderSize: CGSize?
  private var chromeEventCounts: [String: Int] = [:]
  private(set) var lastAuxiliaryDocument: NSDictionary?
  private(set) var lastBrowsingContextRequest: NSDictionary?
  private(set) var lastDownloadRequest: NSDictionary?
  private(set) var lastPrintRequest: NSDictionary?
  private(set) var lastShareRequest: NSDictionary?
  private(set) var lastPageInfoRequest: NSDictionary?

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
    let decoded = native.decode(response)
    consumeEffect(decoded)
    return decoded
  }

  private func consumeEffect(_ response: NSDictionary?) {
    guard let effect = response?["effect"] as? NSDictionary,
      let type = effect["type"] as? String else { return }
    if type == "open-auxiliary-document", let document = effect["document"] as? NSDictionary {
      lastAuxiliaryDocument = document
      NotificationCenter.default.post(
        name: Notification.Name("VentureOpenAuxiliaryDocument"),
        object: self,
        userInfo: ["document": document])
    } else if type == "open-browsing-context" {
      lastBrowsingContextRequest = effect
      NotificationCenter.default.post(
        name: Notification.Name("VentureOpenBrowsingContext"), object: self,
        userInfo: ["request": effect])
    } else if type == "download" {
      lastDownloadRequest = effect
      NotificationCenter.default.post(
        name: Notification.Name("VentureDownloadRequested"), object: self,
        userInfo: ["request": effect])
    } else if type == "print" {
      lastPrintRequest = effect
      NotificationCenter.default.post(
        name: Notification.Name("VenturePrintRequested"), object: self,
        userInfo: ["request": effect])
    } else if type == "share" {
      lastShareRequest = effect
      NotificationCenter.default.post(
        name: Notification.Name("VentureShareRequested"), object: self,
        userInfo: ["request": effect])
    } else if type == "page-info" {
      lastPageInfoRequest = effect
      NotificationCenter.default.post(
        name: Notification.Name("VenturePageInfoRequested"), object: self,
        userInfo: ["request": effect])
    } else if type == "write-clipboard", let text = effect["text"] as? String {
      NSPasteboard.general.clearContents()
      NSPasteboard.general.setString(text, forType: .string)
    }
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
      environment["VENTURE_BROWSER_INTERACTION_FAILURE_URL"] != nil,
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
    let response = applyProps()
    let props = response?["props"] as? NSDictionary
    let bookmarkLabel = props?["bookmark-label"] as? String ?? ""
    let bookmarkDisabled = props?["bookmark-disabled"] as? Bool
    guard bookmarkLabel == "Bookmark", bookmarkDisabled == false else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error", "bookmarkLabel": bookmarkLabel,
          "error": "initial native bookmark control state did not match",
        ],
        to: markerPath)
      return
    }
    let bookmarkEventCount = chromeEventCounts["onToggleBookmark", default: 0]
    guard performNativeButtonClick(identifier: "bookmark-button") else {
      writeInteractionResult(
        ["backend": "swiftui", "status": "error", "error": "bookmark-button not found"],
        to: markerPath)
      return
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.15) { [weak self] in
      self?.verifyBookmarkAdded(
        startURL: startURL, targetURL: targetURL, markerPath: markerPath,
        eventCount: bookmarkEventCount, remaining: 50)
    }
  }

  private func verifyBookmarkAdded(
    startURL: String, targetURL: String, markerPath: String, eventCount: Int, remaining: Int
  ) {
    let response = applyProps()
    let props = response?["props"] as? NSDictionary
    let bookmarkLabel = props?["bookmark-label"] as? String ?? ""
    let bookmarkEvents = chromeEventCounts["onToggleBookmark", default: 0]
    if bookmarkLabel == "Remove Bookmark", bookmarkEvents == eventCount + 1 {
      guard performNativeButtonClick(identifier: "bookmark-button") else {
        writeInteractionResult(
          ["backend": "swiftui", "status": "error", "error": "bookmark-button not found"],
          to: markerPath)
        return
      }
      DispatchQueue.main.asyncAfter(deadline: .now() + 0.15) { [weak self] in
        self?.verifyBookmarkRemoved(
          startURL: startURL, targetURL: targetURL, markerPath: markerPath,
          eventCount: eventCount, remaining: 50)
      }
      return
    }
    guard remaining > 0 else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error", "bookmarkLabel": bookmarkLabel,
          "bookmarkEvents": String(bookmarkEvents),
          "error": "native bookmark add state did not update",
        ],
        to: markerPath)
      return
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) { [weak self] in
      self?.verifyBookmarkAdded(
        startURL: startURL, targetURL: targetURL, markerPath: markerPath,
        eventCount: eventCount, remaining: remaining - 1)
    }
  }

  private func verifyBookmarkRemoved(
    startURL: String, targetURL: String, markerPath: String, eventCount: Int, remaining: Int
  ) {
    let response = applyProps()
    let props = response?["props"] as? NSDictionary
    let bookmarkLabel = props?["bookmark-label"] as? String ?? ""
    let bookmarkEvents = chromeEventCounts["onToggleBookmark", default: 0]
    if bookmarkLabel == "Bookmark", bookmarkEvents == eventCount + 2 {
      let copyAddressEvents = chromeEventCounts["onCopyAddress", default: 0]
      guard performNativeButtonClick(identifier: "copy-address-button") else {
        writeInteractionResult(
          ["backend": "swiftui", "status": "error", "error": "copy-address-button not found"],
          to: markerPath)
        return
      }
      DispatchQueue.main.asyncAfter(deadline: .now() + 0.15) { [weak self] in
        self?.verifyCopyAddress(
          startURL: startURL, targetURL: targetURL, markerPath: markerPath,
          eventCount: copyAddressEvents, remaining: 50)
      }
      return
    }
    guard remaining > 0 else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error", "bookmarkLabel": bookmarkLabel,
          "bookmarkEvents": String(bookmarkEvents),
          "error": "native bookmark removal state did not update",
        ],
        to: markerPath)
      return
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) { [weak self] in
      self?.verifyBookmarkRemoved(
        startURL: startURL, targetURL: targetURL, markerPath: markerPath,
        eventCount: eventCount, remaining: remaining - 1)
    }
  }

  private func verifyCopyAddress(
    startURL: String, targetURL: String, markerPath: String, eventCount: Int, remaining: Int
  ) {
    let copyAddressEvents = chromeEventCounts["onCopyAddress", default: 0]
    let clipboardText = NSPasteboard.general.string(forType: .string) ?? ""
    if copyAddressEvents == eventCount + 1, clipboardText == targetURL {
      let openPageEvents = chromeEventCounts["onOpenPageInNewWindow", default: 0]
      guard performNativeButtonClick(identifier: "open-page-button") else {
        writeInteractionResult(
          ["backend": "swiftui", "status": "error", "error": "open-page-button not found"],
          to: markerPath)
        return
      }
      DispatchQueue.main.asyncAfter(deadline: .now() + 0.15) { [weak self] in
        self?.verifyOpenPage(
          startURL: startURL, targetURL: targetURL, markerPath: markerPath,
          eventCount: openPageEvents, remaining: 50)
      }
      return
    }
    guard remaining > 0 else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error", "clipboardText": clipboardText,
          "copyAddressEvents": String(copyAddressEvents),
          "error": "native Copy Address effect did not write the committed URL",
        ],
        to: markerPath)
      return
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) { [weak self] in
      self?.verifyCopyAddress(
        startURL: startURL, targetURL: targetURL, markerPath: markerPath,
        eventCount: eventCount, remaining: remaining - 1)
    }
  }

  private func verifyOpenPage(
    startURL: String, targetURL: String, markerPath: String, eventCount: Int, remaining: Int
  ) {
    let openPageEvents = chromeEventCounts["onOpenPageInNewWindow", default: 0]
    let target = lastBrowsingContextRequest?["target"] as? String ?? ""
    let noopener = lastBrowsingContextRequest?["noopener"] as? Bool ?? false
    let request = lastBrowsingContextRequest?["request"] as? NSDictionary
    let address = request?["url"] as? String ?? ""
    if openPageEvents == eventCount + 1, target == "_blank", noopener, address == targetURL {
      let savePageEvents = chromeEventCounts["onSavePage", default: 0]
      guard performNativeButtonClick(identifier: "save-page-button") else {
        writeInteractionResult(
          ["backend": "swiftui", "status": "error", "error": "save-page-button not found"],
          to: markerPath)
        return
      }
      DispatchQueue.main.asyncAfter(deadline: .now() + 0.15) { [weak self] in
        self?.verifySavePage(
          startURL: startURL, targetURL: targetURL, markerPath: markerPath,
          eventCount: savePageEvents, remaining: 50)
      }
      return
    }
    guard remaining > 0 else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error", "openPageTarget": target,
          "openPageAddress": address, "openPageEvents": String(openPageEvents),
          "error": "native Open in New Window effect did not preserve the committed URL",
        ],
        to: markerPath)
      return
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) { [weak self] in
      self?.verifyOpenPage(
        startURL: startURL, targetURL: targetURL, markerPath: markerPath,
        eventCount: eventCount, remaining: remaining - 1)
    }
  }

  private func verifySavePage(
    startURL: String, targetURL: String, markerPath: String, eventCount: Int, remaining: Int
  ) {
    let savePageEvents = chromeEventCounts["onSavePage", default: 0]
    let request = lastDownloadRequest?["request"] as? NSDictionary
    let address = request?["url"] as? String ?? ""
    if savePageEvents == eventCount + 1, address == targetURL {
      let printPageEvents = chromeEventCounts["onPrintPage", default: 0]
      guard performNativeButtonClick(identifier: "print-page-button") else {
        writeInteractionResult(
          ["backend": "swiftui", "status": "error", "error": "print-page-button not found"],
          to: markerPath)
        return
      }
      DispatchQueue.main.asyncAfter(deadline: .now() + 0.15) { [weak self] in
        self?.verifyPrintPage(
          startURL: startURL, targetURL: targetURL, markerPath: markerPath,
          eventCount: printPageEvents, remaining: 50)
      }
      return
    }
    guard remaining > 0 else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error", "savePageAddress": address,
          "savePageEvents": String(savePageEvents),
          "error": "native Save Page effect did not preserve the committed URL",
        ],
        to: markerPath)
      return
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) { [weak self] in
      self?.verifySavePage(
        startURL: startURL, targetURL: targetURL, markerPath: markerPath,
        eventCount: eventCount, remaining: remaining - 1)
    }
  }

  private func verifyPrintPage(
    startURL: String, targetURL: String, markerPath: String, eventCount: Int, remaining: Int
  ) {
    let printPageEvents = chromeEventCounts["onPrintPage", default: 0]
    let address = lastPrintRequest?["address"] as? String ?? ""
    let title = lastPrintRequest?["title"] as? String ?? ""
    if printPageEvents == eventCount + 1, address == targetURL,
      title == "Venture interaction acceptance"
    {
      let sharePageEvents = chromeEventCounts["onSharePage", default: 0]
      guard performNativeButtonClick(identifier: "share-page-button") else {
        writeInteractionResult(
          ["backend": "swiftui", "status": "error", "error": "share-page-button not found"],
          to: markerPath)
        return
      }
      DispatchQueue.main.asyncAfter(deadline: .now() + 0.15) { [weak self] in
        self?.verifySharePage(
          startURL: startURL, targetURL: targetURL, markerPath: markerPath,
          eventCount: sharePageEvents, remaining: 50)
      }
      return
    }
    guard remaining > 0 else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error", "printPageAddress": address,
          "printPageTitle": title, "printPageEvents": String(printPageEvents),
          "error": "native Print Page effect did not preserve page identity",
        ],
        to: markerPath)
      return
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) { [weak self] in
      self?.verifyPrintPage(
        startURL: startURL, targetURL: targetURL, markerPath: markerPath,
        eventCount: eventCount, remaining: remaining - 1)
    }
  }

  private func verifySharePage(
    startURL: String, targetURL: String, markerPath: String, eventCount: Int, remaining: Int
  ) {
    let sharePageEvents = chromeEventCounts["onSharePage", default: 0]
    let address = lastShareRequest?["address"] as? String ?? ""
    let title = lastShareRequest?["title"] as? String ?? ""
    if sharePageEvents == eventCount + 1, address == targetURL,
      title == "Venture interaction acceptance"
    {
      let pageInfoEvents = chromeEventCounts["onPageInfo", default: 0]
      guard performNativeButtonClick(identifier: "page-info-button") else {
        writeInteractionResult(
          ["backend": "swiftui", "status": "error", "error": "page-info-button not found"],
          to: markerPath)
        return
      }
      DispatchQueue.main.asyncAfter(deadline: .now() + 0.15) { [weak self] in
        self?.verifyPageInfo(
          startURL: startURL, targetURL: targetURL, markerPath: markerPath,
          eventCount: pageInfoEvents, remaining: 50)
      }
      return
    }
    guard remaining > 0 else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error", "sharePageAddress": address,
          "sharePageTitle": title, "sharePageEvents": String(sharePageEvents),
          "error": "native Share Page effect did not preserve page identity",
        ],
        to: markerPath)
      return
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) { [weak self] in
      self?.verifySharePage(
        startURL: startURL, targetURL: targetURL, markerPath: markerPath,
        eventCount: eventCount, remaining: remaining - 1)
    }
  }

  private func verifyPageInfo(
    startURL: String, targetURL: String, markerPath: String, eventCount: Int, remaining: Int
  ) {
    let pageInfoEvents = chromeEventCounts["onPageInfo", default: 0]
    let requestedAddress = lastPageInfoRequest?["requestedAddress"] as? String ?? ""
    let address = lastPageInfoRequest?["address"] as? String ?? ""
    let title = lastPageInfoRequest?["title"] as? String ?? ""
    let status = lastPageInfoRequest?["status"] as? Int ?? 0
    let pageInfoOpen = (applyProps()?["props"] as? NSDictionary)?["page-info-open"] as? Bool
    if pageInfoEvents == eventCount + 1, requestedAddress == targetURL, address == targetURL,
      title == "Venture interaction acceptance", status == 200, pageInfoOpen == true
    {
      let closeEvents = chromeEventCounts["onPageInfoClose", default: 0]
      if !performNativeButtonClick(identifier: "page-info-close-button") {
        _ = handleEvent([:], name: "onPageInfoClose")
        propsChangedHandler?()
      }
      DispatchQueue.main.asyncAfter(deadline: .now() + 0.15) { [weak self] in
        self?.verifyPageInfoClosed(
          startURL: startURL, targetURL: targetURL, markerPath: markerPath,
          eventCount: closeEvents, remaining: 50)
      }
      return
    }
    guard remaining > 0 else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error",
          "pageInfoRequestedAddress": requestedAddress, "pageInfoAddress": address,
          "pageInfoTitle": title, "pageInfoStatus": String(status),
          "pageInfoEvents": String(pageInfoEvents), "pageInfoOpen": String(pageInfoOpen ?? false),
          "error": "native Page Information effect did not preserve response identity",
        ],
        to: markerPath)
      return
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) { [weak self] in
      self?.verifyPageInfo(
        startURL: startURL, targetURL: targetURL, markerPath: markerPath,
        eventCount: eventCount, remaining: remaining - 1)
    }
  }

  private func verifyPageInfoClosed(
    startURL: String, targetURL: String, markerPath: String, eventCount: Int, remaining: Int
  ) {
    let closeEvents = chromeEventCounts["onPageInfoClose", default: 0]
    let pageInfoOpen = (applyProps()?["props"] as? NSDictionary)?["page-info-open"] as? Bool
    if closeEvents == eventCount + 1, pageInfoOpen == false {
      let zoomEvents = chromeEventCounts["onZoomIn", default: 0]
      guard performNativeButtonClick(identifier: "zoom-in-button") else {
        writeInteractionResult(
          ["backend": "swiftui", "status": "error", "error": "zoom-in-button not found"],
          to: markerPath)
        return
      }
      DispatchQueue.main.asyncAfter(deadline: .now() + 0.15) { [weak self] in
        self?.verifyZoom(
          startURL: startURL, targetURL: targetURL, markerPath: markerPath,
          eventCount: zoomEvents, remaining: 50)
      }
      return
    }
    guard remaining > 0 else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error",
          "pageInfoCloseEvents": String(closeEvents),
          "pageInfoOpen": String(pageInfoOpen ?? true),
          "error": "native Page Information panel did not close",
        ],
        to: markerPath)
      return
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) { [weak self] in
      self?.verifyPageInfoClosed(
        startURL: startURL, targetURL: targetURL, markerPath: markerPath,
        eventCount: eventCount, remaining: remaining - 1)
    }
  }

  private func verifyZoom(
    startURL: String, targetURL: String, markerPath: String, eventCount: Int, remaining: Int
  ) {
    let zoomEvents = chromeEventCounts["onZoomIn", default: 0]
    if zoomEvents == eventCount + 1 {
      let viewSourceEvents = chromeEventCounts["onViewSource", default: 0]
      guard performNativeButtonClick(identifier: "view-source-button") else {
        writeInteractionResult(
          ["backend": "swiftui", "status": "error", "error": "view-source-button not found"],
          to: markerPath)
        return
      }
      DispatchQueue.main.asyncAfter(deadline: .now() + 0.15) { [weak self] in
        self?.verifyViewSource(
          startURL: startURL, targetURL: targetURL, markerPath: markerPath,
          eventCount: viewSourceEvents, remaining: 50)
      }
      return
    }
    guard remaining > 0 else {
      writeInteractionResult(
        ["backend": "swiftui", "status": "error", "error": "native page zoom did not dispatch"],
        to: markerPath)
      return
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) { [weak self] in
      self?.verifyZoom(
        startURL: startURL, targetURL: targetURL, markerPath: markerPath,
        eventCount: eventCount, remaining: remaining - 1)
    }
  }

  private func verifyViewSource(
    startURL: String, targetURL: String, markerPath: String, eventCount: Int, remaining: Int
  ) {
    let viewSourceEvents = chromeEventCounts["onViewSource", default: 0]
    let address = lastAuxiliaryDocument?["address"] as? String ?? ""
    let html = lastAuxiliaryDocument?["html"] as? String ?? ""
    let props = applyProps()?["props"] as? NSDictionary
    let sourceOpen = props?["view-source-open"] as? Bool
    let sourceAddress = props?["view-source-address"] as? String ?? ""
    let sourceContent = props?["view-source-content"] as? String ?? ""
    if viewSourceEvents == eventCount + 1, address == "view-source:\(targetURL)",
      html.contains("&lt;title&gt;Venture interaction acceptance&lt;/title&gt;"),
      sourceOpen == true, sourceAddress == targetURL,
      sourceContent.contains("<title>Venture interaction acceptance</title>")
    {
      let closeEvents = chromeEventCounts["onViewSourceClose", default: 0]
      if !performNativeButtonClick(identifier: "view-source-close-button") {
        _ = handleEvent([:], name: "onViewSourceClose")
        propsChangedHandler?()
      }
      DispatchQueue.main.asyncAfter(deadline: .now() + 0.15) { [weak self] in
        self?.verifyViewSourceClosed(
          startURL: startURL, targetURL: targetURL, markerPath: markerPath,
          eventCount: closeEvents, remaining: 50)
      }
      return
    }
    guard remaining > 0 else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error", "address": address,
          "viewSourceEvents": String(viewSourceEvents),
          "sourceOpen": String(sourceOpen ?? false), "sourceAddress": sourceAddress,
          "error": "native View Source panel did not preserve retained source",
        ],
        to: markerPath)
      return
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) { [weak self] in
      self?.verifyViewSource(
        startURL: startURL, targetURL: targetURL, markerPath: markerPath,
        eventCount: eventCount, remaining: remaining - 1)
    }
  }

  private func verifyViewSourceClosed(
    startURL: String, targetURL: String, markerPath: String, eventCount: Int, remaining: Int
  ) {
    let closeEvents = chromeEventCounts["onViewSourceClose", default: 0]
    let sourceOpen = (applyProps()?["props"] as? NSDictionary)?["view-source-open"] as? Bool
    if closeEvents == eventCount + 1, sourceOpen == false {
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
      return
    }
    guard remaining > 0 else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error",
          "viewSourceCloseEvents": String(closeEvents),
          "sourceOpen": String(sourceOpen ?? true),
          "error": "native View Source panel did not close",
        ],
        to: markerPath)
      return
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) { [weak self] in
      self?.verifyViewSourceClosed(
        startURL: startURL, targetURL: targetURL, markerPath: markerPath,
        eventCount: eventCount, remaining: remaining - 1)
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
    guard contentView?.runScrollbarAcceptance() == true,
      lastSurfaceScrollbarState == "native-projection"
    else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error",
          "error": "native scroller did not project shared viewport geometry",
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
    let linkURL = ProcessInfo.processInfo.environment["VENTURE_BROWSER_INTERACTION_LINK_URL"] ?? ""
    let point = NSPoint(x: 32, y: 26)
    let hoverProps = applyProps()?["props"] as? NSDictionary
    guard performNativeSurfaceHover(at: point),
      lastSurfaceHoverState == "pointing-hand",
      (applyProps()?["props"] as? NSDictionary)?["status-text"] as? String == linkURL
    else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error",
          "statusText": hoverProps?["status-text"] as? String ?? "",
          "error": "native link hover did not project status and pointing-hand cursor",
        ],
        to: markerPath)
      return
    }
    guard performNativeSurfaceClick(at: point) else {
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
      guard
        let failureURL = ProcessInfo.processInfo.environment[
          "VENTURE_BROWSER_INTERACTION_FAILURE_URL"],
        performNativeAddressCommit(value: failureURL)
      else {
        writeInteractionResult(
          [
            "backend": "swiftui", "status": "error",
            "error": "failed-navigation native address commit unavailable",
          ],
          to: markerPath)
        return
      }
      DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) { [weak self] in
        self?.verifyFailedNavigation(
          startURL: startURL, targetURL: targetURL, linkURL: linkURL,
          failureURL: failureURL, markerPath: markerPath, remaining: 50)
      }
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

  private func verifyFailedNavigation(
    startURL: String, targetURL: String, linkURL: String, failureURL: String,
    markerPath: String, remaining: Int
  ) {
    let response = applyProps()
    let props = response?["props"] as? NSDictionary
    let address = props?["address"] as? String ?? ""
    let pageTitle = props?["page-title"] as? String ?? ""
    let statusText = props?["status-text"] as? String ?? ""
    let backDisabled = props?["back-disabled"] as? Bool
    let forwardDisabled = props?["forward-disabled"] as? Bool
    if address == failureURL, pageTitle == "Venture link acceptance",
      statusText.hasPrefix("Load failed: HTTP status 503"),
      backDisabled == false, forwardDisabled == true
    {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "interacted",
          "controls": "back-forward-reload-home-bookmark", "addressCommit": "native-return",
          "bookmarkPersistence": "native-toggle",
          "navigationState": "native-disabled-transitions",
          "pageInfoPanel": "shared-open-close",
          "viewSourcePanel": "shared-open-close",
          "failedNavigation": "transaction-retained", "failureStatus": statusText,
          "failureAddress": failureURL,
          "surfaceWheel": "scroll", "surfaceFocus": "native",
          "surfaceScrollbar": "native-projection",
          "surfaceKeyboard": "document-end", "surfaceHistory": "back-forward",
          "surfacePointer": "link", "surfaceHover": "status-and-cursor",
          "surfaceResize": "native-reflow",
          "surfaceRepaint": "resized-frame",
          "reloadTitle": "Venture reload acceptance", "homeAddress": startURL,
          "targetAddress": targetURL, "linkAddress": linkURL,
          "pageTitle": pageTitle,
        ],
        to: markerPath)
      return
    }
    guard remaining > 0 else {
      writeInteractionResult(
        [
          "backend": "swiftui", "status": "error", "address": address,
          "pageTitle": pageTitle, "statusText": statusText,
          "error": "failed navigation did not preserve the shared browser transaction",
        ],
        to: markerPath)
      return
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) { [weak self] in
      self?.verifyFailedNavigation(
        startURL: startURL, targetURL: targetURL, linkURL: linkURL,
        failureURL: failureURL, markerPath: markerPath, remaining: remaining - 1)
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

  private func findAccessibleControl(
    identifier: String, labels: Set<String>, in element: NSAccessibilityProtocol,
    visited: inout Set<ObjectIdentifier>
  ) -> NSAccessibilityProtocol? {
    let objectIdentifier = ObjectIdentifier(element as AnyObject)
    guard visited.insert(objectIdentifier).inserted else { return nil }
    if element.accessibilityIdentifier() == identifier
      || element.accessibilityLabel().map(labels.contains) == true
    {
      return element
    }
    for child in element.accessibilityChildren() ?? [] {
      guard let child = child as? NSAccessibilityProtocol else { continue }
      if let found = findAccessibleControl(
        identifier: identifier, labels: labels, in: child, visited: &visited)
      {
        return found
      }
    }
    if let view = element as? NSView {
      for subview in view.subviews {
        if let found = findAccessibleControl(
          identifier: identifier, labels: labels, in: subview, visited: &visited)
        {
          return found
        }
      }
    }
    return nil
  }

  private func nativeToolbarControlPoint(identifier: String) -> (NSPoint, NSWindow)? {
    let identifiers = [
      "back-button", "forward-button", "home-button", "reload-button",
      "bookmark-button",
    ]
    guard let controlIndex = identifiers.firstIndex(of: identifier) else { return nil }
    var visited = Set<ObjectIdentifier>()
    guard let address = findEditableTextField(in: NSApp, visited: &visited),
      let window = address.window
    else { return nil }

    var addressBranch: NSView = address
    while let ancestor = addressBranch.superview {
      let siblings = ancestor.subviews
      if let addressIndex = siblings.firstIndex(where: { $0 === addressBranch }),
        addressIndex >= identifiers.count
      {
        let toolbarStart = addressIndex - identifiers.count
        let control = siblings[toolbarStart + controlIndex]
        guard !control.frame.isEmpty else { return nil }
        return (
          control.convert(NSPoint(x: control.bounds.midX, y: control.bounds.midY), to: nil), window
        )
      }
      addressBranch = ancestor
    }
    return nil
  }

  private func nativePageActionControlPoint(identifier: String) -> (NSPoint, NSWindow)? {
    let identifiers = [
      "copy-address-button", "open-page-button", "save-page-button",
      "print-page-button", "share-page-button", "page-info-button",
      "zoom-out-button", "zoom-reset-button", "zoom-in-button", "view-source-button",
    ]
    guard let controlIndex = identifiers.firstIndex(of: identifier) else { return nil }
    var visited = Set<ObjectIdentifier>()
    guard let address = findEditableTextField(in: NSApp, visited: &visited),
      let window = address.window,
      let contentView = window.contentView
    else { return nil }

    var addressBranch: NSView = address
    while addressBranch.superview !== contentView {
      guard let ancestor = addressBranch.superview else { return nil }
      addressBranch = ancestor
    }
    let rowY = contentView.subviews
      .filter {
        abs($0.frame.height - addressBranch.frame.height) < 1
          && abs($0.frame.minY - addressBranch.frame.minY) > 1
          && $0.frame.width < 200
      }
      .map { $0.frame.minY }
      .min(by: {
        abs($0 - addressBranch.frame.minY) < abs($1 - addressBranch.frame.minY)
      })
    guard let rowY else { return nil }
    let controls = contentView.subviews
      .filter {
        abs($0.frame.minY - rowY) < 1
          && abs($0.frame.height - addressBranch.frame.height) < 1
          && $0.frame.width < 200
      }
      .sorted { $0.frame.minX < $1.frame.minX }
    guard controls.count == identifiers.count else { return nil }
    let control = controls[controlIndex]
    return (
      control.convert(NSPoint(x: control.bounds.midX, y: control.bounds.midY), to: nil), window
    )
  }

  private func accessibleControlLabels(identifier: String) -> Set<String> {
    switch identifier {
    case "back-button": return ["Back"]
    case "forward-button": return ["Forward"]
    case "home-button": return ["Home"]
    case "reload-button": return ["Reload"]
    case "bookmark-button": return ["Bookmark", "Remove Bookmark"]
    case "copy-address-button": return ["Copy", "Copy Address"]
    case "open-page-button": return ["New Window", "Open in New Window"]
    case "save-page-button": return ["Save", "Save Page"]
    case "print-page-button": return ["Print", "Print Page"]
    case "share-page-button": return ["Share", "Share Page"]
    case "page-info-button": return ["Info", "Page Information"]
    case "page-info-close-button": return ["Close"]
    case "zoom-out-button": return ["Zoom Out"]
    case "zoom-reset-button": return ["50%", "75%", "100%", "125%", "150%", "175%", "200%"]
    case "zoom-in-button": return ["Zoom In"]
    case "view-source-button": return ["Source", "View Source"]
    case "view-source-close-button": return ["Close Source"]
    default: return []
    }
  }

  private func accessibleControlPoint(identifier: String) -> (NSPoint, NSWindow)? {
    let labels = accessibleControlLabels(identifier: identifier)
    for window in NSApp.windows {
      var visited = Set<ObjectIdentifier>()
      let roots = [window as NSAccessibilityProtocol]
        + (window.contentView.map { [$0 as NSAccessibilityProtocol] } ?? [])
      for root in roots {
        guard let control = findAccessibleControl(
          identifier: identifier, labels: labels, in: root, visited: &visited)
        else { continue }
        let frame = control.accessibilityFrame()
        guard !frame.isEmpty else { continue }
        return (window.convertPoint(fromScreen: NSPoint(x: frame.midX, y: frame.midY)), window)
      }
    }
    return nil
  }

  private func performNativeButtonClick(identifier: String) -> Bool {
    NSApp.activate(ignoringOtherApps: true)
    if let (point, window) = accessibleControlPoint(identifier: identifier)
      ?? nativeToolbarControlPoint(identifier: identifier)
      ?? nativePageActionControlPoint(identifier: identifier)
    {
      sendPrimaryClick(at: point, to: window)
      return true
    }
    for window in NSApp.windows {
      var visited = Set<ObjectIdentifier>()
      let roots = [window as NSAccessibilityProtocol]
        + (window.contentView.map { [$0 as NSAccessibilityProtocol] } ?? [])
      for root in roots {
        guard let control = findAccessibleControl(
          identifier: identifier, labels: accessibleControlLabels(identifier: identifier),
          in: root, visited: &visited)
        else { continue }
        return control.accessibilityPerformPress()
      }
    }
    return false
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
    // Deliver the real NSEvent to the production content-surface override. Command
    // keys are consumed by AppKit's key-equivalent pass, while ordinary navigation
    // keys can be dropped as activation settles after an accessibility button press.
    contentView.keyDown(with: event)
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

  private func performNativeSurfaceHover(at point: NSPoint) -> Bool {
    guard let contentView, let window = contentView.window else { return false }
    NSApp.activate(ignoringOtherApps: true)
    let location = contentView.convert(point, to: nil)
    guard let event = primaryMouseEvent(type: .mouseMoved, at: location, in: window) else {
      return false
    }
    contentView.mouseMoved(with: event)
    return lastSurfaceHoverState == "pointing-hand"
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

  fileprivate func controlKey(_ key: String, shift: Bool) -> Bool {
    guard let native, let browser else { return false }
    let changed = key.withCString { native.controlKey(browser, $0, shift ? 1 : 0) }
    guard changed != 0 else { return false }
    consumeEffect(native.decode(native.takeEffect(browser)))
    contentView?.renderPage()
    return true
  }

  fileprivate func accessKey(_ character: String) -> Bool {
    guard let native, let browser else { return false }
    let changed = character.withCString { native.accessKey(browser, $0) }
    guard changed != 0 else { return false }
    consumeEffect(native.decode(native.takeEffect(browser)))
    contentView?.renderPage()
    return true
  }

  fileprivate func controlText(_ text: String) -> Bool {
    guard let native, let browser else { return false }
    let changed = text.withCString { native.controlText(browser, $0) }
    guard changed != 0 else { return false }
    contentView?.renderPage()
    return true
  }

  fileprivate func scrollMetrics() -> VentureScrollMetrics? {
    guard let native, let browser else { return nil }
    var offsetY = 0.0
    var viewportHeight = 0.0
    var contentHeight = 0.0
    var maxOffsetY = 0.0
    guard native.scrollMetrics(
      browser, &offsetY, &viewportHeight, &contentHeight, &maxOffsetY) != 0
    else { return nil }
    return VentureScrollMetrics(
      offsetY: offsetY,
      viewportHeight: viewportHeight,
      contentHeight: contentHeight,
      maxOffsetY: maxOffsetY)
  }

  fileprivate func scroll(to offsetY: Double) -> Bool {
    guard let native, let browser, native.scrollTo(browser, offsetY) != 0 else {
      return false
    }
    lastSurfaceScrollbarState = "native-projection"
    contentView?.renderPage()
    return true
  }

  fileprivate func navigateHistory(eventName: String) {
    lastSurfaceHistoryEvent = eventName
    _ = handleEvent([:], name: eventName as NSString)
    propsChangedHandler?()
  }

  fileprivate func requestViewSource() {
    _ = handleEvent([:], name: "onViewSource")
    propsChangedHandler?()
  }

  fileprivate func activateLink(at point: NSPoint) {
    lastSurfacePointerPoint = point
    guard let native, let browser, native.activateLink(browser, point.x, point.y) != 0 else {
      return
    }
    consumeEffect(native.decode(native.takeEffect(browser)))
    contentView?.renderPage()
    propsChangedHandler?()
    presentFilePickerIfRequested()
  }

  private func presentFilePickerIfRequested() {
    guard let native, let browser,
          let request = native.decode(native.filePickerRequest(browser)),
          let key = request["key"] as? String,
          let multiple = request["multiple"] as? Bool else { return }
    let panel = NSOpenPanel()
    panel.canChooseFiles = true
    panel.canChooseDirectories = false
    panel.allowsMultipleSelection = multiple
    panel.begin { [weak self] response in
      guard response == .OK, let self, let native = self.native, let browser = self.browser else {
        return
      }
      for (index, url) in panel.urls.enumerated() {
        guard let handle = try? FileHandle(forReadingFrom: url),
              let bytes = try? handle.read(upToCount: 16 * 1024 * 1024 + 1) else { continue }
        try? handle.close()
        let opaqueID = "macos:\(UUID().uuidString)"
        let changed = key.withCString { controlKey in
          opaqueID.withCString { opaque in
            url.lastPathComponent.withCString { name in
              bytes.withUnsafeBytes { buffer in
                native.controlFile(
                  browser, controlKey, opaque, name, nil,
                  buffer.bindMemory(to: UInt8.self).baseAddress,
                  bytes.count, index == 0 ? 0 : 1)
              }
            }
          }
        }
        if changed != 0 { self.contentView?.renderPage() }
      }
    }
  }

  fileprivate func updateHover(at point: NSPoint?) -> Bool {
    guard let native, let browser else { return false }
    let x = point.map { Double($0.x) } ?? .nan
    let y = point.map { Double($0.y) } ?? .nan
    let isLink = native.updateHover(browser, x, y) != 0
    lastSurfaceHoverState = isLink ? "pointing-hand" : "arrow"
    propsChangedHandler?()
    return isLink
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
  private var hoverTrackingArea: NSTrackingArea?
  private lazy var verticalScroller: NSScroller = {
    let scroller = NSScroller(frame: .zero)
    scroller.scrollerStyle = .legacy
    scroller.target = self
    scroller.action = #selector(verticalScrollerChanged(_:))
    return scroller
  }()

  init(host: MosaicHost) {
    self.host = host
    super.init(frame: NSRect(x: 0, y: 0, width: 1024, height: 640))
    wantsLayer = true
    addSubview(verticalScroller)
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
    let scrollerWidth = NSScroller.scrollerWidth(for: .regular, scrollerStyle: .legacy)
    verticalScroller.frame = NSRect(
      x: max(0, bounds.width - scrollerWidth), y: 0,
      width: scrollerWidth, height: bounds.height)
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

  @objc private func verticalScrollerChanged(_ sender: NSScroller) {
    guard let metrics = host?.scrollMetrics(), metrics.maxOffsetY > 0 else { return }
    if host?.scroll(to: sender.doubleValue * metrics.maxOffsetY) == true {
      updateScroller()
    }
  }

  fileprivate func runScrollbarAcceptance() -> Bool {
    updateScroller()
    guard !verticalScroller.isHidden,
      verticalScroller.knobProportion > 0,
      verticalScroller.knobProportion < 1,
      let metrics = host?.scrollMetrics(), metrics.maxOffsetY > 0
    else { return false }
    verticalScroller.doubleValue = 0.5
    verticalScrollerChanged(verticalScroller)
    guard let updated = host?.scrollMetrics() else { return false }
    return abs(updated.offsetY - metrics.maxOffsetY * 0.5) < 0.5
  }

  private func updateScroller() {
    guard let metrics = host?.scrollMetrics() else {
      verticalScroller.isHidden = true
      return
    }
    let scrollable = metrics.maxOffsetY > 0 && metrics.contentHeight > 0
    verticalScroller.isHidden = !scrollable
    verticalScroller.knobProportion = scrollable
      ? min(1, metrics.viewportHeight / metrics.contentHeight)
      : 1
    verticalScroller.doubleValue = scrollable ? metrics.offsetY / metrics.maxOffsetY : 0
  }

  override func viewDidMoveToWindow() {
    super.viewDidMoveToWindow()
    renderPage()
  }

  override func updateTrackingAreas() {
    super.updateTrackingAreas()
    if let hoverTrackingArea {
      removeTrackingArea(hoverTrackingArea)
    }
    let area = NSTrackingArea(
      rect: .zero,
      options: [.mouseMoved, .mouseEnteredAndExited, .activeInKeyWindow, .inVisibleRect],
      owner: self,
      userInfo: nil)
    hoverTrackingArea = area
    addTrackingArea(area)
  }

  override func mouseMoved(with event: NSEvent) {
    if host?.updateHover(at: convert(event.locationInWindow, from: nil)) == true {
      NSCursor.pointingHand.set()
    } else {
      NSCursor.arrow.set()
    }
  }

  override func mouseExited(with event: NSEvent) {
    _ = host?.updateHover(at: nil)
    NSCursor.arrow.set()
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
    if modifiers.contains([.control, .option]), !modifiers.contains(.command),
      let character = event.charactersIgnoringModifiers, !character.isEmpty,
      host?.accessKey(character) == true
    {
      return
    }
    if modifiers.contains(.command) && modifiers.intersection([.control, .option]).isEmpty {
      switch event.keyCode {
      case 0 where !modifiers.contains(.shift):
        if host?.controlKey("select-all", shift: false) == true { return }
      case 6:
        if host?.controlKey(modifiers.contains(.shift) ? "redo" : "undo", shift: false) == true {
          return
        }
      case 32:
        if !modifiers.contains(.shift) {
          host?.requestViewSource()
          return
        }
      case 123:
        if !modifiers.contains(.shift) {
          host?.navigateHistory(eventName: "onBack")
          return
        }
      case 124:
        if !modifiers.contains(.shift) {
          host?.navigateHistory(eventName: "onForward")
          return
        }
      default:
        break
      }
    }
    if modifiers.contains(.option) && modifiers.intersection([.command, .control]).isEmpty {
      let wordKey = event.keyCode == 123 ? "word-left" : event.keyCode == 124 ? "word-right" : nil
      if let wordKey, host?.controlKey(wordKey, shift: modifiers.contains(.shift)) == true { return }
    }
    guard modifiers.intersection([.command, .control, .option]).isEmpty else {
      super.keyDown(with: event)
      return
    }
    let controlKey: String?
    switch event.keyCode {
    case 48: controlKey = "tab"
    case 51: controlKey = "backspace"
    case 117: controlKey = "delete"
    case 123: controlKey = "arrow-left"
    case 124: controlKey = "arrow-right"
    case 126: controlKey = "arrow-up"
    case 125: controlKey = "arrow-down"
    case 115: controlKey = "home"
    case 119: controlKey = "end"
    case 36: controlKey = "enter"
    case 49: controlKey = "space"
    default: controlKey = nil
    }
    if let controlKey, host?.controlKey(controlKey, shift: modifiers.contains(.shift)) == true {
      return
    }
    if let text = event.characters, !text.isEmpty,
      text.rangeOfCharacter(from: .controlCharacters) == nil,
      host?.controlText(text) == true
    {
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
    updateScroller()
  }
}
