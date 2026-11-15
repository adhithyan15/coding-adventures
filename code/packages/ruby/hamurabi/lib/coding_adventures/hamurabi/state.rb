# frozen_string_literal: true

module CodingAdventures
  module Hamurabi
    # Immutable value record capturing the complete simulation state.
    class HamurabiState
      attr_reader :year, :population, :bushels, :acres, :land_price,
                  :harvest_per_acre, :rats_eaten, :starved_last_year,
                  :starved_total, :percent_starved_sum, :immigrants_last_year,
                  :plague_struck, :status, :rng_state, :rng_inc

      def initialize(
        year:,
        population:,
        bushels:,
        acres:,
        land_price:,
        harvest_per_acre:,
        rats_eaten:,
        starved_last_year:,
        starved_total:,
        percent_starved_sum:,
        immigrants_last_year:,
        plague_struck:,
        status:,
        rng_state:,
        rng_inc:
      )
        @year = year
        @population = population
        @bushels = bushels
        @acres = acres
        @land_price = land_price
        @harvest_per_acre = harvest_per_acre
        @rats_eaten = rats_eaten
        @starved_last_year = starved_last_year
        @starved_total = starved_total
        @percent_starved_sum = percent_starved_sum
        @immigrants_last_year = immigrants_last_year
        @plague_struck = !(!plague_struck)
        @status = status.is_a?(GameStatus) ? status : GameStatus.parse(status)
        @rng_state = rng_state
        @rng_inc = rng_inc
        freeze
      end

      def [](key)
        case key.to_s
        when "year" then @year
        when "population" then @population
        when "bushels" then @bushels
        when "acres" then @acres
        when "land_price" then @land_price
        when "harvest_per_acre" then @harvest_per_acre
        when "rats_eaten" then @rats_eaten
        when "starved_last_year" then @starved_last_year
        when "starved_total" then @starved_total
        when "percent_starved_sum" then @percent_starved_sum
        when "immigrants_last_year" then @immigrants_last_year
        when "plague_struck" then @plague_struck
        when "status" then @status
        when "rng_state" then @rng_state
        when "rng_inc" then @rng_inc
        end
      end

      def to_h
        {
          year: @year,
          population: @population,
          bushels: @bushels,
          acres: @acres,
          land_price: @land_price,
          harvest_per_acre: @harvest_per_acre,
          rats_eaten: @rats_eaten,
          starved_last_year: @starved_last_year,
          starved_total: @starved_total,
          percent_starved_sum: @percent_starved_sum,
          immigrants_last_year: @immigrants_last_year,
          plague_struck: @plague_struck,
          status: @status.to_s,
          rng_state: @rng_state,
          rng_inc: @rng_inc
        }
      end

      def ==(other)
        return false unless other.is_a?(HamurabiState)
        @year == other.year &&
          @population == other.population &&
          @bushels == other.bushels &&
          @acres == other.acres &&
          @land_price == other.land_price &&
          @harvest_per_acre == other.harvest_per_acre &&
          @rats_eaten == other.rats_eaten &&
          @starved_last_year == other.starved_last_year &&
          @starved_total == other.starved_total &&
          @percent_starved_sum == other.percent_starved_sum &&
          @immigrants_last_year == other.immigrants_last_year &&
          @plague_struck == other.plague_struck &&
          @status == other.status &&
          @rng_state == other.rng_state &&
          @rng_inc == other.rng_inc
      end

      def self.from_h(h)
        return h if h.is_a?(HamurabiState)
        new(
          year: h[:year] || h["year"],
          population: h[:population] || h["population"],
          bushels: h[:bushels] || h["bushels"],
          acres: h[:acres] || h["acres"],
          land_price: h[:land_price] || h["land_price"],
          harvest_per_acre: h[:harvest_per_acre] || h["harvest_per_acre"],
          rats_eaten: h[:rats_eaten] || h["rats_eaten"],
          starved_last_year: h[:starved_last_year] || h["starved_last_year"],
          starved_total: h[:starved_total] || h["starved_total"],
          percent_starved_sum: h[:percent_starved_sum] || h["percent_starved_sum"],
          immigrants_last_year: h[:immigrants_last_year] || h["immigrants_last_year"],
          plague_struck: h[:plague_struck] || h["plague_struck"] || false,
          status: h[:status] || h["status"],
          rng_state: h[:rng_state] || h["rng_state"],
          rng_inc: h[:rng_inc] || h["rng_inc"]
        )
      end
    end
  end
end
