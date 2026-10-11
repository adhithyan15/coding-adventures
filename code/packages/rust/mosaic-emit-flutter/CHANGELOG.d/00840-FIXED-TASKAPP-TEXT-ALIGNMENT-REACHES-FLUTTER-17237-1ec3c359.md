### Fixed — TaskApp text alignment reaches Flutter (#17237)

Supported `text-align` declarations on `Text` and `HostButton` parts now lower
through Flutter's native `Text.textAlign` and `ButtonStyle.alignment` APIs.
Absolute left/center/right values stay absolute, directional start/end values
follow the ambient text direction, and unsupported spellings remain visible in
the dropped-style report. Fresh TaskApp generation retires all 14 alignment
omissions and reduces Flutter's measured style inventory from 121 to 107.
