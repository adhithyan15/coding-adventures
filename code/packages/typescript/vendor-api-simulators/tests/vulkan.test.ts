/**
 * Tests for the Vulkan runtime simulator.
 */
import { describe, it, expect } from "vitest";
import { halt } from "@coding-adventures/gpu-core";

import {
  VkInstance,
  VkPhysicalDevice,
  VkDevice,
  VkQueue,
  VkCommandPool,
  VkCommandBuffer,
  VkBuffer,
  VkDeviceMemory,
  VkShaderModule,
  VkPipeline,
  VkDescriptorSetLayout,
  VkPipelineLayout,
  VkDescriptorSet,
  VkFence,
  VkSemaphore,
  VkResult,
  VkPipelineBindPoint,
  VkBufferUsageFlagBits,
  VkMemoryPropertyFlagBits,
  VkSharingMode,
} from "../src/index.js";

describe("VkInstance", () => {
  it("creates an instance", () => {
    const instance = new VkInstance();
    expect(instance).toBeDefined();
  });

  it("enumerates physical devices", () => {
    const instance = new VkInstance();
    const devices = instance.vkEnumeratePhysicalDevices();
    expect(devices.length).toBeGreaterThan(0);
    expect(devices[0]).toBeInstanceOf(VkPhysicalDevice);
  });

  it("creates a logical device from physical device", () => {
    const instance = new VkInstance();
    const physicals = instance.vkEnumeratePhysicalDevices();
    const device = instance.vkCreateDevice(physicals[0]);
    expect(device).toBeInstanceOf(VkDevice);
  });
});

describe("VkPhysicalDevice", () => {
  it("getProperties returns device info", () => {
    const instance = new VkInstance();
    const physical = instance.vkEnumeratePhysicalDevices()[0];
    const props = physical.vkGetPhysicalDeviceProperties();
    expect(props.deviceName).toBeDefined();
    expect(props.deviceType).toBeDefined();
    expect(props.vendor).toBeDefined();
  });

  it("getMemoryProperties returns heap info", () => {
    const instance = new VkInstance();
    const physical = instance.vkEnumeratePhysicalDevices()[0];
    const memProps = physical.vkGetPhysicalDeviceMemoryProperties();
    expect(memProps.heapCount).toBeGreaterThan(0);
    expect(memProps.heaps).toBeDefined();
  });

  it("getQueueFamilyProperties returns queue families", () => {
    const instance = new VkInstance();
    const physical = instance.vkEnumeratePhysicalDevices()[0];
    const families = physical.vkGetPhysicalDeviceQueueFamilyProperties();
    expect(families.length).toBeGreaterThan(0);
    expect(families[0].queueType).toBeDefined();
    expect(families[0].queueCount).toBeDefined();
  });
});

describe("VkDevice", () => {
  let instance: InstanceType<typeof VkInstance>;
  let device: InstanceType<typeof VkDevice>;

  function setup() {
    instance = new VkInstance();
    const physicals = instance.vkEnumeratePhysicalDevices();
    device = instance.vkCreateDevice(physicals[0]);
  }

  it("getDeviceQueue returns a VkQueue", () => {
    setup();
    const queue = device.vkGetDeviceQueue(0, 0);
    expect(queue).toBeInstanceOf(VkQueue);
  });

  it("createCommandPool returns a VkCommandPool", () => {
    setup();
    const pool = device.vkCreateCommandPool({ queueFamilyIndex: 0 });
    expect(pool).toBeInstanceOf(VkCommandPool);
  });

  it("createBuffer returns a VkBuffer", () => {
    setup();
    const buffer = device.vkCreateBuffer({
      size: 256,
      usage: VkBufferUsageFlagBits.STORAGE_BUFFER,
      sharingMode: VkSharingMode.EXCLUSIVE,
    });
    expect(buffer).toBeInstanceOf(VkBuffer);
    expect(buffer.size).toBe(256);
  });

  it("allocateMemory returns VkDeviceMemory", () => {
    setup();
    const mem = device.vkAllocateMemory({ size: 512, memoryTypeIndex: 0 });
    expect(mem).toBeInstanceOf(VkDeviceMemory);
  });

  it("allocateMemory with memoryTypeIndex 1", () => {
    setup();
    const mem = device.vkAllocateMemory({ size: 128, memoryTypeIndex: 1 });
    expect(mem).toBeInstanceOf(VkDeviceMemory);
  });

  it("bindBufferMemory is a no-op", () => {
    setup();
    const buffer = device.vkCreateBuffer({
      size: 64,
      usage: VkBufferUsageFlagBits.STORAGE_BUFFER,
      sharingMode: VkSharingMode.EXCLUSIVE,
    });
    const mem = device.vkAllocateMemory({ size: 64, memoryTypeIndex: 0 });
    device.vkBindBufferMemory(buffer, mem, 0);
  });

  it("mapMemory and unmapMemory work", () => {
    setup();
    const mem = device.vkAllocateMemory({ size: 16, memoryTypeIndex: 0 });
    const data = device.vkMapMemory(mem, 0, 16);
    expect(data).toBeInstanceOf(Uint8Array);
    device.vkUnmapMemory(mem);
  });

  it("createShaderModule returns VkShaderModule", () => {
    setup();
    const shader = device.vkCreateShaderModule({ code: null });
    expect(shader).toBeInstanceOf(VkShaderModule);
  });

  it("createDescriptorSetLayout returns layout", () => {
    setup();
    const layout = device.vkCreateDescriptorSetLayout({
      bindings: [{ binding: 0, descriptorType: "storage", descriptorCount: 1 }],
    });
    expect(layout).toBeInstanceOf(VkDescriptorSetLayout);
  });

  it("createPipelineLayout returns layout", () => {
    setup();
    const dsLayout = device.vkCreateDescriptorSetLayout({ bindings: [] });
    const plLayout = device.vkCreatePipelineLayout({
      setLayouts: [dsLayout],
      pushConstantSize: 0,
    });
    expect(plLayout).toBeInstanceOf(VkPipelineLayout);
  });

  it("createComputePipelines returns pipelines", () => {
    setup();
    const shader = device.vkCreateShaderModule({ code: null });
    const dsLayout = device.vkCreateDescriptorSetLayout({ bindings: [] });
    const plLayout = device.vkCreatePipelineLayout({
      setLayouts: [dsLayout],
      pushConstantSize: 0,
    });
    const pipelines = device.vkCreateComputePipelines([
      {
        shaderStage: {
          stage: "compute",
          module: shader,
          entryPoint: "main",
        },
        layout: plLayout,
      },
    ]);
    expect(pipelines.length).toBe(1);
    expect(pipelines[0]).toBeInstanceOf(VkPipeline);
  });

  it("allocateDescriptorSets returns sets", () => {
    setup();
    const dsLayout = device.vkCreateDescriptorSetLayout({
      bindings: [{ binding: 0, descriptorType: "storage", descriptorCount: 1 }],
    });
    const sets = device.vkAllocateDescriptorSets({ setLayouts: [dsLayout] });
    expect(sets.length).toBe(1);
    expect(sets[0]).toBeInstanceOf(VkDescriptorSet);
  });

  it("updateDescriptorSets writes buffer binding", () => {
    setup();
    const buffer = device.vkCreateBuffer({
      size: 64,
      usage: VkBufferUsageFlagBits.STORAGE_BUFFER,
      sharingMode: VkSharingMode.EXCLUSIVE,
    });
    const dsLayout = device.vkCreateDescriptorSetLayout({
      bindings: [{ binding: 0, descriptorType: "storage", descriptorCount: 1 }],
    });
    const sets = device.vkAllocateDescriptorSets({ setLayouts: [dsLayout] });
    device.vkUpdateDescriptorSets([
      {
        dstSet: sets[0],
        dstBinding: 0,
        descriptorType: "storage",
        bufferInfo: { buffer, offset: 0, range: 64 },
      },
    ]);
  });

  it("createFence returns VkFence", () => {
    setup();
    const fence = device.vkCreateFence();
    expect(fence).toBeInstanceOf(VkFence);
  });

  it("createFence signaled returns signaled fence", () => {
    setup();
    const fence = device.vkCreateFence(1);
    expect(fence.signaled).toBe(true);
  });

  it("createSemaphore returns VkSemaphore", () => {
    setup();
    const sem = device.vkCreateSemaphore();
    expect(sem).toBeInstanceOf(VkSemaphore);
  });

  it("waitForFences with signaled fence returns SUCCESS", () => {
    setup();
    const fence = device.vkCreateFence(1);
    const result = device.vkWaitForFences([fence], true, 1000);
    expect(result).toBe(VkResult.SUCCESS);
  });

  it("waitForFences waitAll=false with one signaled returns SUCCESS", () => {
    setup();
    const f1 = device.vkCreateFence(1);
    const f2 = device.vkCreateFence(0);
    const result = device.vkWaitForFences([f1, f2], false, 1000);
    expect(result).toBe(VkResult.SUCCESS);
  });

  it("waitForFences waitAll=true with unsignaled returns NOT_READY", () => {
    setup();
    const f1 = device.vkCreateFence(1);
    const f2 = device.vkCreateFence(0);
    const result = device.vkWaitForFences([f1, f2], true, 1000);
    expect(result).toBe(VkResult.NOT_READY);
  });

  it("resetFences resets signaled fences", () => {
    setup();
    const fence = device.vkCreateFence(1);
    expect(fence.signaled).toBe(true);
    device.vkResetFences([fence]);
    expect(fence.signaled).toBe(false);
  });

  it("deviceWaitIdle completes without error", () => {
    setup();
    device.vkDeviceWaitIdle();
  });
});

describe("VkCommandPool and VkCommandBuffer", () => {
  it("allocateCommandBuffers returns command buffers", () => {
    const instance = new VkInstance();
    const physicals = instance.vkEnumeratePhysicalDevices();
    const device = instance.vkCreateDevice(physicals[0]);
    const pool = device.vkCreateCommandPool({ queueFamilyIndex: 0 });
    const cbs = pool.vkAllocateCommandBuffers(2);
    expect(cbs.length).toBe(2);
    expect(cbs[0]).toBeInstanceOf(VkCommandBuffer);
  });

  it("command buffer begin/end cycle", () => {
    const instance = new VkInstance();
    const physicals = instance.vkEnumeratePhysicalDevices();
    const device = instance.vkCreateDevice(physicals[0]);
    const pool = device.vkCreateCommandPool({ queueFamilyIndex: 0 });
    const [cb] = pool.vkAllocateCommandBuffers(1);
    cb.vkBeginCommandBuffer();
    cb.vkEndCommandBuffer();
  });

  it("command buffer dispatch compute", () => {
    const instance = new VkInstance();
    const physicals = instance.vkEnumeratePhysicalDevices();
    const device = instance.vkCreateDevice(physicals[0]);
    const pool = device.vkCreateCommandPool({ queueFamilyIndex: 0 });
    const [cb] = pool.vkAllocateCommandBuffers(1);

    const shader = device.vkCreateShaderModule({ code: null });
    const dsLayout = device.vkCreateDescriptorSetLayout({ bindings: [] });
    const plLayout = device.vkCreatePipelineLayout({ setLayouts: [dsLayout], pushConstantSize: 0 });
    const [pipeline] = device.vkCreateComputePipelines([{
      shaderStage: { stage: "compute", module: shader, entryPoint: "main" },
      layout: plLayout,
    }]);

    cb.vkBeginCommandBuffer();
    cb.vkCmdBindPipeline(VkPipelineBindPoint.COMPUTE, pipeline);
    cb.vkCmdDispatch(4, 1, 1);
    cb.vkEndCommandBuffer();
  });

  it("command buffer copy and fill", () => {
    const instance = new VkInstance();
    const physicals = instance.vkEnumeratePhysicalDevices();
    const device = instance.vkCreateDevice(physicals[0]);
    const pool = device.vkCreateCommandPool({ queueFamilyIndex: 0 });
    const [cb] = pool.vkAllocateCommandBuffers(1);

    const src = device.vkCreateBuffer({ size: 64, usage: VkBufferUsageFlagBits.TRANSFER_SRC, sharingMode: VkSharingMode.EXCLUSIVE });
    const dst = device.vkCreateBuffer({ size: 64, usage: VkBufferUsageFlagBits.TRANSFER_DST, sharingMode: VkSharingMode.EXCLUSIVE });

    cb.vkBeginCommandBuffer();
    cb.vkCmdFillBuffer(src, 0, 64, 0xab);
    cb.vkCmdCopyBuffer(src, dst, [{ srcOffset: 0, dstOffset: 0, size: 64 }]);
    cb.vkEndCommandBuffer();
  });

  it("command buffer push constants", () => {
    const instance = new VkInstance();
    const physicals = instance.vkEnumeratePhysicalDevices();
    const device = instance.vkCreateDevice(physicals[0]);
    const pool = device.vkCreateCommandPool({ queueFamilyIndex: 0 });
    const [cb] = pool.vkAllocateCommandBuffers(1);

    const dsLayout = device.vkCreateDescriptorSetLayout({ bindings: [] });
    const plLayout = device.vkCreatePipelineLayout({ setLayouts: [dsLayout], pushConstantSize: 16 });

    cb.vkBeginCommandBuffer();
    cb.vkCmdPushConstants(plLayout, 0, new Uint8Array([1, 2, 3, 4]));
    cb.vkEndCommandBuffer();
  });

  it("command buffer bind descriptor sets", () => {
    const instance = new VkInstance();
    const physicals = instance.vkEnumeratePhysicalDevices();
    const device = instance.vkCreateDevice(physicals[0]);
    const pool = device.vkCreateCommandPool({ queueFamilyIndex: 0 });
    const [cb] = pool.vkAllocateCommandBuffers(1);

    const dsLayout = device.vkCreateDescriptorSetLayout({
      bindings: [{ binding: 0, descriptorType: "storage", descriptorCount: 1 }],
    });
    const plLayout = device.vkCreatePipelineLayout({ setLayouts: [dsLayout], pushConstantSize: 0 });
    const sets = device.vkAllocateDescriptorSets({ setLayouts: [dsLayout] });

    cb.vkBeginCommandBuffer();
    cb.vkCmdBindDescriptorSets(VkPipelineBindPoint.COMPUTE, plLayout, sets);
    cb.vkEndCommandBuffer();
  });

  it("command buffer pipeline barrier", () => {
    const instance = new VkInstance();
    const physicals = instance.vkEnumeratePhysicalDevices();
    const device = instance.vkCreateDevice(physicals[0]);
    const pool = device.vkCreateCommandPool({ queueFamilyIndex: 0 });
    const [cb] = pool.vkAllocateCommandBuffers(1);

    cb.vkBeginCommandBuffer();
    cb.vkCmdPipelineBarrier("compute", "compute");
    cb.vkEndCommandBuffer();
  });

  it("resetCommandPool resets all command buffers", () => {
    const instance = new VkInstance();
    const physicals = instance.vkEnumeratePhysicalDevices();
    const device = instance.vkCreateDevice(physicals[0]);
    const pool = device.vkCreateCommandPool({ queueFamilyIndex: 0 });
    pool.vkAllocateCommandBuffers(3);
    pool.vkResetCommandPool();
  });

  it("freeCommandBuffers removes from pool", () => {
    const instance = new VkInstance();
    const physicals = instance.vkEnumeratePhysicalDevices();
    const device = instance.vkCreateDevice(physicals[0]);
    const pool = device.vkCreateCommandPool({ queueFamilyIndex: 0 });
    const cbs = pool.vkAllocateCommandBuffers(2);
    pool.vkFreeCommandBuffers([cbs[0]]);
    expect(pool._commandBuffers.length).toBe(1);
  });
});

describe("VkQueue", () => {
  it("submit with fence signals the fence", () => {
    const instance = new VkInstance();
    const physicals = instance.vkEnumeratePhysicalDevices();
    const device = instance.vkCreateDevice(physicals[0]);
    const queue = device.vkGetDeviceQueue(0, 0);
    const pool = device.vkCreateCommandPool({ queueFamilyIndex: 0 });
    const [cb] = pool.vkAllocateCommandBuffers(1);
    const fence = device.vkCreateFence();

    cb.vkBeginCommandBuffer();
    cb.vkEndCommandBuffer();
    const result = queue.vkQueueSubmit(
      [{ commandBuffers: [cb], waitSemaphores: [], signalSemaphores: [] }],
      fence,
    );
    expect(result).toBe(VkResult.SUCCESS);
  });

  it("submit with semaphores", () => {
    const instance = new VkInstance();
    const physicals = instance.vkEnumeratePhysicalDevices();
    const device = instance.vkCreateDevice(physicals[0]);
    const queue = device.vkGetDeviceQueue(0, 0);
    const pool = device.vkCreateCommandPool({ queueFamilyIndex: 0 });
    const [cb] = pool.vkAllocateCommandBuffers(1);
    const sem = device.vkCreateSemaphore();

    cb.vkBeginCommandBuffer();
    cb.vkEndCommandBuffer();
    const result = queue.vkQueueSubmit(
      [{ commandBuffers: [cb], waitSemaphores: [], signalSemaphores: [sem] }],
    );
    expect(result).toBe(VkResult.SUCCESS);
  });

  it("queueWaitIdle completes", () => {
    const instance = new VkInstance();
    const physicals = instance.vkEnumeratePhysicalDevices();
    const device = instance.vkCreateDevice(physicals[0]);
    const queue = device.vkGetDeviceQueue(0, 0);
    queue.vkQueueWaitIdle();
  });
});
