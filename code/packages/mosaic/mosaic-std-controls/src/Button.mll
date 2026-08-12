layout Button {
  pkg::mosaic-pkg-toolkit::Button [ button-root ] (
    label: slot: label,
    variant: "primary",
    size: "md",
    disabled: slot: disabled,
    onClick: emit: onClick
  )
}
