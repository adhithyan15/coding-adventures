# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-01

### Added

- Objective-C runtime bindings: `objc_getClass`, `sel_registerName`, `objc_msgSend`
- Runtime class creation: `objc_allocateClassPair`, `class_addMethod`, `class_addIvar`
- Metal framework bindings: `MTLCreateSystemDefaultDevice`, Metal types and constants
- CoreGraphics bindings: bitmap context creation, color space, rect fill
- CoreText bindings: font creation, line layout, text drawing
- CoreFoundation bindings: CFString, CFDictionary, CFAttributedString
- AppKit constants for window creation
- Safe wrappers: `class()`, `sel()`, `cfstring()`, `nsstring()`, `alloc_init()`, `release()`, `retain()`
