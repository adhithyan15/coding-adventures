- Package-owned SwiftUI and WinUI content surfaces report native logical size
  changes through matching Rust resize ABIs, reflowing the retained document
  and repainting without refetching the page or duplicating chrome.
