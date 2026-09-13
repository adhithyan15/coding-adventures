# Fix backend-specific Mosaic measurement in the emitter, not with shared pixel widths

Giving TaskApp's progress group a 90px Compose-safe bound kept its caption visible there but made Flutter's real widget test fail with a 58px `RenderFlex` overflow, and widening it still changed SwiftUI launch behavior. Preserve the cross-platform `.msl`; thread the host layout scope through the affected emitter instead (Compose Row children stay intrinsic, while `width: 100%`/`flex-grow` lower to `RowScope.weight`), then rerun every strict native lifecycle that consumes the shared source.
