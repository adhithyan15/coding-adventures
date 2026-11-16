# frozen_string_literal: true

require "coding_adventures_zip"
require "coding_adventures/pixel_container"
require_relative "constants"
require_relative "error"
require_relative "chunk"
require_relative "filter"
require_relative "adler32"

module CodingAdventures
  module ImageCodecPng
    # Decodes bounded, non-interlaced, 8-bit IC18 PNG byte streams.
    module Decoder
      # Eagerly validates a caller-provided or default pixel limit.
      def self.validate_max_pixels(requested)
        return Constants::DEFAULT_MAX_PIXELS if requested.nil?

        unless requested.is_a?(Integer) && requested.positive? && requested <= Constants::PNG_MAX_PIXELS
          raise PngError.new(Constants::ERR_INVALID_MAX_PIXELS)
        end

        requested
      end

      # Decodes a PNG stream into a PixelContainer::Container.
      def self.decode(data, max_pixels: nil)
        limit = validate_max_pixels(max_pixels)
        raw = if data.is_a?(String)
          data.b
        elsif data.is_a?(Array) && data.all? { |b| b.is_a?(Integer) && b.between?(0, Constants::BYTE_MASK) }
          data.pack(Constants::BYTE_ARRAY_PACK_FORMAT).b
        else
          raise PngError.new(Constants::ERR_FILE_TOO_SHORT)
        end
        total_len = raw.bytesize

        if total_len < Constants::SIGNATURE_LENGTH
          raise PngError.new(Constants::ERR_FILE_TOO_SHORT)
        end
        if raw.byteslice(0, Constants::SIGNATURE_LENGTH) != Constants::PNG_SIGNATURE
          raise PngError.new(Constants::ERR_INVALID_SIGNATURE)
        end

        state = parse_chunks(raw, total_len, limit)
        uncompressed = decompress_idat(state[:zlib_data], state[:width], state[:height], state[:colour_type])
        reconstruct_image(uncompressed, state[:width], state[:height], state[:colour_type],
          state[:transparent_grey], state[:transparent_rgb])
      end

      # Walks and validates framing for all chunks in the PNG file.
      def self.parse_chunks(raw, total_len, limit)
        pos = Constants::SIGNATURE_LENGTH
        saw_ihdr = false
        saw_iend = false
        saw_plte = false
        saw_trns = false
        saw_idat = false
        in_idat = false
        idat_ended = false

        width = 0
        height = 0
        colour_type = 0
        transparent_grey = nil
        transparent_rgb = nil
        zlib_data = String.new(encoding: Encoding::BINARY)

        while pos < total_len
          if total_len - pos < Constants::CHUNK_HEADER_SIZE
            raise PngError.new(Constants::ERR_TRUNCATED_CHUNK)
          end

          length = raw.byteslice(pos, Constants::CHUNK_LENGTH_SIZE).unpack1(Constants::CHUNK_LENGTH_PACK_FORMAT)
          if length > total_len - pos - Constants::CHUNK_OVERHEAD
            raise PngError.new(Constants::ERR_TRUNCATED_CHUNK)
          end

          chunk_type = raw.byteslice(pos + Constants::CHUNK_LENGTH_SIZE, Constants::CHUNK_TYPE_SIZE)
          unless Chunk.valid_type?(chunk_type)
            raise PngError.new(Constants::ERR_INVALID_CHUNK_TYPE)
          end

          data_start = pos + Constants::CHUNK_HEADER_SIZE
          payload = raw.byteslice(data_start, length)
          crc_offset = data_start + length
          declared_crc = raw.byteslice(crc_offset, Constants::CHUNK_CRC_SIZE).unpack1(Constants::CRC_PACK_FORMAT)

          actual_crc = CodingAdventures::Zip.crc32(chunk_type)
          actual_crc = CodingAdventures::Zip.crc32(payload, initial: actual_crc) unless payload.empty?
          if actual_crc != declared_crc
            raise PngError.new(Constants::ERR_CHUNK_CRC_MISMATCH)
          end

          if !saw_ihdr && chunk_type != Constants::CHUNK_IHDR
            raise PngError.new(Constants::ERR_CHUNK_BEFORE_IHDR)
          end

          case chunk_type
          when Constants::CHUNK_IHDR
            if saw_ihdr
              raise PngError.new(Constants::ERR_DUPLICATE_IHDR)
            end
            if length != Constants::IHDR_LENGTH
              raise PngError.new(Constants::ERR_INVALID_IHDR_LENGTH)
            end

            w, h, bit_depth, col_type, compression, filter_method, interlace = payload.unpack(Constants::IHDR_PACK_FORMAT)
            if w == 0 || h == 0
              raise PngError.new(Constants::ERR_INVALID_DIMENSIONS)
            end
            if w > Constants::PNG_MAX_DIMENSION || h > Constants::PNG_MAX_DIMENSION
              raise PngError.new(Constants::ERR_DIMENSION_LIMIT)
            end
            if w * h > limit
              raise PngError.new(Constants::ERR_PIXEL_LIMIT)
            end
            if compression != Constants::COMPRESSION_METHOD_DEFLATE ||
                filter_method != Constants::FILTER_METHOD_ADAPTIVE ||
                interlace != Constants::INTERLACE_METHOD_NONE
              raise PngError.new(Constants::ERR_UNSUPPORTED_FEATURE)
            end
            if bit_depth != Constants::SUPPORTED_BIT_DEPTH || !Constants::SUPPORTED_COLOUR_TYPES.include?(col_type)
              raise PngError.new(Constants::ERR_UNSUPPORTED_FEATURE)
            end

            width = w
            height = h
            colour_type = col_type
            saw_ihdr = true
          when Constants::CHUNK_PLTE
            if saw_plte || saw_idat || saw_trns ||
                (colour_type != Constants::COLOUR_TYPE_RGB && colour_type != Constants::COLOUR_TYPE_RGBA) ||
                length < Constants::PLTE_MIN_LENGTH || length > Constants::PLTE_MAX_LENGTH ||
                (length % Constants::PLTE_ENTRY_SIZE != 0)
              raise PngError.new(Constants::ERR_INVALID_PLTE)
            end
            saw_plte = true
          when Constants::CHUNK_TRNS
            if saw_trns || saw_idat
              raise PngError.new(Constants::ERR_INVALID_TRNS)
            end

            case colour_type
            when Constants::COLOUR_TYPE_GREY
              if length != Constants::TRNS_GREY_LENGTH
                raise PngError.new(Constants::ERR_INVALID_TRNS)
              end
              sample = payload.unpack1(Constants::TRNS_GREY_PACK_FORMAT)
              if sample > Constants::MAX_SAMPLE_VALUE
                raise PngError.new(Constants::ERR_INVALID_TRNS)
              end
              transparent_grey = sample
            when Constants::COLOUR_TYPE_RGB
              if length != Constants::TRNS_RGB_LENGTH
                raise PngError.new(Constants::ERR_INVALID_TRNS)
              end
              samples = payload.unpack(Constants::TRNS_RGB_PACK_FORMAT)
              if samples.any? { |s| s > Constants::MAX_SAMPLE_VALUE }
                raise PngError.new(Constants::ERR_INVALID_TRNS)
              end
              transparent_rgb = samples
            else
              raise PngError.new(Constants::ERR_INVALID_TRNS)
            end
            saw_trns = true
          when Constants::CHUNK_IDAT
            if idat_ended
              raise PngError.new(Constants::ERR_NONCONSECUTIVE_IDAT)
            end
            saw_idat = true
            zlib_data << payload
            in_idat = true
          when Constants::CHUNK_IEND
            if length != 0
              raise PngError.new(Constants::ERR_INVALID_IEND)
            end
            if crc_offset + Constants::CHUNK_CRC_SIZE != total_len
              raise PngError.new(Constants::ERR_TRAILING_DATA)
            end
            saw_iend = true
            pos = total_len
            break
          when *Constants::APNG_CHUNKS
            raise PngError.new(Constants::ERR_UNSUPPORTED_FEATURE)
          else
            if Chunk.critical?(chunk_type)
              raise PngError.new(Constants::ERR_UNKNOWN_CRITICAL_CHUNK)
            end
          end

          if chunk_type != Constants::CHUNK_IDAT && in_idat
            in_idat = false
            idat_ended = true
          end

          pos = crc_offset + Constants::CHUNK_CRC_SIZE
        end

        if !saw_ihdr || !saw_iend || !saw_idat
          raise PngError.new(Constants::ERR_MISSING_REQUIRED_CHUNK)
        end

        {
          width: width,
          height: height,
          colour_type: colour_type,
          transparent_grey: transparent_grey,
          transparent_rgb: transparent_rgb,
          zlib_data: zlib_data
        }
      end

      # Validates zlib wrapper and decompresses DEFLATE stream bounded by expected scanline size.
      def self.decompress_idat(zlib_data, width, height, colour_type)
        if zlib_data.bytesize < Constants::ZLIB_MIN_STREAM_SIZE
          raise PngError.new(Constants::ERR_INVALID_ZLIB_HEADER)
        end

        cmf = zlib_data.getbyte(0)
        flg = zlib_data.getbyte(1)
        if (cmf & Constants::ZLIB_CMF_METHOD_MASK) != Constants::ZLIB_METHOD_DEFLATE ||
            (cmf >> Constants::ZLIB_CMF_CINFO_SHIFT) > Constants::ZLIB_CINFO_MAX ||
            (((cmf << Constants::BITS_PER_BYTE) | flg) % Constants::ZLIB_FCHECK_MODULUS) != 0
          raise PngError.new(Constants::ERR_INVALID_ZLIB_HEADER)
        end
        if (flg & Constants::ZLIB_FDICT_MASK) != 0
          raise PngError.new(Constants::ERR_PRESET_DICTIONARY)
        end

        channels = Constants::CHANNELS_BY_COLOUR_TYPE.fetch(colour_type)
        stride = width * channels
        expected_length = height * (stride + 1)
        deflate_data = zlib_data.byteslice(Constants::ZLIB_HEADER_SIZE, zlib_data.bytesize - Constants::ZLIB_OVERHEAD)

        begin
          inflated = CodingAdventures::Zip.raw_inflate_counted(deflate_data, max_output: expected_length)
        rescue CodingAdventures::Zip::RawInflateError => e
          if e.code == Constants::ZIP_ERR_OUTPUT_LIMIT_EXCEEDED
            raise PngError.new(Constants::ERR_INFLATED_LENGTH_MISMATCH)
          end
          raise PngError.new(Constants::ERR_INFLATE_FAILED)
        end

        if inflated.output.bytesize != expected_length
          raise PngError.new(Constants::ERR_INFLATED_LENGTH_MISMATCH)
        end
        if inflated.bytes_consumed != deflate_data.bytesize
          raise PngError.new(Constants::ERR_IDAT_CAVITY)
        end

        declared_adler = zlib_data.byteslice(zlib_data.bytesize - Constants::ZLIB_TRAILER_SIZE, Constants::ZLIB_TRAILER_SIZE).unpack1(Constants::ADLER32_PACK_FORMAT)
        actual_adler = Adler32.compute(inflated.output)
        if actual_adler != declared_adler
          raise PngError.new(Constants::ERR_ADLER_MISMATCH)
        end

        row_size = stride + 1
        y = 0
        while y < height
          if inflated.output.getbyte(y * row_size) > Constants::MAX_FILTER_TYPE
            raise PngError.new(Constants::ERR_INVALID_FILTER)
          end
          y += 1
        end

        inflated.output
      end

      # Inverses row filters and widens all colour types into RGBA PixelContainer.
      def self.reconstruct_image(uncompressed, width, height, colour_type, transparent_grey, transparent_rgb)
        channels = Constants::CHANNELS_BY_COLOUR_TYPE.fetch(colour_type)
        stride = width * channels
        row_size = stride + 1

        container = CodingAdventures::PixelContainer.create(width, height)
        prior = Array.new(stride, 0)
        row = Array.new(stride, 0)

        y = 0
        while y < height
          offset = y * row_size
          filter_type = uncompressed.getbyte(offset)

          i = 0
          while i < stride
            row[i] = uncompressed.getbyte(offset + 1 + i)
            i += 1
          end

          Filter.undo_filter(filter_type, row, prior, channels)

          dest_row = y * width * Constants::BYTES_PER_RGBA_PIXEL
          x = 0
          while x < width
            src = x * channels
            dest = dest_row + x * Constants::BYTES_PER_RGBA_PIXEL

            case channels
            when Constants::CHANNELS_GREY
              v = row[src]
              a = (transparent_grey == v) ? Constants::TRANSPARENT_ALPHA : Constants::OPAQUE_ALPHA
              container.data.setbyte(dest, v)
              container.data.setbyte(dest + 1, v)
              container.data.setbyte(dest + 2, v)
              container.data.setbyte(dest + 3, a)
            when Constants::CHANNELS_GREY_ALPHA
              v = row[src]
              a = row[src + 1]
              container.data.setbyte(dest, v)
              container.data.setbyte(dest + 1, v)
              container.data.setbyte(dest + 2, v)
              container.data.setbyte(dest + 3, a)
            when Constants::CHANNELS_RGB
              r = row[src]
              g = row[src + 1]
              b = row[src + 2]
              is_transparent = transparent_rgb && transparent_rgb[0] == r && transparent_rgb[1] == g && transparent_rgb[2] == b
              a = is_transparent ? Constants::TRANSPARENT_ALPHA : Constants::OPAQUE_ALPHA
              container.data.setbyte(dest, r)
              container.data.setbyte(dest + 1, g)
              container.data.setbyte(dest + 2, b)
              container.data.setbyte(dest + 3, a)
            else
              container.data.setbyte(dest, row[src])
              container.data.setbyte(dest + 1, row[src + 1])
              container.data.setbyte(dest + 2, row[src + 2])
              container.data.setbyte(dest + 3, row[src + 3])
            end

            x += 1
          end

          prior.replace(row)
          y += 1
        end

        container
      end
    end
  end
end
