# layout-positioned

`layout-positioned` owns the host-neutral `ext["positioned"]` contract used by
computed CSS, shared layout, paint, clipping, and hit testing. It resolves
relative and out-of-flow insets, stable z-index ordering, overflow policy, and
scroll extents without depending on an HTML parser or a rendering toolkit.
