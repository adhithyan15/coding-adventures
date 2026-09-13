layout VisiCalcStartup {
  Column [surface] {
    Column [card] {
      Text [eyebrow] (font-size: slot: font-eyebrow, content: "THE WORKBOOK")
      Text [brand] (font-size: slot: font-brand, content: "VisiCalc")
      Row [paper] {
        Box [paper-first] { }
        Box [paper-second] { }
        Box [paper-third] { }
      }
      Text [heading] (font-size: slot: font-heading, content: slot: heading, a11y-role: heading)
      Text [message] (font-size: slot: font-input, content: slot: message)
      If (when: slot: can-retry) {
        HostButton [retry-button] (font-size: slot: font-input, label: "Try again", onClick: emit: onRetry)
      }
    }
    Text [signature] (font-size: slot: font-signature, content: "A little room for big ideas.")
  }
}
