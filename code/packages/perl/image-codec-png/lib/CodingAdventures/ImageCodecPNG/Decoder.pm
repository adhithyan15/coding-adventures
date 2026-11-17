package CodingAdventures::ImageCodecPNG::Decoder;

use strict;
use warnings;
use Scalar::Util qw(looks_like_number);
use CodingAdventures::PixelContainer;
use CodingAdventures::Zip qw(crc32 raw_inflate_counted);
use CodingAdventures::ImageCodecPNG::Constants qw(:all);
use CodingAdventures::ImageCodecPNG::Error;
use CodingAdventures::ImageCodecPNG::Chunk;
use CodingAdventures::ImageCodecPNG::Filter;
use CodingAdventures::ImageCodecPNG::Adler32;

# Eagerly validates a caller-provided or default pixel limit.
sub validate_max_pixels {
    my ($requested) = @_;
    return DEFAULT_MAX_PIXELS() unless defined $requested;

    if (!is_strict_integer($requested) ||
        $requested <= 0 || $requested > PNG_MAX_PIXELS()) {
        CodingAdventures::ImageCodecPNG::Error->throw(ERR_INVALID_MAX_PIXELS());
    }

    return int($requested);
}

# Decodes a PNG byte stream into a CodingAdventures::PixelContainer object.
sub decode {
    my ($data, @rest) = @_;
    my %opts;
    if (@rest == 1) {
        if (ref($rest[0]) eq 'HASH') {
            %opts = %{$rest[0]};
        } else {
            $opts{max_pixels} = $rest[0];
        }
    } elsif (@rest > 1) {
        if (@rest % 2 != 0) {
            CodingAdventures::ImageCodecPNG::Error->throw(ERR_INVALID_MAX_PIXELS());
        }
        %opts = @rest;
    }

    my $limit = validate_max_pixels($opts{max_pixels});

    my $raw;
    if (!defined($data) || (ref($data) && ref($data) ne 'ARRAY')) {
        CodingAdventures::ImageCodecPNG::Error->throw(ERR_FILE_TOO_SHORT());
    } elsif (ref($data) eq 'ARRAY') {
        for my $b (@$data) {
            if (!is_strict_integer($b) || $b < 0 || $b > BYTE_MASK()) {
                CodingAdventures::ImageCodecPNG::Error->throw(ERR_FILE_TOO_SHORT());
            }
        }
        $raw = pack(BYTE_ARRAY_PACK_FORMAT(), @$data);
    } else {
        if (utf8::is_utf8($data)) {
            $raw = $data;
            if (!utf8::downgrade($raw, 1)) {
                CodingAdventures::ImageCodecPNG::Error->throw(ERR_FILE_TOO_SHORT());
            }
        } else {
            $raw = $data;
        }
    }

    my $total_len = length($raw);
    if ($total_len < SIGNATURE_LENGTH()) {
        CodingAdventures::ImageCodecPNG::Error->throw(ERR_FILE_TOO_SHORT());
    }
    if (substr($raw, 0, SIGNATURE_LENGTH()) ne PNG_SIGNATURE()) {
        CodingAdventures::ImageCodecPNG::Error->throw(ERR_INVALID_SIGNATURE());
    }

    my $state = parse_chunks($raw, $total_len, $limit);
    my $uncompressed = decompress_idat($state->{zlib_data}, $state->{width}, $state->{height}, $state->{colour_type});
    return reconstruct_image($uncompressed, $state->{width}, $state->{height}, $state->{colour_type},
        $state->{transparent_grey}, $state->{transparent_rgb});
}

# Walks and validates framing for all chunks in the PNG file.
sub parse_chunks {
    my ($raw, $total_len, $limit) = @_;

    my $pos = SIGNATURE_LENGTH();
    my $saw_ihdr = 0;
    my $saw_iend = 0;
    my $saw_plte = 0;
    my $saw_trns = 0;
    my $saw_idat = 0;
    my $in_idat = 0;
    my $idat_ended = 0;

    my $width = 0;
    my $height = 0;
    my $colour_type = 0;
    my $transparent_grey;
    my $transparent_rgb;
    my $zlib_data = '';

    while ($pos < $total_len) {
        if ($total_len - $pos < CHUNK_HEADER_SIZE()) {
            CodingAdventures::ImageCodecPNG::Error->throw(ERR_TRUNCATED_CHUNK());
        }

        my $length = unpack(CHUNK_LENGTH_PACK_FORMAT(), substr($raw, $pos, CHUNK_LENGTH_SIZE()));
        if ($length > $total_len - $pos - CHUNK_OVERHEAD()) {
            CodingAdventures::ImageCodecPNG::Error->throw(ERR_TRUNCATED_CHUNK());
        }

        my $chunk_type = substr($raw, $pos + CHUNK_LENGTH_SIZE(), CHUNK_TYPE_SIZE());
        unless (CodingAdventures::ImageCodecPNG::Chunk::valid_type($chunk_type)) {
            CodingAdventures::ImageCodecPNG::Error->throw(ERR_INVALID_CHUNK_TYPE());
        }

        my $data_start = $pos + CHUNK_HEADER_SIZE();
        my $payload = substr($raw, $data_start, $length);
        my $crc_offset = $data_start + $length;
        my $declared_crc = unpack(CRC_PACK_FORMAT(), substr($raw, $crc_offset, CHUNK_CRC_SIZE()));

        my $actual_crc = CodingAdventures::Zip::crc32($chunk_type);
        if (length($payload) > 0) {
            $actual_crc = CodingAdventures::Zip::crc32($payload, $actual_crc);
        }
        if ($actual_crc != $declared_crc) {
            CodingAdventures::ImageCodecPNG::Error->throw(ERR_CHUNK_CRC_MISMATCH());
        }

        if (!$saw_ihdr && $chunk_type ne CHUNK_IHDR()) {
            CodingAdventures::ImageCodecPNG::Error->throw(ERR_CHUNK_BEFORE_IHDR());
        }

        if ($chunk_type eq CHUNK_IHDR()) {
            if ($saw_ihdr) {
                CodingAdventures::ImageCodecPNG::Error->throw(ERR_DUPLICATE_IHDR());
            }
            if ($length != IHDR_LENGTH()) {
                CodingAdventures::ImageCodecPNG::Error->throw(ERR_INVALID_IHDR_LENGTH());
            }

            my ($w, $h, $bit_depth, $col_type, $compression, $filter_method, $interlace)
                = unpack(IHDR_PACK_FORMAT(), $payload);

            if ($w == 0 || $h == 0) {
                CodingAdventures::ImageCodecPNG::Error->throw(ERR_INVALID_DIMENSIONS());
            }
            if ($w > PNG_MAX_DIMENSION() || $h > PNG_MAX_DIMENSION()) {
                CodingAdventures::ImageCodecPNG::Error->throw(ERR_DIMENSION_LIMIT());
            }
            if ($w * $h > $limit) {
                CodingAdventures::ImageCodecPNG::Error->throw(ERR_PIXEL_LIMIT());
            }
            if ($compression != COMPRESSION_METHOD_DEFLATE() ||
                $filter_method != FILTER_METHOD_ADAPTIVE() ||
                $interlace != INTERLACE_METHOD_NONE()) {
                CodingAdventures::ImageCodecPNG::Error->throw(ERR_UNSUPPORTED_FEATURE());
            }
            if ($bit_depth != SUPPORTED_BIT_DEPTH() ||
                !exists $CodingAdventures::ImageCodecPNG::Constants::CHANNELS_BY_COLOUR_TYPE{$col_type}) {
                CodingAdventures::ImageCodecPNG::Error->throw(ERR_UNSUPPORTED_FEATURE());
            }

            $width = $w;
            $height = $h;
            $colour_type = $col_type;
            $saw_ihdr = 1;
        } elsif ($chunk_type eq CHUNK_PLTE()) {
            if ($saw_plte || $saw_idat || $saw_trns ||
                ($colour_type != COLOUR_TYPE_RGB() && $colour_type != COLOUR_TYPE_RGBA()) ||
                $length < PLTE_MIN_LENGTH() || $length > PLTE_MAX_LENGTH() ||
                ($length % PLTE_ENTRY_SIZE() != 0)) {
                CodingAdventures::ImageCodecPNG::Error->throw(ERR_INVALID_PLTE());
            }
            $saw_plte = 1;
        } elsif ($chunk_type eq CHUNK_TRNS()) {
            if ($saw_trns || $saw_idat) {
                CodingAdventures::ImageCodecPNG::Error->throw(ERR_INVALID_TRNS());
            }

            if ($colour_type == COLOUR_TYPE_GREY()) {
                if ($length != TRNS_GREY_LENGTH()) {
                    CodingAdventures::ImageCodecPNG::Error->throw(ERR_INVALID_TRNS());
                }
                my $sample = unpack(TRNS_GREY_PACK_FORMAT(), $payload);
                if ($sample > MAX_SAMPLE_VALUE()) {
                    CodingAdventures::ImageCodecPNG::Error->throw(ERR_INVALID_TRNS());
                }
                $transparent_grey = $sample;
            } elsif ($colour_type == COLOUR_TYPE_RGB()) {
                if ($length != TRNS_RGB_LENGTH()) {
                    CodingAdventures::ImageCodecPNG::Error->throw(ERR_INVALID_TRNS());
                }
                my @samples = unpack(TRNS_RGB_PACK_FORMAT(), $payload);
                for my $s (@samples) {
                    if ($s > MAX_SAMPLE_VALUE()) {
                        CodingAdventures::ImageCodecPNG::Error->throw(ERR_INVALID_TRNS());
                    }
                }
                $transparent_rgb = \@samples;
            } else {
                CodingAdventures::ImageCodecPNG::Error->throw(ERR_INVALID_TRNS());
            }
            $saw_trns = 1;
        } elsif ($chunk_type eq CHUNK_IDAT()) {
            if ($idat_ended) {
                CodingAdventures::ImageCodecPNG::Error->throw(ERR_NONCONSECUTIVE_IDAT());
            }
            $saw_idat = 1;
            $zlib_data .= $payload;
            $in_idat = 1;
        } elsif ($chunk_type eq CHUNK_IEND()) {
            if ($length != 0) {
                CodingAdventures::ImageCodecPNG::Error->throw(ERR_INVALID_IEND());
            }
            if ($crc_offset + CHUNK_CRC_SIZE() != $total_len) {
                CodingAdventures::ImageCodecPNG::Error->throw(ERR_TRAILING_DATA());
            }
            $saw_iend = 1;
            last;
        } elsif ($chunk_type eq CHUNK_ACTL() || $chunk_type eq CHUNK_FCTL() || $chunk_type eq CHUNK_FDAT()) {
            CodingAdventures::ImageCodecPNG::Error->throw(ERR_UNSUPPORTED_FEATURE());
        } else {
            if (CodingAdventures::ImageCodecPNG::Chunk::critical($chunk_type)) {
                CodingAdventures::ImageCodecPNG::Error->throw(ERR_UNKNOWN_CRITICAL_CHUNK());
            }
        }

        if ($chunk_type ne CHUNK_IDAT() && $in_idat) {
            $in_idat = 0;
            $idat_ended = 1;
        }

        $pos = $crc_offset + CHUNK_CRC_SIZE();
    }

    if (!$saw_ihdr || !$saw_iend || !$saw_idat) {
        CodingAdventures::ImageCodecPNG::Error->throw(ERR_MISSING_REQUIRED_CHUNK());
    }

    return {
        width            => $width,
        height           => $height,
        colour_type      => $colour_type,
        transparent_grey => $transparent_grey,
        transparent_rgb  => $transparent_rgb,
        zlib_data        => $zlib_data,
    };
}

# Validates zlib wrapper and decompresses DEFLATE stream bounded by expected scanline size.
sub decompress_idat {
    my ($zlib_data, $width, $height, $colour_type) = @_;

    if (length($zlib_data) < ZLIB_MIN_STREAM_SIZE()) {
        CodingAdventures::ImageCodecPNG::Error->throw(ERR_INVALID_ZLIB_HEADER());
    }

    my $cmf = ord(substr($zlib_data, 0, 1));
    my $flg = ord(substr($zlib_data, 1, 1));

    if (($cmf & ZLIB_CMF_METHOD_MASK()) != ZLIB_METHOD_DEFLATE() ||
        ($cmf >> ZLIB_CMF_CINFO_SHIFT()) > ZLIB_CINFO_MAX() ||
        ((($cmf << BITS_PER_BYTE()) | $flg) % ZLIB_FCHECK_MODULUS()) != 0) {
        CodingAdventures::ImageCodecPNG::Error->throw(ERR_INVALID_ZLIB_HEADER());
    }
    if (($flg & ZLIB_FDICT_MASK()) != 0) {
        CodingAdventures::ImageCodecPNG::Error->throw(ERR_PRESET_DICTIONARY());
    }

    my $channels = $CodingAdventures::ImageCodecPNG::Constants::CHANNELS_BY_COLOUR_TYPE{$colour_type};
    my $stride = $width * $channels;
    my $expected_length = $height * ($stride + 1);
    my $deflate_data = substr($zlib_data, ZLIB_HEADER_SIZE(), length($zlib_data) - ZLIB_OVERHEAD());

    my $inflated;
    eval {
        $inflated = CodingAdventures::Zip::raw_inflate_counted($deflate_data, $expected_length);
        1;
    } or do {
        my $e = $@;
        if (ref($e) && $e->can('code') && $e->code eq ZIP_ERR_OUTPUT_LIMIT_EXCEEDED()) {
            CodingAdventures::ImageCodecPNG::Error->throw(ERR_INFLATED_LENGTH_MISMATCH());
        }
        CodingAdventures::ImageCodecPNG::Error->throw(ERR_INFLATE_FAILED());
    };

    if (length($inflated->output) != $expected_length) {
        CodingAdventures::ImageCodecPNG::Error->throw(ERR_INFLATED_LENGTH_MISMATCH());
    }
    if ($inflated->bytes_consumed != length($deflate_data)) {
        CodingAdventures::ImageCodecPNG::Error->throw(ERR_IDAT_CAVITY());
    }

    my $declared_adler = unpack(ADLER32_PACK_FORMAT(), substr($zlib_data, length($zlib_data) - ZLIB_TRAILER_SIZE(), ZLIB_TRAILER_SIZE()));
    my $actual_adler = CodingAdventures::ImageCodecPNG::Adler32::compute($inflated->output);
    if ($actual_adler != $declared_adler) {
        CodingAdventures::ImageCodecPNG::Error->throw(ERR_ADLER_MISMATCH());
    }

    my $row_size = $stride + 1;
    for (my $y = 0; $y < $height; $y++) {
        if (ord(substr($inflated->output, $y * $row_size, 1)) > MAX_FILTER_TYPE()) {
            CodingAdventures::ImageCodecPNG::Error->throw(ERR_INVALID_FILTER());
        }
    }

    return $inflated->output;
}

# Inverses row filters and widens all colour types into RGBA PixelContainer.
sub reconstruct_image {
    my ($uncompressed, $width, $height, $colour_type, $transparent_grey, $transparent_rgb) = @_;

    my $channels = $CodingAdventures::ImageCodecPNG::Constants::CHANNELS_BY_COLOUR_TYPE{$colour_type};
    my $stride = $width * $channels;
    my $row_size = $stride + 1;

    my $container = CodingAdventures::PixelContainer->new($width, $height);
    my $out_data = $container->data;

    my @prior = (0) x $stride;
    my @row = (0) x $stride;

    for (my $y = 0; $y < $height; $y++) {
        my $offset = $y * $row_size;
        my $filter_type = ord(substr($uncompressed, $offset, 1));
        my $row_slice = substr($uncompressed, $offset + 1, $stride);
        @row = unpack(BYTE_ARRAY_PACK_FORMAT(), $row_slice);

        CodingAdventures::ImageCodecPNG::Filter::undo_filter($filter_type, \@row, \@prior, $channels);

        my $dest_row = $y * $width * BYTES_PER_RGBA_PIXEL();
        for (my $x = 0; $x < $width; $x++) {
            my $src = $x * $channels;
            my $dest = $dest_row + $x * BYTES_PER_RGBA_PIXEL();

            if ($channels == CHANNELS_GREY()) {
                my $v = $row[$src];
                my $a = (defined $transparent_grey && $transparent_grey == $v)
                    ? TRANSPARENT_ALPHA()
                    : OPAQUE_ALPHA();
                substr($$out_data, $dest, BYTES_PER_RGBA_PIXEL()) = pack(RGBA_PACK_FORMAT(), $v, $v, $v, $a);
            } elsif ($channels == CHANNELS_GREY_ALPHA()) {
                my $v = $row[$src];
                my $a = $row[$src + 1];
                substr($$out_data, $dest, BYTES_PER_RGBA_PIXEL()) = pack(RGBA_PACK_FORMAT(), $v, $v, $v, $a);
            } elsif ($channels == CHANNELS_RGB()) {
                my $r = $row[$src];
                my $g = $row[$src + 1];
                my $b = $row[$src + 2];
                my $is_trans = (defined $transparent_rgb &&
                                $transparent_rgb->[0] == $r &&
                                $transparent_rgb->[1] == $g &&
                                $transparent_rgb->[2] == $b);
                my $a = $is_trans ? TRANSPARENT_ALPHA() : OPAQUE_ALPHA();
                substr($$out_data, $dest, BYTES_PER_RGBA_PIXEL()) = pack(RGBA_PACK_FORMAT(), $r, $g, $b, $a);
            } else {
                substr($$out_data, $dest, BYTES_PER_RGBA_PIXEL()) = pack(RGBA_PACK_FORMAT(), $row[$src], $row[$src + 1], $row[$src + 2], $row[$src + 3]);
            }
        }

        @prior = @row;
    }

    return $container;
}

1;
