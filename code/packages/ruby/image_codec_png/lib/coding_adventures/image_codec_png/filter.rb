# frozen_string_literal: true

require_relative "constants"
require_relative "error"

module CodingAdventures
  module ImageCodecPng
    # RFC 2083 Section 6: Scanline prediction filters and reconstruction.
    module Filter
      # Paeth predictor (RFC 2083 Section 6.6).
      # Normative tie-breaking order: a, then b, then c.
      def self.paeth(a, b, c)
        p = a + b - c
        pa = (p - a).abs
        pb = (p - b).abs
        pc = (p - c).abs

        if pa <= pb && pa <= pc
          a
        elsif pb <= pc
          b
        else
          c
        end
      end

      # Applies a single filter to a raw row against the reconstructed prior row.
      def self.apply_filter(filter_id, raw, prior, bpp, output)
        stride = raw.length
        i = 0
        while i < stride
          left = (i >= bpp) ? raw[i - bpp] : 0
          above = prior[i]
          above_left = (i >= bpp) ? prior[i - bpp] : 0

          predicted = case filter_id
          when Constants::FILTER_SUB
            left
          when Constants::FILTER_UP
            above
          when Constants::FILTER_AVERAGE
            (left + above) / 2
          when Constants::FILTER_PAETH
            paeth(left, above, above_left)
          else
            0
          end

          output[i] = (raw[i] - predicted) & Constants::BYTE_MASK
          i += 1
        end
      end

      # Chooses the best filter for a row using the sum of signed differences heuristic.
      # Lowest sum wins; ties break towards the lowest numbered filter type.
      def self.choose_filter(raw, prior, bpp, scratch, best)
        best_filter = Constants::FILTER_NONE
        best_score = nil

        filter_id = 0
        while filter_id < Constants::FILTER_COUNT
          apply_filter(filter_id, raw, prior, bpp, scratch)
          score = 0
          scratch.each do |val|
            score += (val < Constants::BYTE_MIDPOINT) ? val : (Constants::BYTE_MODULUS - val)
          end

          if best_score.nil? || score < best_score
            best_score = score
            best_filter = filter_id
            best.replace(scratch)
          end
          filter_id += 1
        end

        best_filter
      end

      # Inverses a scanline filter in place using already-reconstructed samples.
      def self.undo_filter(filter_id, row, prior, bpp)
        len = row.length
        case filter_id
        when Constants::FILTER_NONE
          nil
        when Constants::FILTER_SUB
          i = bpp
          while i < len
            row[i] = (row[i] + row[i - bpp]) & Constants::BYTE_MASK
            i += 1
          end
        when Constants::FILTER_UP
          i = 0
          while i < len
            row[i] = (row[i] + prior[i]) & Constants::BYTE_MASK
            i += 1
          end
        when Constants::FILTER_AVERAGE
          i = 0
          while i < len
            left = (i >= bpp) ? row[i - bpp] : 0
            row[i] = (row[i] + ((left + prior[i]) / 2)) & Constants::BYTE_MASK
            i += 1
          end
        when Constants::FILTER_PAETH
          i = 0
          while i < len
            left = (i >= bpp) ? row[i - bpp] : 0
            above = prior[i]
            above_left = (i >= bpp) ? prior[i - bpp] : 0
            row[i] = (row[i] + paeth(left, above, above_left)) & Constants::BYTE_MASK
            i += 1
          end
        else
          raise PngError.new(Constants::ERR_INVALID_FILTER)
        end
      end
    end
  end
end
