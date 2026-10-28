use cdp1802_gatelevel::{add8, subtract8, Cdp1802Simulator};

#[test]
fn every_add_and_subtract_tuple_matches_independent_wide_oracle() {
    for left in 0u8..=u8::MAX {
        for right in 0u8..=u8::MAX {
            for carry_or_borrow in [false, true] {
                let wide = u16::from(left) + u16::from(right) + u16::from(carry_or_borrow);
                assert_eq!(
                    add8(left, right, carry_or_borrow),
                    (wide as u8, wide > 0xff)
                );

                let required = u16::from(right) + u16::from(carry_or_borrow);
                assert_eq!(
                    subtract8(left, right, carry_or_borrow),
                    (
                        left.wrapping_sub(right)
                            .wrapping_sub(u8::from(carry_or_borrow)),
                        u16::from(left) >= required,
                    )
                );
            }
        }
    }
}

fn one_instruction(opcode: u8, operand: u8, d: u8, df: bool, memory: u8) -> (u8, bool) {
    let program = if matches!(opcode, 0x7c | 0x7d | 0x7f | 0xf8..=0xfd | 0xff) {
        vec![opcode, operand]
    } else {
        vec![opcode]
    };
    let mut cpu = Cdp1802Simulator::new();
    cpu.load(&program, 0x0100).unwrap();
    let mut state = cpu.get_state();
    state.d = d;
    state.df = df;
    state.x = 1;
    state.registers[1] = 0x2000;
    state.memory[0x2000] = memory;
    cpu.restore(state).unwrap();
    cpu.step().unwrap();
    let state = cpu.get_state();
    (state.d, state.df)
}

#[test]
fn all_alu_operand_orders_and_df_forms_reach_the_shared_equations() {
    assert_eq!(one_instruction(0xf4, 0, 0xfe, false, 3), (1, true));
    assert_eq!(one_instruction(0x74, 0, 0xfe, true, 0), (0xff, false));
    assert_eq!(one_instruction(0x7c, 1, 0xff, true, 0), (1, true));

    assert_eq!(one_instruction(0xf5, 0, 2, true, 7), (5, true));
    assert_eq!(one_instruction(0xf7, 0, 7, true, 2), (5, true));
    assert_eq!(one_instruction(0x75, 0, 2, false, 7), (4, true));
    assert_eq!(one_instruction(0x77, 0, 7, false, 2), (4, true));
    assert_eq!(one_instruction(0x7d, 7, 2, false, 0), (4, true));
    assert_eq!(one_instruction(0x7f, 2, 7, false, 0), (4, true));
    assert_eq!(one_instruction(0xfd, 7, 2, false, 0), (5, true));
    assert_eq!(one_instruction(0xff, 2, 7, false, 0), (5, true));
}

#[test]
fn shifts_use_prestate_df_and_never_read_x_memory() {
    assert_eq!(one_instruction(0xf6, 0, 0x81, false, 0xff), (0x40, true));
    assert_eq!(one_instruction(0xfe, 0, 0x81, false, 0xff), (0x02, true));
    assert_eq!(one_instruction(0x76, 0, 0x02, true, 0xff), (0x81, false));
    assert_eq!(one_instruction(0x7e, 0, 0x80, true, 0xff), (0x01, true));
}
