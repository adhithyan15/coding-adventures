package com.codingadventures.imagecodecpng;

import com.codingadventures.pixelcontainer.ImageCodec;
import com.codingadventures.pixelcontainer.PixelContainer;

/** PixelContainer adapter for the bounded IC18 PNG profile. */
public final class PngCodec implements ImageCodec {
    private final long maxPixels;

    /** Construct a codec using the default 32-mebipixel ceiling. */
    public PngCodec() {
        maxPixels = Png.DEFAULT_MAX_PIXELS;
    }

    /** Construct a codec with a caller-lowered pixel ceiling. */
    public PngCodec(Double maxPixels) {
        this.maxPixels = Png.validateMaxPixels(maxPixels);
    }

    @Override
    public String mimeType() {
        return "image/png";
    }

    @Override
    public byte[] encode(PixelContainer container) {
        return Png.encodePng(container);
    }

    @Override
    public PixelContainer decode(byte[] data) {
        return Png.decodePngWithLimit(data, maxPixels);
    }
}
