/// A bounded retained-tail decoder for arbitrarily fragmented RESP2 streams.
///
/// Each feed evaluates a private candidate buffer and commits state only after
/// parsing succeeds. Repeated incomplete work is charged, so adversarial
/// one-byte feeds cannot create unbounded re-scanning.
public struct RespDecoder: Sendable {
  public let limits: RespLimits
  private var buffer: [UInt8]
  private var fragmentedWork: Int64

  public init(limits: RespLimits = .default) {
    self.limits = limits
    buffer = []
    fragmentedWork = 0
  }

  public var pendingBytes: [UInt8] { buffer }
  public var pendingByteCount: Int { buffer.count }

  /// Add one owned fragment and return every newly complete value.
  /// On error, neither the prior tail nor its work accounting changes.
  public mutating func feed(_ fragment: [UInt8]) throws -> [RespValue] {
    let (candidateCount, countOverflow) = buffer.count.addingReportingOverflow(fragment.count)
    guard !countOverflow && candidateCount <= limits.maximumBufferedBytes else {
      throw RespProtocolError.bufferedByteLimitExceeded(maximum: limits.maximumBufferedBytes)
    }
    let (candidateWork, workOverflow) = fragmentedWork.addingReportingOverflow(
      Int64(candidateCount))
    guard !workOverflow && candidateWork <= limits.maximumFragmentedWork else {
      throw RespProtocolError.fragmentedWorkLimitExceeded(
        maximum: limits.maximumFragmentedWork)
    }

    var candidate = buffer
    candidate.append(contentsOf: fragment)
    let batch = try RESP.decodeAll(candidate, limits: limits)
    let retained = Array(candidate.dropFirst(batch.consumed))
    buffer = retained
    fragmentedWork = retained.isEmpty ? 0 : candidateWork
    return batch.values
  }

  /// Require the stream to end on a frame boundary.
  public func finish() throws {
    guard buffer.isEmpty else { throw RespProtocolError.incompleteFrame }
  }

  public mutating func reset() {
    buffer.removeAll(keepingCapacity: false)
    fragmentedWork = 0
  }
}
