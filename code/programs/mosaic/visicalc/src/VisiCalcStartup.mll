layout VisiCalcStartup {
  Column [surface] {
    Column [card] {
      Text [eyebrow] (content: "THE WORKBOOK")
      Text [brand] (content: "VisiCalc")
      Row [paper] {
        Box [paper-first] { }
        Box [paper-second] { }
        Box [paper-third] { }
      }
      Text [heading] (content: slot: heading, a11y-role: heading)
      Text [message] (content: slot: message)
      If (when: slot: can-retry) {
        HostButton [retry-button] (label: "Try again", onClick: emit: onRetry)
      }
    }
    Text [signature] (content: "A little room for big ideas.")
  }
}
