### Fixed — TaskApp calendar columns reach Flutter (#17289)

Flutter now lowers valid percentage widths on bounded direct `Row` children to
proportional flex shares and lowers wrapping rows to native `Wrap` widgets with
fractionally sized children. TaskApp's seven weekday headers and 42 day cells
preserve their authored seven-column geometry, while invalid or unbounded
percentages remain visible in degradation reports.
