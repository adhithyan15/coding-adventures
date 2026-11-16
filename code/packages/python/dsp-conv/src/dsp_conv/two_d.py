"""2-D and separable convolution for image filtering."""

from __future__ import annotations

from collections.abc import Sequence

from dsp_conv.boundary import (
    MAX_KERNEL_LEN,
    MAX_SAMPLES,
    BoundaryMode,
    ConvMode,
    EmptyKernelError,
    EmptySignalError,
    ImageSizeMismatchError,
    InvalidInputError,
    KernelTooLargeError,
    extend_index,
    parse_boundary_mode,
    parse_conv_mode,
    validate_floats,
)
from dsp_conv.conv1d import conv1d


def conv2d(
    image: Sequence[float],
    kernel: Sequence[float],
    image_height: int,
    image_width: int,
    kernel_height: int,
    kernel_width: int,
    mode: BoundaryMode | str = BoundaryMode.ZERO,
) -> list[float]:
    """Same-size 2-D convolution on a row-major [H, W] real image.

    Output is a same-size [H, W] row-major list of floats.
    Kernel is centred at (kernel_height // 2, kernel_width // 2).
    """
    if image_height <= 0 or image_width <= 0:
        raise ImageSizeMismatchError(
            f"image dimensions must be positive; got {image_height}x{image_width}"
        )
    if kernel_height <= 0 or kernel_width <= 0:
        raise EmptyKernelError("kernel dimensions must be positive")

    h = image_height
    w = image_width
    kh = kernel_height
    kw = kernel_width

    expected_image_len = h * w
    if expected_image_len > MAX_SAMPLES:
        raise InvalidInputError(
            f"image size {expected_image_len} exceeds MAX_SAMPLES {MAX_SAMPLES}"
        )
    if len(image) != expected_image_len:
        raise ImageSizeMismatchError(
            f"image length {len(image)} does not match {h}x{w} = {expected_image_len}"
        )

    expected_kernel_len = kh * kw
    if expected_kernel_len > MAX_KERNEL_LEN:
        raise InvalidInputError(
            f"kernel size {expected_kernel_len} exceeds MAX_KERNEL_LEN {MAX_KERNEL_LEN}"
        )
    if len(kernel) != expected_kernel_len:
        raise ImageSizeMismatchError(
            f"kernel length {len(kernel)} does not match {kh}x{kw}"
        )

    b_mode = parse_boundary_mode(mode)
    validate_floats(image, "image")
    validate_floats(kernel, "kernel")

    if kh > h or kw > w:
        raise KernelTooLargeError(
            f"kernel {kh}x{kw} exceeds image {h}x{w}"
        )

    ch = kh // 2
    cw = kw // 2

    out = [0.0] * expected_image_len
    for r in range(h):
        r_offset = r * w
        for c in range(w):
            acc = 0.0
            for kr in range(kh):
                src_r = r + ch - kr
                row = extend_index(src_r, h, b_mode)
                if row is None:
                    continue
                row_offset = row * w
                for kc in range(kw):
                    src_c = c + cw - kc
                    col = extend_index(src_c, w, b_mode)
                    if col is None:
                        continue
                    acc += kernel[kr * kw + kc] * image[row_offset + col]
            out[r_offset + c] = acc
    return out


def sep_conv2d(
    image: Sequence[float],
    horizontal_kernel: Sequence[float],
    vertical_kernel: Sequence[float],
    image_height: int,
    image_width: int,
    mode: BoundaryMode | str = BoundaryMode.ZERO,
) -> list[float]:
    """Separable 2-D convolution: row-pass then column-pass in O(H * W * (KH + KW))."""
    if image_height <= 0 or image_width <= 0:
        raise ImageSizeMismatchError(
            f"image dimensions must be positive; got {image_height}x{image_width}"
        )
    if len(horizontal_kernel) == 0 or len(vertical_kernel) == 0:
        raise EmptyKernelError("separable kernels must be non-empty")

    h = image_height
    w = image_width
    expected_image_len = h * w
    if expected_image_len > MAX_SAMPLES:
        raise InvalidInputError(
            f"image size {expected_image_len} exceeds MAX_SAMPLES {MAX_SAMPLES}"
        )
    if len(image) != expected_image_len:
        raise ImageSizeMismatchError(
            f"image length {len(image)} does not match {h}x{w} = {expected_image_len}"
        )
    if (
        len(horizontal_kernel) > MAX_KERNEL_LEN
        or len(vertical_kernel) > MAX_KERNEL_LEN
    ):
        raise InvalidInputError(
            f"separable kernel length exceeds MAX_KERNEL_LEN {MAX_KERNEL_LEN}"
        )

    b_mode = parse_boundary_mode(mode)
    validate_floats(image, "image")
    validate_floats(horizontal_kernel, "horizontal_kernel")
    validate_floats(vertical_kernel, "vertical_kernel")

    # Pass 1: convolve each row horizontally
    intermediate = [0.0] * expected_image_len
    for r in range(h):
        row_slice = image[r * w : (r + 1) * w]
        conv_row = conv1d(row_slice, horizontal_kernel, mode=b_mode)
        intermediate[r * w : (r + 1) * w] = conv_row

    # Pass 2: convolve each column vertically
    out = [0.0] * expected_image_len
    col_buf = [0.0] * h
    for c in range(w):
        for r in range(h):
            col_buf[r] = intermediate[r * w + c]
        conv_col = conv1d(col_buf, vertical_kernel, mode=b_mode)
        for r in range(h):
            out[r * w + c] = conv_col[r]

    return out


def convolve2d(
    image: Sequence[float] | Sequence[Sequence[float]],
    kernel: Sequence[float] | Sequence[Sequence[float]],
    mode: ConvMode | str = ConvMode.FULL,
    boundary: BoundaryMode | str = BoundaryMode.ZERO,
    image_height: int | None = None,
    image_width: int | None = None,
    kernel_height: int | None = None,
    kernel_width: int | None = None,
) -> list[float] | list[list[float]]:
    """Flexible 2-D discrete convolution supporting flat and 2-D nested arrays."""
    if len(image) == 0:
        raise EmptySignalError("image must be non-empty")
    if len(kernel) == 0:
        raise EmptyKernelError("kernel must be non-empty")

    if image_height is not None or image_width is not None:
        is_2d_nested = False
        if image_height is None or image_width is None:
            raise ImageSizeMismatchError(
                "image_height and image_width required for flat 1-D image list"
            )
        h = image_height
        w = image_width
    elif isinstance(image[0], (list, tuple)):
        is_2d_nested = True
        h = len(image)
        w = len(image[0]) if h > 0 and isinstance(image[0], (list, tuple)) else 0  # type: ignore[arg-type]
        for row in image:
            if not isinstance(row, (list, tuple)) or len(row) != w:
                raise ImageSizeMismatchError("jagged 2-D image array")
    else:
        raise ImageSizeMismatchError(
            "image_height and image_width required for flat 1-D image list"
        )

    if h <= 0 or w <= 0:
        raise ImageSizeMismatchError("image dimensions must be > 0")
    img_len = h * w
    if img_len > MAX_SAMPLES:
        raise InvalidInputError("image size exceeds MAX_SAMPLES")
    if not is_2d_nested and len(image) != img_len:
        raise ImageSizeMismatchError(
            f"image length {len(image)} != image_height * image_width ({img_len})"
        )

    if kernel_height is not None or kernel_width is not None:
        is_ker_2d = False
        if kernel_height is None or kernel_width is None:
            raise ImageSizeMismatchError(
                "kernel_height and kernel_width required for flat 1-D kernel list"
            )
        kh = kernel_height
        kw = kernel_width
    elif isinstance(kernel[0], (list, tuple)):
        is_ker_2d = True
        kh = len(kernel)
        kw = len(kernel[0]) if kh > 0 and isinstance(kernel[0], (list, tuple)) else 0  # type: ignore[arg-type]
        for row in kernel:
            if not isinstance(row, (list, tuple)) or len(row) != kw:
                raise ImageSizeMismatchError("jagged 2-D kernel array")
    else:
        raise ImageSizeMismatchError(
            "kernel_height and kernel_width required for flat 1-D kernel list"
        )

    if kh <= 0 or kw <= 0:
        raise EmptyKernelError("kernel dimensions must be > 0")
    ker_len = kh * kw
    if ker_len > MAX_KERNEL_LEN:
        raise InvalidInputError("kernel size exceeds MAX_KERNEL_LEN")
    if not is_ker_2d and len(kernel) != ker_len:
        raise ImageSizeMismatchError(
            f"kernel length {len(kernel)} != kernel_height * kernel_width ({ker_len})"
        )

    c_mode = parse_conv_mode(mode)
    b_mode = parse_boundary_mode(boundary)

    # Budget output dimensions check before any allocation
    if c_mode == ConvMode.SAME:
        out_h, out_w = h, w
    elif c_mode == ConvMode.VALID:
        if h < kh or w < kw:
            raise KernelTooLargeError(
                f"kernel {kh}x{kw} exceeds image {h}x{w} in valid mode"
            )
        out_h = h - kh + 1
        out_w = w - kw + 1
    else:  # FULL
        out_h = h + kh - 1
        out_w = w + kw - 1

    out_len = out_h * out_w
    if out_len > MAX_SAMPLES:
        raise InvalidInputError("output size exceeds MAX_SAMPLES")

    # Allocate flat buffers only after dimensions and bounds verified
    if is_2d_nested:
        flat_img = [float(px) for row in image for px in row]  # type: ignore[union-attr]
    else:
        flat_img = [float(x) for x in image]  # type: ignore[union-attr]
    validate_floats(flat_img, "image")

    if is_ker_2d:
        flat_ker = [float(val) for row in kernel for val in row]  # type: ignore[union-attr]
    else:
        flat_ker = [float(x) for x in kernel]  # type: ignore[union-attr]
    validate_floats(flat_ker, "kernel")

    if c_mode == ConvMode.SAME:
        result_flat = conv2d(flat_img, flat_ker, h, w, kh, kw, mode=b_mode)
    elif c_mode == ConvMode.VALID:
        result_flat = [0.0] * out_len
        for r in range(out_h):
            for c in range(out_w):
                acc = 0.0
                for kr in range(kh):
                    for kc in range(kw):
                        acc += (
                            flat_ker[kr * kw + kc]
                            * flat_img[(r + kh - 1 - kr) * w + (c + kw - 1 - kc)]
                        )
                result_flat[r * out_w + c] = acc
    else:  # FULL
        result_flat = [0.0] * out_len
        for r in range(out_h):
            for c in range(out_w):
                acc = 0.0
                for kr in range(kh):
                    src_r = r - kr
                    row = extend_index(src_r, h, b_mode)
                    if row is None:
                        continue
                    row_offset = row * w
                    for kc in range(kw):
                        src_c = c - kc
                        col = extend_index(src_c, w, b_mode)
                        if col is None:
                            continue
                        acc += flat_ker[kr * kw + kc] * flat_img[row_offset + col]
                result_flat[r * out_w + c] = acc

    if is_2d_nested:
        return [
            result_flat[r * out_w : (r + 1) * out_w]
            for r in range(out_h)
        ]
    return result_flat
