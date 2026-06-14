use core::ptr::{read_volatile, write_volatile};

const NUM_LEDS: usize = 96;
const FRAMEBUFFER_BYTES: usize = NUM_LEDS / 8;

const LED_PINS: [[usize; 2]; NUM_LEDS] = [
    [7, 3],
    [3, 7],
    [7, 4],
    [4, 7],
    [3, 4],
    [4, 3],
    [7, 8],
    [8, 7],
    [3, 8],
    [8, 3],
    [4, 8],
    [8, 4],
    [7, 0],
    [0, 7],
    [3, 0],
    [0, 3],
    [4, 0],
    [0, 4],
    [8, 0],
    [0, 8],
    [7, 6],
    [6, 7],
    [3, 6],
    [6, 3],
    [4, 6],
    [6, 4],
    [8, 6],
    [6, 8],
    [0, 6],
    [6, 0],
    [7, 5],
    [5, 7],
    [3, 5],
    [5, 3],
    [4, 5],
    [5, 4],
    [8, 5],
    [5, 8],
    [0, 5],
    [5, 0],
    [6, 5],
    [5, 6],
    [7, 1],
    [1, 7],
    [3, 1],
    [1, 3],
    [4, 1],
    [1, 4],
    [8, 1],
    [1, 8],
    [0, 1],
    [1, 0],
    [6, 1],
    [1, 6],
    [5, 1],
    [1, 5],
    [7, 2],
    [2, 7],
    [3, 2],
    [2, 3],
    [4, 2],
    [2, 4],
    [8, 2],
    [2, 8],
    [0, 2],
    [2, 0],
    [6, 2],
    [2, 6],
    [5, 2],
    [2, 5],
    [1, 2],
    [2, 1],
    [7, 10],
    [10, 7],
    [3, 10],
    [10, 3],
    [4, 10],
    [10, 4],
    [8, 10],
    [10, 8],
    [0, 10],
    [10, 0],
    [6, 10],
    [10, 6],
    [5, 10],
    [10, 5],
    [1, 10],
    [10, 1],
    [2, 10],
    [10, 2],
    [7, 9],
    [9, 7],
    [3, 9],
    [9, 3],
    [4, 9],
    [9, 4],
];

const MATRIX_RA_PINS: [(u8, u8); 11] = [
    (0, 3),
    (0, 4),
    (0, 11),
    (0, 12),
    (0, 13),
    (0, 15),
    (2, 4),
    (2, 5),
    (2, 6),
    (2, 12),
    (2, 13),
];

const PORT0_PCNTR1: *mut u32 = 0x4004_0000 as *mut u32;
const PORT2_PCNTR1: *mut u32 = 0x4004_0040 as *mut u32;
const PFS_BASE: usize = 0x4004_0800;
const PMISC_PWPR: *mut u8 = 0x4004_0D03 as *mut u8;

const LED_MATRIX_PORT0_MASK: u32 =
    (1 << 3) | (1 << 4) | (1 << 11) | (1 << 12) | (1 << 13) | (1 << 15);
const LED_MATRIX_PORT2_MASK: u32 = (1 << 4) | (1 << 5) | (1 << 6) | (1 << 12) | (1 << 13);

const PWPR_B0WI: u8 = 1 << 7;
const PWPR_PFSWE: u8 = 1 << 6;
const PFS_DIRECTION_OUTPUT: u32 = 1 << 2;
const PFS_OUTPUT_HIGH: u32 = 1;
const PFS_OUTPUT_LOW: u32 = 0;

pub struct UnoR4WifiLedMatrix {
    framebuffer: [u8; FRAMEBUFFER_BYTES],
    cursor: usize,
    enabled: bool,
}

impl UnoR4WifiLedMatrix {
    pub const fn new() -> Self {
        Self {
            framebuffer: [0; FRAMEBUFFER_BYTES],
            cursor: 0,
            enabled: false,
        }
    }

    pub fn set_frame(&mut self, frame: [u32; 3]) {
        for (index, word) in frame.iter().enumerate() {
            let bytes = word.reverse_bits().to_le_bytes();
            self.framebuffer[(index * 4)..(index * 4 + 4)].copy_from_slice(&bytes);
        }
        self.cursor = 0;
        self.enabled = true;
    }

    pub fn clear(&mut self) {
        self.framebuffer = [0; FRAMEBUFFER_BYTES];
        self.cursor = 0;
        self.enabled = false;
        unsafe {
            clear_matrix_ports();
        }
    }

    pub fn refresh_once(&mut self) {
        if !self.enabled {
            return;
        }

        let index = self.cursor;
        let on = (self.framebuffer[index >> 3] & (1 << (index & 0x07))) != 0;
        unsafe {
            turn_led(index, on);
        }
        self.cursor = (self.cursor + 1) % NUM_LEDS;
    }
}

impl Default for UnoR4WifiLedMatrix {
    fn default() -> Self {
        Self::new()
    }
}

unsafe fn turn_led(index: usize, on: bool) {
    clear_matrix_ports();

    if on {
        let [anode, cathode] = LED_PINS[index];
        let (anode_port, anode_pin) = MATRIX_RA_PINS[anode];
        let (cathode_port, cathode_pin) = MATRIX_RA_PINS[cathode];

        enable_pfs_writes();
        write_volatile(
            pfs(anode_port, anode_pin),
            PFS_DIRECTION_OUTPUT | PFS_OUTPUT_HIGH,
        );
        write_volatile(
            pfs(cathode_port, cathode_pin),
            PFS_DIRECTION_OUTPUT | PFS_OUTPUT_LOW,
        );
        disable_pfs_writes();
    }
}

unsafe fn clear_matrix_ports() {
    write_volatile(
        PORT0_PCNTR1,
        read_volatile(PORT0_PCNTR1) & !LED_MATRIX_PORT0_MASK,
    );
    write_volatile(
        PORT2_PCNTR1,
        read_volatile(PORT2_PCNTR1) & !LED_MATRIX_PORT2_MASK,
    );
}

fn pfs(port: u8, pin: u8) -> *mut u32 {
    (PFS_BASE + (port as usize * 0x40) + (pin as usize * 4)) as *mut u32
}

unsafe fn enable_pfs_writes() {
    let mut pwpr = read_volatile(PMISC_PWPR);
    pwpr &= !PWPR_B0WI;
    write_volatile(PMISC_PWPR, pwpr);
    write_volatile(PMISC_PWPR, pwpr | PWPR_PFSWE);
}

unsafe fn disable_pfs_writes() {
    let mut pwpr = read_volatile(PMISC_PWPR);
    pwpr &= !PWPR_B0WI;
    write_volatile(PMISC_PWPR, pwpr);
    write_volatile(PMISC_PWPR, pwpr & !PWPR_PFSWE);
}
