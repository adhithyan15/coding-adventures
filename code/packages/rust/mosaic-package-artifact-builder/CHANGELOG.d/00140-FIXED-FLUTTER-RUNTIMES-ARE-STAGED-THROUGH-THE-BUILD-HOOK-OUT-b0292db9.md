### Fixed — Flutter runtimes are staged through the build-hook output

Generated Flutter build hooks now copy the selected prebuilt Mosaic runtime
from the package's `runtime/` directory into `input.outputDirectory` before
registering it as a `DynamicLoadingBundled` code asset. This follows Flutter's
code-asset contract. TaskApp's CI and release workflows also move from Flutter
3.44.0 to the locally verified 3.47.0 toolchain and explicitly enable native
assets, instead of inheriting machine-global Flutter configuration. When
Flutter leaves the registered runtime in its native-assets staging tree, the
TaskApp CI and release lanes now finish installing that exact staged file into
the application bundle before byte and launch validation. This restores
`libmosaic_app.so` in Linux application bundles on fresh hosted runner images
(#14249).

