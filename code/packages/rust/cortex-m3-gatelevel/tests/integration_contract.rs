use cortex_m3_gatelevel::{
    encoding,
    integration::{self, AllocationController},
    CortexM3Error, CortexM3GateLevel, FLIP_FLOP_COUNT, MEMORY_SIZE,
};

const ORIGIN: u32 = 0x100;

fn buffer(controller: &AllocationController) -> Vec<u8> {
    controller.memory_buffer().unwrap()
}

#[test]
fn inventory_is_derived_from_live_production_containers() {
    let core = CortexM3GateLevel::new();
    let inventory = integration::storage_inventory(&core);
    assert_eq!(inventory.memory, MEMORY_SIZE * 8);
    assert_eq!(inventory.general_registers, 13 * 32);
    assert_eq!(inventory.stack_link_pc, 4 * 32);
    assert_eq!(inventory.status, 5 + 8 + 8 + 1 + 8 + 1 + 2 + 1);
    assert_eq!(inventory.power_event, 4);
    assert_eq!(inventory.exclusive_monitor, 33);
    assert_eq!(inventory.installed_image, 49);
    assert_eq!(inventory.total(), FLIP_FLOP_COUNT);
}

#[test]
fn delegated_trace_is_non_committing_until_candidate_commit() {
    let controller = AllocationController::new();
    let mut core = CortexM3GateLevel::new();
    core.load(&encoding::program(&[encoding::movs(0, 7)]), ORIGIN)
        .unwrap();
    let authoritative = core.get_state();
    let mut validation = buffer(&controller);

    let trace = integration::delegated_step(
        &core,
        buffer(&controller),
        buffer(&controller),
        &mut validation,
        &controller,
    )
    .unwrap();
    assert_eq!(trace.state_after.registers[0], 7);
    assert_eq!(core.get_state(), authoritative);

    let candidate = integration::settle(trace.state_after).unwrap();
    integration::commit(&mut core, candidate);
    assert_eq!(core.read_register(0).unwrap(), 7);
}

#[test]
fn candidate_divergence_and_failures_cannot_mutate_q() {
    let controller = AllocationController::new();
    let core = CortexM3GateLevel::new();
    let before = core.get_state();
    let mut observer = integration::snapshot_into(&core, buffer(&controller), &controller).unwrap();
    observer.registers[0] = 0xdead_beef;
    assert_eq!(core.get_state(), before);

    observer.memory.pop();
    assert!(matches!(
        integration::settle(observer),
        Err(CortexM3Error::InvalidState { .. })
    ));
    assert_eq!(core.get_state(), before);

    controller.fail_after(0);
    assert_eq!(
        integration::snapshot_into(&core, Vec::with_capacity(MEMORY_SIZE), &controller),
        Err(CortexM3Error::AllocationFailed)
    );
    assert_eq!(core.get_state(), before);
}
