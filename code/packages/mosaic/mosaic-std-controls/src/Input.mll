layout Input {
  pkg::mosaic-pkg-toolkit::Input [ input-root ] (
    value: slot: value,
    placeholder: slot: placeholder,
    disabled: slot: disabled,
    size: "md",
    onChange: emit: onChange,
    onCommit: emit: onCommit
  )
}
