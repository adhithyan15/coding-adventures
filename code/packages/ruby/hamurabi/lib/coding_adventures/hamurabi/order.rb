# frozen_string_literal: true

module CodingAdventures
  module Hamurabi
    # Player orders for a single year.
    class TurnOrder
      attr_reader :acres_delta, :bushels_to_feed, :acres_to_plant

      def initialize(acres_delta: 0, bushels_to_feed: 0, acres_to_plant: 0)
        @acres_delta = acres_delta
        @bushels_to_feed = bushels_to_feed
        @acres_to_plant = acres_to_plant
        freeze
      end

      def [](key)
        case key.to_s
        when "acres_delta" then @acres_delta
        when "bushels_to_feed" then @bushels_to_feed
        when "acres_to_plant" then @acres_to_plant
        end
      end

      def to_h
        {
          acres_delta: @acres_delta,
          bushels_to_feed: @bushels_to_feed,
          acres_to_plant: @acres_to_plant
        }
      end

      def ==(other)
        return false unless other.respond_to?(:acres_delta) &&
                            other.respond_to?(:bushels_to_feed) &&
                            other.respond_to?(:acres_to_plant)
        @acres_delta == other.acres_delta &&
          @bushels_to_feed == other.bushels_to_feed &&
          @acres_to_plant == other.acres_to_plant
      end

      def self.from(order)
        if order.is_a?(TurnOrder)
          unless order.acres_delta.is_a?(Integer) &&
                 order.bushels_to_feed.is_a?(Integer) &&
                 order.acres_to_plant.is_a?(Integer)
            raise InvalidOrderBoundsError, "Order parameters must be integers (rejecting float/string/bool/nil)"
          end
          return order
        end

        if order.is_a?(Hash)
          ad = if order.key?(:acres_delta)
            order[:acres_delta]
          elsif order.key?("acres_delta")
            order["acres_delta"]
          end

          bf = if order.key?(:bushels_to_feed)
            order[:bushels_to_feed]
          elsif order.key?("bushels_to_feed")
            order["bushels_to_feed"]
          end

          ap = if order.key?(:acres_to_plant)
            order[:acres_to_plant]
          elsif order.key?("acres_to_plant")
            order["acres_to_plant"]
          end

          unless ad.is_a?(Integer) && bf.is_a?(Integer) && ap.is_a?(Integer)
            raise InvalidOrderBoundsError, "Order parameters must be integers (rejecting float/string/bool/nil)"
          end

          new(acres_delta: ad, bushels_to_feed: bf, acres_to_plant: ap)
        elsif order.respond_to?(:acres_delta) && order.respond_to?(:bushels_to_feed) && order.respond_to?(:acres_to_plant)
          ad = order.acres_delta
          bf = order.bushels_to_feed
          ap = order.acres_to_plant

          unless ad.is_a?(Integer) && bf.is_a?(Integer) && ap.is_a?(Integer)
            raise InvalidOrderBoundsError, "Order parameters must be integers (rejecting float/string/bool/nil)"
          end

          new(acres_delta: ad, bushels_to_feed: bf, acres_to_plant: ap)
        else
          raise InvalidOrderBoundsError, "Order must be a TurnOrder, Hash, or respond to order attributes"
        end
      end
    end
  end
end
