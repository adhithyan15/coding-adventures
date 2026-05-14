#include <array>
#include <stddef.h>
#include <stdint.h>
#include <new>

#include "Arduino.h"
#include "pwm.h"
#include "pinmux.inc"

namespace {

constexpr uint8_t kPwmPins[] = {3, 5, 6, 9, 10, 11};
constexpr size_t kOperatorNewHeapBytes = 2048;

struct PwmSlot {
  uint8_t pin;
  alignas(PwmOut) uint8_t storage[sizeof(PwmOut)];
  PwmOut *pwm;
  bool started;
};

alignas(max_align_t) uint8_t g_operator_new_heap[kOperatorNewHeapBytes];
size_t g_operator_new_next = 0;
PwmSlot g_pwm_slots[] = {
  {3, {}, nullptr, false},
  {5, {}, nullptr, false},
  {6, {}, nullptr, false},
  {9, {}, nullptr, false},
  {10, {}, nullptr, false},
  {11, {}, nullptr, false},
};
bool g_pwm_channels_reserved = false;

PwmSlot *find_slot(uint8_t pin) {
  for (auto &slot : g_pwm_slots) {
    if (slot.pin == pin) {
      return &slot;
    }
  }
  return nullptr;
}

size_t align_up(size_t value, size_t align) {
  return (value + align - 1) & ~(align - 1);
}

void *allocate_operator_new(size_t size) {
  if (size == 0) {
    size = 1;
  }

  size_t aligned = align_up(g_operator_new_next, alignof(max_align_t));
  size_t next = aligned + size;
  if (next > kOperatorNewHeapBytes) {
    while (true) {
    }
  }

  g_operator_new_next = next;
  return &g_operator_new_heap[aligned];
}

void reserve_pwm_timer_channels() {
  if (g_pwm_channels_reserved) {
    return;
  }

  for (uint8_t pin : kPwmPins) {
    auto cfg = getPinCfgs(pin, PIN_CFG_REQ_PWM);
    if (cfg[0] != 0) {
      FspTimer::set_initial_timer_channel_as_pwm(GPT_TIMER, GET_CHANNEL(cfg[0]));
    }
  }

  g_pwm_channels_reserved = true;
}

PwmOut *slot_pwm(PwmSlot &slot) {
  if (slot.pwm == nullptr) {
    slot.pwm = new (slot.storage) PwmOut(slot.pin);
  }
  return slot.pwm;
}

bool pin_cfg_matches(uint16_t cfg, PinCfgReq_t req) {
  switch (req) {
    case PIN_CFG_REQ_PWM:
      return IS_PIN_PWM(cfg);
    default:
      return false;
  }
}

}  // namespace

void *operator new(size_t size) {
  return allocate_operator_new(size);
}

void *operator new[](size_t size) {
  return allocate_operator_new(size);
}

void operator delete(void *) noexcept {}

void operator delete[](void *) noexcept {}

void operator delete(void *, size_t) noexcept {}

void operator delete[](void *, size_t) noexcept {}

extern "C" const PinMuxCfg_t g_pin_cfg[] = {
  {BSP_IO_PORT_03_PIN_01, P301}, /* (0) D0 */
  {BSP_IO_PORT_03_PIN_02, P302}, /* (1) D1 */
  {BSP_IO_PORT_01_PIN_04, P104}, /* (2) D2 */
  {BSP_IO_PORT_01_PIN_05, P105}, /* (3) D3~ */
  {BSP_IO_PORT_01_PIN_06, P106}, /* (4) D4 */
  {BSP_IO_PORT_01_PIN_07, P107}, /* (5) D5~ */
  {BSP_IO_PORT_01_PIN_11, P111}, /* (6) D6~ */
  {BSP_IO_PORT_01_PIN_12, P112}, /* (7) D7 */
  {BSP_IO_PORT_03_PIN_04, P304}, /* (8) D8 */
  {BSP_IO_PORT_03_PIN_03, P303}, /* (9) D9~ */
  {BSP_IO_PORT_01_PIN_03, P103}, /* (10) D10~ */
  {BSP_IO_PORT_04_PIN_11, P411}, /* (11) D11~ */
  {BSP_IO_PORT_04_PIN_10, P410}, /* (12) D12 */
  {BSP_IO_PORT_01_PIN_02, P102}, /* (13) D13 */
};

extern "C" ioport_instance_ctrl_t g_ioport_ctrl = {};

extern "C" unsigned int PINCOUNT_fn() {
  return sizeof(g_pin_cfg) / sizeof(g_pin_cfg[0]);
}

int32_t getPinIndex(bsp_io_port_pin_t pin) {
  for (unsigned int index = 0; index < PINCOUNT_fn(); index++) {
    if (g_pin_cfg[index].pin == pin) {
      return static_cast<int32_t>(index);
    }
  }
  return -1;
}

std::array<uint16_t, 3> getPinCfgs(const pin_size_t pin, PinCfgReq_t req) {
  std::array<uint16_t, 3> ret = {0, 0, 0};
  if (pin >= PINCOUNT_fn()) {
    return ret;
  }

  const uint16_t *cfg = g_pin_cfg[pin].list;
  uint8_t out = 0;
  for (uint8_t index = 0; out < ret.size(); index++) {
    uint16_t item = *(cfg + index);
    if (pin_cfg_matches(item, req)) {
      ret[out++] = item;
    }
    if (IS_LAST_ITEM(item)) {
      break;
    }
  }

  return ret;
}

FspTimer *__get_timer_for_channel(int channel) {
  for (auto &slot : g_pwm_slots) {
    if (slot.started && slot.pwm != nullptr &&
        slot.pwm->get_timer()->get_channel() == static_cast<uint32_t>(channel)) {
      return slot.pwm->get_timer();
    }
  }
  return nullptr;
}

extern "C" bool board_vm_uno_r4_pwm_write(uint8_t pin, uint16_t duty) {
  PwmSlot *slot = find_slot(pin);
  if (slot == nullptr) {
    return false;
  }

  reserve_pwm_timer_channels();
  PwmOut *pwm = slot_pwm(*slot);
  if (!slot->started) {
    if (!pwm->begin()) {
      return false;
    }
    slot->started = true;
  }

  float duty_percent = (static_cast<float>(duty) * 100.0f) / 65535.0f;
  return pwm->pulse_perc(duty_percent);
}
