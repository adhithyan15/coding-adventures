# One inline curriculum extension cannot own lessons from two path segments

An extension listed in a path segment's `inline` array may contain only lessons
owned by that same segment. A chapter can still cross two spine nodes, but its
lessons need separate path segments and separate inline extensions. Chain the
second extension to the first when both are required; do not use one umbrella
extension whose lesson list reaches across segment ownership.
