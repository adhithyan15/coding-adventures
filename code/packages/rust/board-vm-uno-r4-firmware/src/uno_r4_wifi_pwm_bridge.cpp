#include <array>
#include <stddef.h>
#include <stdint.h>
#include <new>

#include "Arduino.h"
#include "hal_data.h"
#include "pwm.h"
#include "pinmux.inc"
#include "r_adc.h"

namespace {

constexpr uint8_t kPwmPins[] = {3, 5, 6, 9, 10, 11};
constexpr uint8_t kAdcPins[] = {14, 15, 16, 17, 18, 19};
constexpr size_t kOperatorNewHeapBytes = 2048;
constexpr uint32_t kAdcRawMax = (1u << BSP_FEATURE_ADC_MAX_RESOLUTION_BITS) - 1u;
constexpr uint32_t kAdcNormalizedMax = 65535u;
constexpr uint32_t kAdcScanPollLimit = 100000u;

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
adc_instance_ctrl_t g_board_vm_adc_ctrl = {};
adc_extended_cfg_t g_board_vm_adc_extend = {};
adc_cfg_t g_board_vm_adc_cfg = {};
adc_channel_cfg_t g_board_vm_adc_channel_cfg = {};
bool g_board_vm_adc_initialized = false;

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
    case PIN_CFG_REQ_ADC:
      return IS_PIN_ANALOG(cfg);
    default:
      return false;
  }
}

bool is_adc_header_pin(uint8_t pin) {
  for (uint8_t candidate : kAdcPins) {
    if (candidate == pin) {
      return true;
    }
  }
  return false;
}

adc_resolution_t adc_hardware_resolution() {
#if 12U == BSP_FEATURE_ADC_MAX_RESOLUTION_BITS
  return ADC_RESOLUTION_12_BIT;
#elif 14U == BSP_FEATURE_ADC_MAX_RESOLUTION_BITS
  return ADC_RESOLUTION_14_BIT;
#elif 16U == BSP_FEATURE_ADC_MAX_RESOLUTION_BITS
  return ADC_RESOLUTION_16_BIT;
#else
#error Unsupported Uno R4 ADC hardware resolution.
#endif
}

void initialize_adc_config() {
  if (g_board_vm_adc_initialized) {
    return;
  }

  g_board_vm_adc_extend.add_average_count = ADC_ADD_OFF;
  g_board_vm_adc_extend.clearing = ADC_CLEAR_AFTER_READ_ON;
  g_board_vm_adc_extend.trigger_group_b = ADC_TRIGGER_SYNC_ELC;
  g_board_vm_adc_extend.double_trigger_mode = ADC_DOUBLE_TRIGGER_DISABLED;
  g_board_vm_adc_extend.adc_vref_control = ADC_VREF_CONTROL_AVCC0_AVSS0;
  g_board_vm_adc_extend.enable_adbuf = 0;
  g_board_vm_adc_extend.window_a_irq = FSP_INVALID_VECTOR;
  g_board_vm_adc_extend.window_b_irq = FSP_INVALID_VECTOR;
  g_board_vm_adc_extend.window_a_ipl = 12;
  g_board_vm_adc_extend.window_b_ipl = 12;

  g_board_vm_adc_cfg.unit = 0;
  g_board_vm_adc_cfg.mode = ADC_MODE_SINGLE_SCAN;
  g_board_vm_adc_cfg.resolution = adc_hardware_resolution();
  g_board_vm_adc_cfg.alignment = ADC_ALIGNMENT_RIGHT;
  g_board_vm_adc_cfg.trigger = ADC_TRIGGER_SOFTWARE;
  g_board_vm_adc_cfg.scan_end_irq = FSP_INVALID_VECTOR;
  g_board_vm_adc_cfg.scan_end_b_irq = FSP_INVALID_VECTOR;
  g_board_vm_adc_cfg.scan_end_ipl = 12;
  g_board_vm_adc_cfg.scan_end_b_ipl = 12;
  g_board_vm_adc_cfg.p_callback = nullptr;
  g_board_vm_adc_cfg.p_context = nullptr;
  g_board_vm_adc_cfg.p_extend = &g_board_vm_adc_extend;

  g_board_vm_adc_channel_cfg.scan_mask = 0;
  g_board_vm_adc_channel_cfg.scan_mask_group_b = 0;
  g_board_vm_adc_channel_cfg.add_mask = 0;
  g_board_vm_adc_channel_cfg.p_window_cfg = nullptr;
  g_board_vm_adc_channel_cfg.priority_group_a = ADC_GROUP_A_PRIORITY_OFF;
  g_board_vm_adc_channel_cfg.sample_hold_mask = 0;
  g_board_vm_adc_channel_cfg.sample_hold_states = 24;

  g_board_vm_adc_initialized = true;
}

uint16_t normalize_adc_sample(uint16_t raw) {
  uint32_t scaled =
      (static_cast<uint32_t>(raw) * kAdcNormalizedMax + (kAdcRawMax / 2u)) / kAdcRawMax;
  return static_cast<uint16_t>(scaled);
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
  {BSP_IO_PORT_00_PIN_14, P014}, /* (14) A0 */
  {BSP_IO_PORT_00_PIN_00, P000}, /* (15) A1 */
  {BSP_IO_PORT_00_PIN_01, P001}, /* (16) A2 */
  {BSP_IO_PORT_00_PIN_02, P002}, /* (17) A3 */
  {BSP_IO_PORT_01_PIN_01, P101}, /* (18) A4/SDA */
  {BSP_IO_PORT_01_PIN_00, P100}, /* (19) A5/SCL */
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

extern "C" bool board_vm_uno_r4_adc_read(uint8_t pin, uint16_t *sample) {
  if (sample == nullptr || !is_adc_header_pin(pin)) {
    return false;
  }

  auto cfg = getPinCfgs(pin, PIN_CFG_REQ_ADC);
  if (cfg[0] == 0) {
    return false;
  }

  initialize_adc_config();

  if (g_board_vm_adc_ctrl.opened) {
    R_ADC_Close(&g_board_vm_adc_ctrl);
  }

  uint8_t channel = GET_CHANNEL(cfg[0]);
  g_board_vm_adc_channel_cfg.scan_mask = 1u << channel;
  g_board_vm_adc_channel_cfg.scan_mask_group_b = 0;
  g_board_vm_adc_channel_cfg.add_mask = 0;
  g_board_vm_adc_channel_cfg.sample_hold_mask = 0;

  fsp_err_t pin_status =
      R_IOPORT_PinCfg(&g_ioport_ctrl, g_pin_cfg[pin].pin, IOPORT_CFG_ANALOG_ENABLE);
  if (pin_status != FSP_SUCCESS) {
    return false;
  }

  if (R_ADC_Open(&g_board_vm_adc_ctrl, &g_board_vm_adc_cfg) != FSP_SUCCESS) {
    return false;
  }
  if (R_ADC_ScanCfg(&g_board_vm_adc_ctrl, &g_board_vm_adc_channel_cfg) != FSP_SUCCESS) {
    return false;
  }
  if (R_ADC_ScanStart(&g_board_vm_adc_ctrl) != FSP_SUCCESS) {
    return false;
  }

  adc_status_t status;
  status.state = ADC_STATE_SCAN_IN_PROGRESS;
  for (uint32_t poll = 0; poll < kAdcScanPollLimit; poll++) {
    if (R_ADC_StatusGet(&g_board_vm_adc_ctrl, &status) != FSP_SUCCESS) {
      return false;
    }
    if (status.state != ADC_STATE_SCAN_IN_PROGRESS) {
      break;
    }
  }
  if (status.state == ADC_STATE_SCAN_IN_PROGRESS) {
    return false;
  }

  uint16_t raw = 0;
  if (R_ADC_Read(&g_board_vm_adc_ctrl, static_cast<adc_channel_t>(channel), &raw) != FSP_SUCCESS) {
    return false;
  }

  *sample = normalize_adc_sample(raw);
  return true;
}
