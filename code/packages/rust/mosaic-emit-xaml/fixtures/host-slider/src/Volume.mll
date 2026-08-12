layout Volume {
  HostSlider [ volume ] (
    value: slot: value,
    min: 0,
    max: 100,
    step: 5,
    disabled: slot: disabled,
    onChange: emit: onChange,
    onCommit: emit: onCommit
  )
}
