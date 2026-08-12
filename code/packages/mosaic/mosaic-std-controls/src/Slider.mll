layout Slider {
  Column [ slider-root ] {
    Row [ slider-heading ] {
      Text [ slider-label ] ( content: slot: label )
      Spacer
      Text [ slider-value ] (
        content: slot: display-value,
        a11y-hidden: true
      )
    }
    HostSlider [ slider-control ] (
      a11y-label: slot: label,
      value: slot: value,
      min: slot: min,
      max: slot: max,
      step: slot: step,
      disabled: slot: disabled,
      onChange: emit: onChange,
      onCommit: emit: onCommit
    )
  }
}
