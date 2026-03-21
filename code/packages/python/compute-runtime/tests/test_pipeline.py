"""Tests for Pipeline, ShaderModule, DescriptorSet."""

import pytest
from gpu_core import limm, halt, fadd

from compute_runtime import (
    RuntimeInstance,
    MemoryType,
    BufferUsage,
    DescriptorBinding,
    ShaderModule,
    DescriptorSetLayout,
    PipelineLayout,
    Pipeline,
    DescriptorSet,
)


def make_device():
    instance = RuntimeInstance()
    physical = instance.enumerate_physical_devices()[0]
    return instance.create_logical_device(physical)


class TestShaderModule:
    def test_gpu_style(self) -> None:
        shader = ShaderModule(code=[limm(0, 1.0), halt()])
        assert shader.is_gpu_style
        assert not shader.is_dataflow_style
        assert shader.code is not None
        assert len(shader.code) == 2

    def test_dataflow_style(self) -> None:
        shader = ShaderModule(operation="matmul")
        assert shader.is_dataflow_style
        assert not shader.is_gpu_style
        assert shader.operation == "matmul"

    def test_local_size(self) -> None:
        shader = ShaderModule(
            code=[halt()], local_size=(256, 1, 1)
        )
        assert shader.local_size == (256, 1, 1)

    def test_entry_point(self) -> None:
        shader = ShaderModule(code=[halt()], entry_point="compute_main")
        assert shader.entry_point == "compute_main"

    def test_unique_ids(self) -> None:
        s1 = ShaderModule(code=[halt()])
        s2 = ShaderModule(code=[halt()])
        assert s1.module_id != s2.module_id

    def test_default_entry_point(self) -> None:
        shader = ShaderModule(code=[halt()])
        assert shader.entry_point == "main"

    def test_default_local_size(self) -> None:
        shader = ShaderModule(code=[halt()])
        assert shader.local_size == (32, 1, 1)


class TestDescriptorSetLayout:
    def test_basic_layout(self) -> None:
        layout = DescriptorSetLayout([
            DescriptorBinding(binding=0, type="storage"),
            DescriptorBinding(binding=1, type="storage"),
        ])
        assert len(layout.bindings) == 2
        assert layout.bindings[0].binding == 0
        assert layout.bindings[1].binding == 1

    def test_empty_layout(self) -> None:
        layout = DescriptorSetLayout([])
        assert len(layout.bindings) == 0

    def test_uniform_binding(self) -> None:
        layout = DescriptorSetLayout([
            DescriptorBinding(binding=0, type="uniform"),
        ])
        assert layout.bindings[0].type == "uniform"

    def test_unique_ids(self) -> None:
        l1 = DescriptorSetLayout([])
        l2 = DescriptorSetLayout([])
        assert l1.layout_id != l2.layout_id


class TestPipelineLayout:
    def test_basic(self) -> None:
        ds_layout = DescriptorSetLayout([
            DescriptorBinding(binding=0, type="storage"),
        ])
        layout = PipelineLayout([ds_layout], push_constant_size=16)
        assert len(layout.set_layouts) == 1
        assert layout.push_constant_size == 16

    def test_no_push_constants(self) -> None:
        layout = PipelineLayout([])
        assert layout.push_constant_size == 0

    def test_unique_ids(self) -> None:
        l1 = PipelineLayout([])
        l2 = PipelineLayout([])
        assert l1.layout_id != l2.layout_id


class TestPipeline:
    def test_creation(self) -> None:
        shader = ShaderModule(code=[limm(0, 1.0), halt()])
        ds_layout = DescriptorSetLayout([])
        pl_layout = PipelineLayout([ds_layout])
        pipeline = Pipeline(shader, pl_layout)
        assert pipeline.shader is shader
        assert pipeline.layout is pl_layout

    def test_workgroup_size(self) -> None:
        shader = ShaderModule(
            code=[halt()], local_size=(128, 2, 1)
        )
        pl_layout = PipelineLayout([])
        pipeline = Pipeline(shader, pl_layout)
        assert pipeline.workgroup_size == (128, 2, 1)

    def test_unique_ids(self) -> None:
        shader = ShaderModule(code=[halt()])
        pl_layout = PipelineLayout([])
        p1 = Pipeline(shader, pl_layout)
        p2 = Pipeline(shader, pl_layout)
        assert p1.pipeline_id != p2.pipeline_id


class TestDescriptorSet:
    def test_write_and_read(self) -> None:
        device = make_device()
        mm = device.memory_manager
        buf = mm.allocate(64, MemoryType.DEVICE_LOCAL, usage=BufferUsage.STORAGE)

        layout = DescriptorSetLayout([
            DescriptorBinding(binding=0, type="storage"),
        ])
        desc_set = DescriptorSet(layout)
        desc_set.write(0, buf)

        assert desc_set.get_buffer(0) is buf

    def test_multiple_bindings(self) -> None:
        device = make_device()
        mm = device.memory_manager
        buf_x = mm.allocate(64, MemoryType.DEVICE_LOCAL, usage=BufferUsage.STORAGE)
        buf_y = mm.allocate(64, MemoryType.DEVICE_LOCAL, usage=BufferUsage.STORAGE)

        layout = DescriptorSetLayout([
            DescriptorBinding(binding=0, type="storage"),
            DescriptorBinding(binding=1, type="storage"),
        ])
        desc_set = DescriptorSet(layout)
        desc_set.write(0, buf_x)
        desc_set.write(1, buf_y)

        assert desc_set.get_buffer(0) is buf_x
        assert desc_set.get_buffer(1) is buf_y

    def test_invalid_binding(self) -> None:
        device = make_device()
        mm = device.memory_manager
        buf = mm.allocate(64, MemoryType.DEVICE_LOCAL, usage=BufferUsage.STORAGE)

        layout = DescriptorSetLayout([
            DescriptorBinding(binding=0, type="storage"),
        ])
        desc_set = DescriptorSet(layout)
        with pytest.raises(ValueError, match="not in layout"):
            desc_set.write(99, buf)

    def test_freed_buffer(self) -> None:
        device = make_device()
        mm = device.memory_manager
        buf = mm.allocate(64, MemoryType.DEVICE_LOCAL, usage=BufferUsage.STORAGE)
        mm.free(buf)

        layout = DescriptorSetLayout([
            DescriptorBinding(binding=0, type="storage"),
        ])
        desc_set = DescriptorSet(layout)
        with pytest.raises(ValueError, match="freed"):
            desc_set.write(0, buf)

    def test_unbound_returns_none(self) -> None:
        layout = DescriptorSetLayout([
            DescriptorBinding(binding=0, type="storage"),
        ])
        desc_set = DescriptorSet(layout)
        assert desc_set.get_buffer(0) is None

    def test_bindings_dict(self) -> None:
        device = make_device()
        mm = device.memory_manager
        buf = mm.allocate(64, MemoryType.DEVICE_LOCAL, usage=BufferUsage.STORAGE)

        layout = DescriptorSetLayout([
            DescriptorBinding(binding=0, type="storage"),
        ])
        desc_set = DescriptorSet(layout)
        desc_set.write(0, buf)
        bindings = desc_set.bindings
        assert 0 in bindings
        assert bindings[0] is buf

    def test_unique_ids(self) -> None:
        layout = DescriptorSetLayout([])
        d1 = DescriptorSet(layout)
        d2 = DescriptorSet(layout)
        assert d1.set_id != d2.set_id


class TestDeviceFactory:
    """Test pipeline creation through LogicalDevice factory methods."""

    def test_create_shader_module(self) -> None:
        device = make_device()
        shader = device.create_shader_module(
            code=[limm(0, 1.0), halt()],
            local_size=(64, 1, 1),
        )
        assert shader.is_gpu_style
        assert shader.local_size == (64, 1, 1)

    def test_create_dataflow_shader(self) -> None:
        device = make_device()
        shader = device.create_shader_module(operation="matmul")
        assert shader.is_dataflow_style

    def test_create_full_pipeline(self) -> None:
        device = make_device()
        shader = device.create_shader_module(code=[limm(0, 1.0), halt()])
        ds_layout = device.create_descriptor_set_layout([
            DescriptorBinding(binding=0, type="storage"),
        ])
        pl_layout = device.create_pipeline_layout([ds_layout], push_constant_size=4)
        pipeline = device.create_compute_pipeline(shader, pl_layout)

        assert pipeline.shader is shader
        assert pipeline.layout is pl_layout
        assert pipeline.layout.push_constant_size == 4
