defmodule MicrobenchmarkConsumer do
  alias CodingAdventures.{ArrayWorkload, RankStatistics}

  def run do
    values = ArrayWorkload.make_values()
    with {:ok, 2.0} <- RankStatistics.median([1.0, 3.0]),
         511_533 <- ArrayWorkload.indexed_batch(values, 1),
         511_533 <- ArrayWorkload.iterator_batch(values, 1) do
      :ok
    else
      other -> {:error, other}
    end
  end
end
