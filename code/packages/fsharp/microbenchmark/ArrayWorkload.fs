namespace CodingAdventures.Microbenchmark

open System
open System.Diagnostics
open CodingAdventures.RankStatistics

/// Fixed array experiment and runtime clock used by the F# command.
module ArrayWorkload =
    let length = 4096
    let checksum = 511533L
    let workloadId = "indexed-array-sum-4096"

    /// Constructs the input before the collector begins any timed batches.
    let createArray () = Array.init length (fun index -> (17 * index + 3) % 251)

    /// Sums every element through indexed access for each operation.
    let indexedBatch (values: int array) operations =
        if operations < 0L then raise (ArgumentOutOfRangeException("operations", "operations must be nonnegative"))
        let mutable total = 0L
        for _operation in 1L .. operations do
            for index in 0 .. values.Length - 1 do
                total <- Checked.(+) total (int64 values[index])
        total

    /// Sums every element through native F# sequence iteration.
    let iterationBatch (values: int array) operations =
        if operations < 0L then raise (ArgumentOutOfRangeException("operations", "operations must be nonnegative"))
        let mutable total = 0L
        for _operation in 1L .. operations do
            for value in values do
                total <- Checked.(+) total (int64 value)
        total

    /// Creates a monotonic elapsed nanosecond clock from Stopwatch ticks.
    let createClock () =
        let start = Stopwatch.GetTimestamp()
        fun () ->
            try Checked.(*) (Stopwatch.GetElapsedTime(start).Ticks) 100L
            with :? OverflowException -> -1L

    /// Runs the fixed paired workload, retaining only portable CLI metadata.
    let run config invocation revision profile clock =
        let values = createArray()
        if values.Length <> length || values |> Array.sumBy int64 <> checksum then
            Error(InvalidObservation "indexed-array checksum definition changed")
        else
            let baseline count = Ok(indexedBatch values count)
            let candidate count = Ok(iterationBatch values count)
            match Microbenchmark.collect config checksum baseline candidate clock with
            | Error error -> Error error
            | Ok report ->
                let metadata =
                    { workload = workloadId
                      invocation = invocation
                      revision = revision
                      runtime = System.Runtime.InteropServices.RuntimeInformation.FrameworkDescription
                      os = System.Runtime.InteropServices.RuntimeInformation.OSDescription
                      arch = System.Runtime.InteropServices.RuntimeInformation.ProcessArchitecture.ToString().ToLowerInvariant()
                      profile = Some profile }
                Ok { report with metadata = Some metadata }
