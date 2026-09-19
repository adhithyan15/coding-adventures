## Unreleased - CLR02 explicit encoded int64 values

Adapt the scalar encoded CLR test helper to explicitly refuse an unexpected
Int64 return. Simulator and direct builder int64 support do not yet enable
wide IIR literals or host input; existing CLR01 refusal remains covered.
