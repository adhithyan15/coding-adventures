package com.codingadventures.barcode2d;

/**
 * Thrown by {@link Barcode2D#layout} when the layout configuration is invalid.
 *
 * <p>Specific causes:
 * <ul>
 *   <li>{@link Barcode2DLayoutConfig#moduleSizePx} ≤ 0</li>
 *   <li>{@link Barcode2DLayoutConfig#quietZoneModules} &lt; 0</li>
 *   <li>{@link Barcode2DLayoutConfig#moduleShape} does not match
 *       {@link ModuleGrid#moduleShape}</li>
 * </ul>
 *
 * <p>These are programming errors in the caller — the encoder produced a grid
 * with one shape and the caller passed a config expecting a different shape.
 *
 * <p>Using a dedicated exception lets callers catch barcode-specific errors with
 * {@code catch (InvalidBarcode2DConfigException e)} without accidentally swallowing
 * general {@link RuntimeException}s from the JVM or other libraries.
 *
 * <p>Spec: DT2D01 barcode-2d.
 */
public class InvalidBarcode2DConfigException extends RuntimeException {

    /**
     * Construct with an explanatory message.
     *
     * @param message Description of the invalid configuration.
     */
    public InvalidBarcode2DConfigException(String message) {
        super(message);
    }
}
