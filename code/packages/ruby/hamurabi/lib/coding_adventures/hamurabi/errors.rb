# frozen_string_literal: true

module CodingAdventures
  module Hamurabi
    # Base class for all Hamurabi simulation errors.
    class HamurabiError < StandardError
      attr_reader :code

      def initialize(msg = nil, code = nil)
        @code = code || self.class.default_code
        super(msg || self.class.default_message)
      end

      def self.default_code
        "HamurabiError"
      end

      def self.default_message
        "Hamurabi simulation error"
      end

      def self.symbolic_name
        :hamurabi_error
      end

      def ==(other)
        if other.is_a?(String)
          @code == other || to_s == other
        elsif other.is_a?(Symbol)
          @code.to_sym == other || self.class.symbolic_name == other
        elsif other.is_a?(Class)
          is_a?(other)
        else
          super
        end
      end
    end

    # Arm 1: Attempted to execute order when game has already ended.
    class GameAlreadyEndedError < HamurabiError
      def self.default_code; "GameAlreadyEnded"; end
      def self.default_message; "Game has already ended"; end
      def self.symbolic_name; :game_already_ended; end
    end
    GameAlreadyEnded = GameAlreadyEndedError

    # Arm 2: Arithmetic overflow or invalid bounds on input parameters / state invariant.
    class InvalidOrderBoundsError < HamurabiError
      def self.default_code; "InvalidOrderBounds"; end
      def self.default_message; "Order parameters exceed valid numerical bounds or overflow"; end
      def self.symbolic_name; :invalid_order_bounds; end
    end
    InvalidOrderBounds = InvalidOrderBoundsError

    # Arm 3: Player ordered to buy land but lacks sufficient bushels in store.
    class InsufficientBushelsForLandError < HamurabiError
      def self.default_code; "InsufficientBushelsForLand"; end
      def self.default_message; "Insufficient bushels to buy specified acreage"; end
      def self.symbolic_name; :insufficient_bushels_for_land; end
    end
    InsufficientBushelsForLand = InsufficientBushelsForLandError

    # Arm 4: Player ordered to sell more land than the city owns.
    class InsufficientAcresToSellError < HamurabiError
      def self.default_code; "InsufficientAcresToSell"; end
      def self.default_message; "Cannot sell more acres than the city owns"; end
      def self.symbolic_name; :insufficient_acres_to_sell; end
    end
    InsufficientAcresToSell = InsufficientAcresToSellError

    # Arm 5: Player ordered to feed more bushels than available after land trade.
    class InsufficientBushelsToFeedError < HamurabiError
      def self.default_code; "InsufficientBushelsToFeed"; end
      def self.default_message; "Insufficient bushels in store to feed population"; end
      def self.symbolic_name; :insufficient_bushels_to_feed; end
    end
    InsufficientBushelsToFeed = InsufficientBushelsToFeedError

    # Arm 6: Player ordered to plant more acres than owned after land trade.
    class InsufficientAcresToPlantError < HamurabiError
      def self.default_code; "InsufficientAcresToPlant"; end
      def self.default_message; "Cannot plant more acres than owned after land trade"; end
      def self.symbolic_name; :insufficient_acres_to_plant; end
    end
    InsufficientAcresToPlant = InsufficientAcresToPlantError

    # Arm 7: Player ordered to plant more acres than population can tend (1 person tends 10 acres).
    class InsufficientPopulationToPlantError < HamurabiError
      def self.default_code; "InsufficientPopulationToPlant"; end
      def self.default_message; "Insufficient population to tend planted acreage"; end
      def self.symbolic_name; :insufficient_population_to_plant; end
    end
    InsufficientPopulationToPlant = InsufficientPopulationToPlantError

    # Arm 8: Player lacks grain to seed fields (1 bushel seeds 2 acres).
    class InsufficientGrainForSeedError < HamurabiError
      def self.default_code; "InsufficientGrainForSeed"; end
      def self.default_message; "Insufficient grain to seed planted acreage"; end
      def self.symbolic_name; :insufficient_grain_for_seed; end
    end
    InsufficientGrainForSeed = InsufficientGrainForSeedError
  end
end
