layout Checkbox {
  pkg::mosaic-pkg-toolkit::Checkbox [ checkbox-root ] (
    label: slot: label,
    checked: slot: checked,
    disabled: slot: disabled,
    indeterminate: false,
    onChange: emit: onChange
  )
}
