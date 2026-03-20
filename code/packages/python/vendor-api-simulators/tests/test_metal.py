"""Tests for the Metal runtime simulator."""

import pytest
from gpu_core import limm, halt

from vendor_api_simulators.metal import (
    MTLDevice,
    MTLCommandQueue,
    MTLCommandBuffer,
    MTLComputeCommandEncoder,
    MTLBlitCommandEncoder,
    MTLBuffer,
    MTLLibrary,
    MTLFunction,
    MTLComputePipelineState,
    MTLSize,
    MTLResourceOptions,
    MTLCommandBufferStatus,
)


class TestMTLDevice:
    """Test device creation and properties."""

    def test_create_device(self):
        """MTLDevice initializes successfully."""
        device = MTLDevice()
        assert device._logical_device is not None

    def test_device_name(self):
        """Device has a non-empty name."""
        device = MTLDevice()
        assert len(device.name) > 0

    def test_make_command_queue(self):
        """make_command_queue() returns an MTLCommandQueue."""
        device = MTLDevice()
        queue = device.make_command_queue()
        assert isinstance(queue, MTLCommandQueue)

    def test_make_buffer_default(self):
        """make_buffer() with default options returns an MTLBuffer."""
        device = MTLDevice()
        buf = device.make_buffer(256)
        assert isinstance(buf, MTLBuffer)
        assert buf.length == 256

    def test_make_buffer_shared(self):
        """make_buffer() with storageModeShared."""
        device = MTLDevice()
        buf = device.make_buffer(128, MTLResourceOptions.storageModeShared)
        assert buf.length == 128

    def test_make_library(self):
        """make_library() returns an MTLLibrary."""
        device = MTLDevice()
        lib = device.make_library(source="test_shader")
        assert isinstance(lib, MTLLibrary)

    def test_make_compute_pipeline_state(self):
        """make_compute_pipeline_state() returns a PSO."""
        device = MTLDevice()
        lib = device.make_library(source="test")
        func = lib.make_function("main")
        pso = device.make_compute_pipeline_state(func)
        assert isinstance(pso, MTLComputePipelineState)
        assert pso.max_total_threads_per_threadgroup > 0


class TestMTLBuffer:
    """Test unified memory buffer access."""

    def test_contents_returns_bytearray(self):
        """contents() returns a bytearray."""
        device = MTLDevice()
        buf = device.make_buffer(64)
        data = buf.contents()
        assert isinstance(data, bytearray)
        assert len(data) == 64

    def test_write_bytes(self):
        """write_bytes() writes data accessible via contents()."""
        device = MTLDevice()
        buf = device.make_buffer(16)
        buf.write_bytes(b"\xAA\xBB\xCC\xDD" * 4)
        result = bytes(buf.contents()[:16])
        assert result == b"\xAA\xBB\xCC\xDD" * 4

    def test_write_bytes_with_offset(self):
        """write_bytes() with offset."""
        device = MTLDevice()
        buf = device.make_buffer(16)
        buf.write_bytes(b"\x00" * 16)
        buf.write_bytes(b"\xFF\xFF", offset=4)
        data = bytes(buf.contents())
        assert data[4:6] == b"\xFF\xFF"
        assert data[0:4] == b"\x00" * 4

    def test_length_property(self):
        """length property returns buffer size."""
        device = MTLDevice()
        buf = device.make_buffer(512)
        assert buf.length == 512


class TestMTLLibraryAndFunction:
    """Test shader library and function management."""

    def test_make_function(self):
        """make_function() returns an MTLFunction."""
        device = MTLDevice()
        lib = device.make_library(source="test")
        func = lib.make_function("compute_fn")
        assert isinstance(func, MTLFunction)
        assert func.name == "compute_fn"

    def test_function_name(self):
        """MTLFunction has the correct name."""
        func = MTLFunction(name="saxpy")
        assert func.name == "saxpy"


class TestMTLCommandBuffer:
    """Test command buffer lifecycle."""

    def test_create_command_buffer(self):
        """make_command_buffer() returns an MTLCommandBuffer."""
        device = MTLDevice()
        queue = device.make_command_queue()
        cb = queue.make_command_buffer()
        assert isinstance(cb, MTLCommandBuffer)
        assert cb.status == MTLCommandBufferStatus.notEnqueued

    def test_make_compute_encoder(self):
        """make_compute_command_encoder() returns an encoder."""
        device = MTLDevice()
        queue = device.make_command_queue()
        cb = queue.make_command_buffer()
        encoder = cb.make_compute_command_encoder()
        assert isinstance(encoder, MTLComputeCommandEncoder)

    def test_make_blit_encoder(self):
        """make_blit_command_encoder() returns an encoder."""
        device = MTLDevice()
        queue = device.make_command_queue()
        cb = queue.make_command_buffer()
        encoder = cb.make_blit_command_encoder()
        assert isinstance(encoder, MTLBlitCommandEncoder)

    def test_commit_changes_status(self):
        """commit() changes status to completed."""
        device = MTLDevice()
        queue = device.make_command_queue()
        cb = queue.make_command_buffer()
        # Need at least one encoder to have commands
        encoder = cb.make_compute_command_encoder()
        encoder.end_encoding()
        cb.commit()
        assert cb.status == MTLCommandBufferStatus.completed

    def test_wait_until_completed(self):
        """wait_until_completed() succeeds after commit."""
        device = MTLDevice()
        queue = device.make_command_queue()
        cb = queue.make_command_buffer()
        encoder = cb.make_compute_command_encoder()
        encoder.end_encoding()
        cb.commit()
        cb.wait_until_completed()

    def test_add_completed_handler(self):
        """add_completed_handler() callback is called on commit."""
        device = MTLDevice()
        queue = device.make_command_queue()
        cb = queue.make_command_buffer()
        encoder = cb.make_compute_command_encoder()
        encoder.end_encoding()
        called = [False]
        cb.add_completed_handler(lambda: called.__setitem__(0, True))
        cb.commit()
        assert called[0]


class TestMTLComputeCommandEncoder:
    """Test compute command encoding."""

    def test_set_pipeline_state(self, simple_instructions):
        """set_compute_pipeline_state() succeeds."""
        device = MTLDevice()
        func = MTLFunction(name="test", code=simple_instructions)
        pso = device.make_compute_pipeline_state(func)
        queue = device.make_command_queue()
        cb = queue.make_command_buffer()
        encoder = cb.make_compute_command_encoder()
        encoder.set_compute_pipeline_state(pso)

    def test_set_buffer(self):
        """set_buffer() binds a buffer to an index."""
        device = MTLDevice()
        buf = device.make_buffer(64)
        queue = device.make_command_queue()
        cb = queue.make_command_buffer()
        encoder = cb.make_compute_command_encoder()
        encoder.set_buffer(buf, offset=0, index=0)
        assert 0 in encoder._buffers

    def test_set_bytes(self):
        """set_bytes() stores push constant data."""
        device = MTLDevice()
        queue = device.make_command_queue()
        cb = queue.make_command_buffer()
        encoder = cb.make_compute_command_encoder()
        encoder.set_bytes(b"\x00\x00\x80\x3f", index=2)  # 1.0f
        assert 2 in encoder._push_data

    def test_dispatch_threadgroups(self, simple_instructions):
        """dispatch_threadgroups() records a dispatch command."""
        device = MTLDevice()
        func = MTLFunction(name="test", code=simple_instructions)
        pso = device.make_compute_pipeline_state(func)
        buf = device.make_buffer(128)
        queue = device.make_command_queue()
        cb = queue.make_command_buffer()
        encoder = cb.make_compute_command_encoder()
        encoder.set_compute_pipeline_state(pso)
        encoder.set_buffer(buf, offset=0, index=0)
        encoder.dispatch_threadgroups(
            MTLSize(1, 1, 1), threads_per_threadgroup=MTLSize(32, 1, 1)
        )
        encoder.end_encoding()
        cb.commit()
        cb.wait_until_completed()

    def test_dispatch_threads(self, simple_instructions):
        """dispatch_threads() calculates grid from total threads."""
        device = MTLDevice()
        func = MTLFunction(name="test", code=simple_instructions)
        pso = device.make_compute_pipeline_state(func)
        queue = device.make_command_queue()
        cb = queue.make_command_buffer()
        encoder = cb.make_compute_command_encoder()
        encoder.set_compute_pipeline_state(pso)
        encoder.dispatch_threads(
            MTLSize(128, 1, 1), threads_per_threadgroup=MTLSize(32, 1, 1)
        )
        encoder.end_encoding()
        cb.commit()
        cb.wait_until_completed()

    def test_dispatch_without_pipeline_raises(self):
        """dispatch without setting pipeline raises RuntimeError."""
        device = MTLDevice()
        queue = device.make_command_queue()
        cb = queue.make_command_buffer()
        encoder = cb.make_compute_command_encoder()
        with pytest.raises(RuntimeError, match="No compute pipeline"):
            encoder.dispatch_threadgroups(
                MTLSize(1, 1, 1), threads_per_threadgroup=MTLSize(32, 1, 1)
            )

    def test_end_encoding(self):
        """end_encoding() marks the encoder as ended."""
        device = MTLDevice()
        queue = device.make_command_queue()
        cb = queue.make_command_buffer()
        encoder = cb.make_compute_command_encoder()
        encoder.end_encoding()
        assert encoder._ended


class TestMTLBlitCommandEncoder:
    """Test blit (copy/fill) operations."""

    def test_copy_from_buffer(self):
        """copy_from_buffer() copies between buffers."""
        device = MTLDevice()
        src = device.make_buffer(16)
        dst = device.make_buffer(16)
        src.write_bytes(b"\xAA" * 16)

        queue = device.make_command_queue()
        cb = queue.make_command_buffer()
        encoder = cb.make_blit_command_encoder()
        encoder.copy_from_buffer(src, 0, to_buffer=dst, dst_offset=0, size=16)
        encoder.end_encoding()
        cb.commit()
        cb.wait_until_completed()

    def test_fill_buffer(self):
        """fill_buffer() fills a range with a value."""
        device = MTLDevice()
        buf = device.make_buffer(16)
        queue = device.make_command_queue()
        cb = queue.make_command_buffer()
        encoder = cb.make_blit_command_encoder()
        encoder.fill_buffer(buf, range(0, 16), value=0xFF)
        encoder.end_encoding()
        cb.commit()
        cb.wait_until_completed()

    def test_blit_end_encoding(self):
        """end_encoding() on blit encoder."""
        device = MTLDevice()
        queue = device.make_command_queue()
        cb = queue.make_command_buffer()
        encoder = cb.make_blit_command_encoder()
        encoder.end_encoding()
        assert encoder._ended


class TestMTLSize:
    """Test MTLSize namedtuple."""

    def test_mtl_size_fields(self):
        """MTLSize has width, height, depth."""
        s = MTLSize(4, 2, 1)
        assert s.width == 4
        assert s.height == 2
        assert s.depth == 1


class TestMTLFullPipeline:
    """End-to-end Metal workflow tests."""

    def test_full_compute_pipeline(self, simple_instructions):
        """Full pipeline: device → queue → buffer → encoder → commit."""
        device = MTLDevice()
        queue = device.make_command_queue()

        # Create buffer and write data
        buf = device.make_buffer(64)
        buf.write_bytes(b"\x00" * 64)

        # Create pipeline
        lib = device.make_library(source="test")
        func = MTLFunction(name="test", code=simple_instructions)
        pso = device.make_compute_pipeline_state(func)

        # Encode and submit
        cb = queue.make_command_buffer()
        encoder = cb.make_compute_command_encoder()
        encoder.set_compute_pipeline_state(pso)
        encoder.set_buffer(buf, offset=0, index=0)
        encoder.dispatch_threadgroups(
            MTLSize(1, 1, 1), threads_per_threadgroup=MTLSize(32, 1, 1)
        )
        encoder.end_encoding()
        cb.commit()
        cb.wait_until_completed()

        # Read result (unified memory — direct access)
        result = bytes(buf.contents()[:64])
        assert len(result) == 64

    def test_multiple_encoders(self, simple_instructions):
        """Multiple encoders in one command buffer."""
        device = MTLDevice()
        queue = device.make_command_queue()
        buf = device.make_buffer(32)

        cb = queue.make_command_buffer()

        # Blit encoder to fill
        blit = cb.make_blit_command_encoder()
        blit.fill_buffer(buf, range(0, 32), value=0)
        blit.end_encoding()

        # Compute encoder
        func = MTLFunction(name="test", code=simple_instructions)
        pso = device.make_compute_pipeline_state(func)
        compute = cb.make_compute_command_encoder()
        compute.set_compute_pipeline_state(pso)
        compute.set_buffer(buf, offset=0, index=0)
        compute.dispatch_threadgroups(
            MTLSize(1, 1, 1), threads_per_threadgroup=MTLSize(32, 1, 1)
        )
        compute.end_encoding()

        cb.commit()
        cb.wait_until_completed()
