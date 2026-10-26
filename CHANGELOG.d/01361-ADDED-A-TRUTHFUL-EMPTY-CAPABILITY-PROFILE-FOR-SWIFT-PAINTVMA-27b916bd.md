### Added a truthful empty capability profile for Swift PaintVmAscii

The Swift PaintVmAscii package now carries an explicit schema-v1 empty
capability manifest. Its documentation makes the authority boundary clear:
rendering consumes an in-memory scene and returns a string, while any terminal
output remains the caller's responsibility.
