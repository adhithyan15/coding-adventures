- Added an opaque caller-timed RFC 8628 polling sequence that rejects early and
  expired polls before request preparation or transport, performs at most one
  audited effect per step, preserves provider/client/endpoint/trace binding,
  applies cumulative `slow_down`, and reschedules transient transport failures
  without acquiring clock or sleep authority.
