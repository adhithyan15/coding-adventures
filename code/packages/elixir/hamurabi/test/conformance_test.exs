defmodule CodingAdventures.Hamurabi.ConformanceTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Hamurabi
  alias CodingAdventures.Hamurabi.{State, TurnOrder}

  @fixture Path.expand("../../../../specs/fixtures/hamurabi-v1/cases.json", __DIR__)

  # ── Minimal Zero-Dependency JSON Decoder ──────────────────────────────────
  # Guarantees arbitrary-precision integer decoding without 53-bit float loss.
  defmodule JSONDecoder do
    def decode!(bin) do
      if Code.ensure_loaded?(JSON) and function_exported?(JSON, :decode!, 1) do
        apply(JSON, :decode!, [bin])
      else
        {val, rest} = parse_value(skip_ws(bin))
        "" = skip_ws(rest)
        val
      end
    end

    defp parse_value("{" <> rest), do: parse_object(skip_ws(rest), %{})
    defp parse_value("[" <> rest), do: parse_array(skip_ws(rest), [])
    defp parse_value("\"" <> rest), do: parse_string(rest, [])
    defp parse_value("true" <> rest), do: {true, rest}
    defp parse_value("false" <> rest), do: {false, rest}
    defp parse_value("null" <> rest), do: {nil, rest}

    defp parse_value(<<c, _::binary>> = bin) when c in ?0..?9 or c == ?- do
      parse_number(bin)
    end

    defp parse_object("}" <> rest, acc), do: {acc, rest}

    defp parse_object(bin, acc) do
      {key, rest1} = parse_value(skip_ws(bin))
      ":" <> rest2 = skip_ws(rest1)
      {val, rest3} = parse_value(skip_ws(rest2))
      new_acc = Map.put(acc, key, val)

      case skip_ws(rest3) do
        "," <> rest4 -> parse_object(skip_ws(rest4), new_acc)
        "}" <> rest4 -> {new_acc, rest4}
      end
    end

    defp parse_array("]" <> rest, acc), do: {Enum.reverse(acc), rest}

    defp parse_array(bin, acc) do
      {val, rest1} = parse_value(skip_ws(bin))

      case skip_ws(rest1) do
        "," <> rest2 -> parse_array(skip_ws(rest2), [val | acc])
        "]" <> rest2 -> {Enum.reverse([val | acc]), rest2}
      end
    end

    defp parse_string("\"" <> rest, acc) do
      {IO.iodata_to_binary(Enum.reverse(acc)), rest}
    end

    defp parse_string("\\\"" <> rest, acc), do: parse_string(rest, [?\" | acc])
    defp parse_string("\\\\" <> rest, acc), do: parse_string(rest, [?\\ | acc])
    defp parse_string("\\n" <> rest, acc), do: parse_string(rest, [?\n | acc])
    defp parse_string("\\r" <> rest, acc), do: parse_string(rest, [?\r | acc])
    defp parse_string("\\t" <> rest, acc), do: parse_string(rest, [?\t | acc])
    defp parse_string(<<c, rest::binary>>, acc), do: parse_string(rest, [c | acc])

    defp parse_number(bin) do
      {num_str, rest} = take_number_chars(bin, [])

      if String.contains?(num_str, ".") or String.contains?(num_str, "e") or
           String.contains?(num_str, "E") do
        {String.to_float(num_str), rest}
      else
        {String.to_integer(num_str), rest}
      end
    end

    defp take_number_chars(<<c, rest::binary>>, acc)
         when c in ?0..?9 or c in [?-, ?+, ?., ?e, ?E] do
      take_number_chars(rest, [c | acc])
    end

    defp take_number_chars(rest, acc) do
      {IO.iodata_to_binary(Enum.reverse(acc)), rest}
    end

    defp skip_ws(<<c, rest::binary>>) when c in [?\s, ?\t, ?\n, ?\r], do: skip_ws(rest)
    defp skip_ws(bin), do: bin
  end

  # Convert JSON state map into State struct
  defp json_to_state(map) do
    {:ok, status} = Hamurabi.status_from_string(map["status"])

    %State{
      year: map["year"],
      population: map["population"],
      bushels: map["bushels"],
      acres: map["acres"],
      land_price: map["land_price"],
      harvest_per_acre: map["harvest_per_acre"],
      rats_eaten: map["rats_eaten"],
      starved_last_year: map["starved_last_year"],
      starved_total: map["starved_total"],
      percent_starved_sum: map["percent_starved_sum"],
      immigrants_last_year: map["immigrants_last_year"],
      plague_struck: map["plague_struck"],
      status: status,
      rng_state: map["rng_state"],
      rng_inc: map["rng_inc"]
    }
  end

  defp json_to_order(map) do
    {:ok, order} = TurnOrder.new(map)
    order
  end

  test "verifies all 31 cases in hamurabi-v1 cases.json corpus" do
    assert File.exists?(@fixture), "Missing cases.json fixture at #{@fixture}"

    json_text = File.read!(@fixture)
    data = JSONDecoder.decode!(json_text)
    cases = data["cases"]
    assert length(cases) == 31

    for tc <- cases do
      id = tc["id"]

      cond do
        tc["expected_error"] != nil ->
          expected_err_str = tc["expected_error"]
          init_state = json_to_state(tc["initial_state"])
          order = json_to_order(tc["order"])

          assert {:error, reason} = Hamurabi.validate_order(init_state, order),
                 "#{id}: validate_order should fail with #{expected_err_str}"

          assert Hamurabi.error_to_string(reason) == expected_err_str,
                 "#{id}: expected #{expected_err_str}, got #{inspect(reason)}"

          assert {:error, step_reason} = Hamurabi.step_year(init_state, order),
                 "#{id}: step_year should fail with #{expected_err_str}"

          assert Hamurabi.error_to_string(step_reason) == expected_err_str,
                 "#{id}: expected #{expected_err_str}, got #{inspect(step_reason)}"

        tc["turns"] != nil ->
          # Verify initial state matches new_game(seed)
          seed = tc["seed"]
          init_from_game = Hamurabi.new_game(seed)
          expected_init = tc["initial_state"]

          assert init_from_game.year == expected_init["year"], "#{id}: init year"
          assert init_from_game.population == expected_init["population"], "#{id}: init pop"
          assert init_from_game.bushels == expected_init["bushels"], "#{id}: init bushels"
          assert init_from_game.acres == expected_init["acres"], "#{id}: init acres"
          assert init_from_game.land_price == expected_init["land_price"], "#{id}: init price"

          assert init_from_game.harvest_per_acre == expected_init["harvest_per_acre"],
                 "#{id}: init harvest"

          assert init_from_game.rats_eaten == expected_init["rats_eaten"], "#{id}: init rats"

          assert init_from_game.starved_last_year == expected_init["starved_last_year"],
                 "#{id}: init starved"

          assert init_from_game.starved_total == expected_init["starved_total"],
                 "#{id}: init starved total"

          assert init_from_game.percent_starved_sum == expected_init["percent_starved_sum"],
                 "#{id}: init pct sum"

          assert init_from_game.immigrants_last_year == expected_init["immigrants_last_year"],
                 "#{id}: init imm"

          assert init_from_game.plague_struck == expected_init["plague_struck"],
                 "#{id}: init plague"

          assert Hamurabi.status_to_string(init_from_game.status) == expected_init["status"],
                 "#{id}: init status"

          assert init_from_game.rng_state == expected_init["rng_state"], "#{id}: init rng_state"
          assert init_from_game.rng_inc == expected_init["rng_inc"], "#{id}: init rng_inc"

          # Play through each turn
          Enum.reduce(tc["turns"], json_to_state(expected_init), fn t, cur_state ->
            turn_num = t["turn"]
            order = json_to_order(t["order"])
            expected_next = t["expected_state"]
            expected_report = t["expected_report"]

            assert :ok == Hamurabi.validate_order(cur_state, order),
                   "#{id} turn #{turn_num}: validate_order failed"

            assert {:ok, next_state} = Hamurabi.step_year(cur_state, order),
                   "#{id} turn #{turn_num}: step_year failed"

            assert next_state.year == expected_next["year"], "#{id} turn #{turn_num}: year"

            assert next_state.population == expected_next["population"],
                   "#{id} turn #{turn_num}: pop"

            assert next_state.bushels == expected_next["bushels"],
                   "#{id} turn #{turn_num}: bushels"

            assert next_state.acres == expected_next["acres"], "#{id} turn #{turn_num}: acres"

            assert next_state.land_price == expected_next["land_price"],
                   "#{id} turn #{turn_num}: price"

            assert next_state.harvest_per_acre == expected_next["harvest_per_acre"],
                   "#{id} turn #{turn_num}: harvest"

            assert next_state.rats_eaten == expected_next["rats_eaten"],
                   "#{id} turn #{turn_num}: rats"

            assert next_state.starved_last_year == expected_next["starved_last_year"],
                   "#{id} turn #{turn_num}: starved"

            assert next_state.starved_total == expected_next["starved_total"],
                   "#{id} turn #{turn_num}: starved total"

            assert next_state.percent_starved_sum == expected_next["percent_starved_sum"],
                   "#{id} turn #{turn_num}: pct sum"

            assert next_state.immigrants_last_year == expected_next["immigrants_last_year"],
                   "#{id} turn #{turn_num}: imm"

            assert next_state.plague_struck == expected_next["plague_struck"],
                   "#{id} turn #{turn_num}: plague"

            assert Hamurabi.status_to_string(next_state.status) == expected_next["status"],
                   "#{id} turn #{turn_num}: status"

            assert next_state.rng_state == expected_next["rng_state"],
                   "#{id} turn #{turn_num}: rng_state"

            assert next_state.rng_inc == expected_next["rng_inc"],
                   "#{id} turn #{turn_num}: rng_inc"

            report = Hamurabi.format_report(next_state)

            assert report == expected_report,
                   "#{id} turn #{turn_num}: report mismatch\nGot:\n#{report}\nExpected:\n#{expected_report}"

            # Continue to next turn
            next_state
          end)

        true ->
          flunk("#{id}: unknown case shape")
      end
    end
  end
end
