# frozen_string_literal: true

module CodingAdventures
  module Barcode2D
    # =========================================================================
    # Error hierarchy
    # =========================================================================
    #
    # All errors raised by this package descend from Barcode2DError so callers
    # can rescue the generic base class or catch individual sub-classes:
    #
    #   rescue CodingAdventures::Barcode2D::Barcode2DError => e
    #     # handles every barcode-2d error
    #   end
    #
    #   rescue CodingAdventures::Barcode2D::InvalidBarcode2DConfigError => e
    #     # handles only config validation failures
    #   end

    # Base class for all errors raised by the barcode_2d package.
    class Barcode2DError < StandardError; end

    # Raised by layout() when the configuration is invalid. Common causes:
    #
    #   - module_size_px <= 0
    #   - quiet_zone_modules < 0
    #   - config[:module_shape] does not match grid.module_shape
    #
    # This is always a programming error in the caller (wrong config), not a
    # user-facing validation problem. The message includes the exact constraint
    # that was violated so the developer can fix it quickly.
    class InvalidBarcode2DConfigError < Barcode2DError; end
  end
end
