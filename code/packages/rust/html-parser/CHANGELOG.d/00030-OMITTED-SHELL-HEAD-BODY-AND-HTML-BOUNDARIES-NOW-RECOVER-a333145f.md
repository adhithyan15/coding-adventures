- Omitted-shell `</head>`, `</body>`, and `</html>` boundaries now recover
  without noisy unmatched-end diagnostics by closing the current lightweight
  body-content stack before subsequent text or element siblings are appended.
