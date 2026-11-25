//! # Microbenchmark Suite and Zero-Allocation Verification (VER07) for `render3d-gpu-bridge`
//!
//! Validates on the real heap via `coding_adventures_test_bootstrap`'s
//! `CountingAllocator`:
//! - zero heap allocation in `execute_render_pass_into` (`ForceSoftware`,
//!   `AutoHardwareWithSoftwareFallback`, and `ForceGpuRuntimeCpu` with
//!   non-identity post-processing)
//! - zero heap allocation in `bridge_frame_digest_fnv1a` and `bridge_depth_digest_fnv1a`
//! - sabotage detection: an intentional 1-byte allocation is caught and rejected.

#![forbid(unsafe_code)]

use std::hint::black_box;
use std::time::Duration;

use coding_adventures_test_bootstrap::bench::{
    AllocCounter, BenchmarkConfig, BenchmarkHarness, BenchmarkResult, Budget, BudgetVerdict,
};
use coding_adventures_test_bootstrap::counting_alloc;
use coding_adventures_test_bootstrap::manifest::{self, Json};
use coding_adventures_test_bootstrap::run::{self, Ctx, Outcome, Suite};
use coding_adventures_test_bootstrap::Verdict;
use math3d::{Mat4, Vec3};
use pixel_container::PixelContainer;
use render3d_gpu_bridge::{
    bridge_depth_digest_fnv1a, bridge_frame_digest_fnv1a, execute_render_pass_into,
    BackendPreference, CameraUniforms, ClipSpace, DepthBufferMut, DrawCall, LightRig,
    MaterialBinding, MeshSlice, PostProcessMode, RenderBridgeScratch, RenderPass, RenderTargetSpec,
    ShadingMode, Texture, Vertex,
};

const FID_BENCH_FAIL: &str = "property-counterexample";

coding_adventures_test_bootstrap::install_counting_allocator!();

fn zero_alloc_ok(result: &BenchmarkResult) -> bool {
    result.budget_verdict.is_acceptable()
        && result.alloc_bytes_per_iter == Some(0.0)
        && !result.unchecked_hard_budgets.contains(&"alloc_bytes")
}

fn fast_bench_config(samples: usize) -> BenchmarkConfig {
    BenchmarkConfig {
        warmup_iterations: 8,
        warmup_max_duration: Duration::from_millis(1),
        min_batch_duration: Duration::from_micros(10),
        max_batch_size: 100_000,
        sample_count: samples,
        seed: 0x9e37_79b9_7f4a_7c15,
    }
}

fn zero_budget() -> Budget {
    Budget {
        max_alloc_bytes: Some(0),
        ..Default::default()
    }
}

fn bench_zero_alloc(name: &str, f: impl FnMut()) -> Verdict {
    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = zero_budget();
    let alloc = AllocCounter::new();
    let res = harness.run_with_counter(name, &budget, &alloc, f);
    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            format!("{name} exceeded zero allocation budget"),
        );
    }
    Verdict::Pass
}

fn build_cube_mesh() -> (Vec<Vertex>, Vec<u32>) {
    let corners = [
        Vec3::new(-0.5, -0.5, -0.5),
        Vec3::new(0.5, -0.5, -0.5),
        Vec3::new(0.5, 0.5, -0.5),
        Vec3::new(-0.5, 0.5, -0.5),
        Vec3::new(-0.5, -0.5, 0.5),
        Vec3::new(0.5, -0.5, 0.5),
        Vec3::new(0.5, 0.5, 0.5),
        Vec3::new(-0.5, 0.5, 0.5),
    ];
    let faces = [
        ([4, 5, 6, 7], Vec3::unit_z(), [0.9, 0.2, 0.2]),
        ([1, 0, 3, 2], Vec3::new(0.0, 0.0, -1.0), [0.2, 0.9, 0.2]),
        ([5, 1, 2, 6], Vec3::unit_x(), [0.2, 0.4, 0.9]),
        ([0, 4, 7, 3], Vec3::new(-1.0, 0.0, 0.0), [0.9, 0.8, 0.2]),
        ([7, 6, 2, 3], Vec3::unit_y(), [0.8, 0.2, 0.9]),
        ([0, 1, 5, 4], Vec3::new(0.0, -1.0, 0.0), [0.2, 0.8, 0.9]),
    ];
    let mut verts = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);
    for (idx, normal, color) in faces {
        let base = verts.len() as u32;
        verts.push(Vertex::vertex_new(corners[idx[0]], normal, color, 0.0, 0.0));
        verts.push(Vertex::vertex_new(corners[idx[1]], normal, color, 1.0, 0.0));
        verts.push(Vertex::vertex_new(corners[idx[2]], normal, color, 1.0, 1.0));
        verts.push(Vertex::vertex_new(corners[idx[3]], normal, color, 0.0, 1.0));
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    (verts, indices)
}

fn check_counting_allocator_is_live(_ctx: &Ctx) -> Verdict {
    if !counting_alloc::is_installed() {
        return Verdict::fail(FID_BENCH_FAIL, "counting allocator is not installed");
    }
    Verdict::Pass
}

fn check_execute_software_f64_zero_alloc(_ctx: &Ctx) -> Verdict {
    let (verts, indices) = build_cube_mesh();
    let mut fb = PixelContainer::new(64, 64);
    let mut zb = vec![1.0_f64; 64 * 64];
    let mut scratch =
        RenderBridgeScratch::bridge_scratch_with_capacity(verts.len(), 64, 64).expect("scratch");

    let view = Mat4::look_at(Vec3::new(1.5, 1.2, 3.0), Vec3::zero(), Vec3::unit_y()).expect("view");
    let proj = Mat4::perspective(std::f64::consts::FRAC_PI_3, 1.0, 0.25, 32.0).expect("proj");
    let camera =
        CameraUniforms::camera_from_view_proj(view, proj, Vec3::new(1.5, 1.2, 3.0)).expect("cam");
    let target = RenderTargetSpec::target_spec_new(64, 64, [10, 12, 16, 255], 1.0).expect("spec");
    let meshes = [MeshSlice::mesh_slice_new(&verts, &indices).expect("mesh")];
    let materials = [MaterialBinding::material_new(ShadingMode::Gouraud)];
    let draw_calls =
        [DrawCall::draw_call_new(0, 0, Mat4::mat4_identity(), 0, indices.len()).expect("dc")];
    let pass = RenderPass {
        target,
        camera,
        light: LightRig::light_rig_default(),
        meshes: &meshes,
        materials: &materials,
        draw_calls: &draw_calls,
    };

    bench_zero_alloc("execute_software_f64_zero_alloc", || {
        let rep = execute_render_pass_into(
            black_box(&pass),
            BackendPreference::ForceSoftware,
            black_box(&mut fb),
            DepthBufferMut::F64(black_box(&mut zb)),
            black_box(&mut scratch),
        )
        .expect("execute should succeed");
        black_box(rep);
    })
}

fn check_execute_auto_fallback_f32_textured_zero_alloc(_ctx: &Ctx) -> Verdict {
    let (verts, indices) = build_cube_mesh();
    let mut fb = PixelContainer::new(64, 64);
    let mut zb = vec![1.0_f32; 64 * 64];
    let mut scratch =
        RenderBridgeScratch::bridge_scratch_with_capacity(verts.len(), 64, 64).expect("scratch");
    let mut tex_pc = PixelContainer::new(16, 16);
    tex_pc.fill(190, 140, 80, 255);
    let tex = Texture::texture_from_container(&tex_pc).expect("tex");

    let view = Mat4::look_at(Vec3::new(1.2, 1.0, 3.2), Vec3::zero(), Vec3::unit_y()).expect("view");
    let proj = Mat4::perspective(std::f64::consts::FRAC_PI_3, 1.0, 0.25, 32.0).expect("proj");
    let camera =
        CameraUniforms::camera_from_view_proj(view, proj, Vec3::new(1.2, 1.0, 3.2)).expect("cam");
    let target = RenderTargetSpec::target_spec_new(64, 64, [0, 0, 0, 255], 1.0).expect("spec");
    let meshes = [MeshSlice::mesh_slice_new(&verts, &indices).expect("mesh")];
    let materials = [MaterialBinding::material_with_texture(
        ShadingMode::TexturedBilinear,
        tex,
    )];
    let draw_calls =
        [DrawCall::draw_call_new(0, 0, Mat4::mat4_identity(), 0, indices.len()).expect("dc")];
    let pass = RenderPass {
        target,
        camera,
        light: LightRig::light_rig_default(),
        meshes: &meshes,
        materials: &materials,
        draw_calls: &draw_calls,
    };

    bench_zero_alloc("execute_auto_fallback_f32_textured_zero_alloc", || {
        let rep = execute_render_pass_into(
            black_box(&pass),
            BackendPreference::AutoHardwareWithSoftwareFallback,
            black_box(&mut fb),
            DepthBufferMut::F32(black_box(&mut zb)),
            black_box(&mut scratch),
        )
        .expect("execute should succeed");
        black_box(rep);
    })
}

fn check_execute_gpu_runtime_cpu_postprocess_zero_alloc(_ctx: &Ctx) -> Verdict {
    let (verts, indices) = build_cube_mesh();
    let mut fb = PixelContainer::new(64, 64);
    let mut zb = vec![1.0_f64; 64 * 64];
    let mut scratch =
        RenderBridgeScratch::bridge_scratch_with_capacity(verts.len(), 64, 64).expect("scratch");

    let view = Mat4::look_at(Vec3::new(1.2, 1.4, 3.0), Vec3::zero(), Vec3::unit_y()).expect("view");
    let proj = Mat4::perspective_zo(std::f64::consts::FRAC_PI_3, 1.0, 0.25, 32.0).expect("proj zo");
    let camera =
        CameraUniforms::camera_from_view_proj(view, proj, Vec3::new(1.2, 1.4, 3.0)).expect("cam");
    let target = RenderTargetSpec {
        width: 64,
        height: 64,
        clear_color: [16, 24, 32, 255],
        clear_depth: 1.0,
        clip_space: ClipSpace::ZeroToOne,
        post_process: PostProcessMode::ExposureGainBias {
            gain_q8: 280,
            bias: 4,
        },
    };
    let meshes = [MeshSlice::mesh_slice_new(&verts, &indices).expect("mesh")];
    let materials = [MaterialBinding::material_new(ShadingMode::Phong)];
    let draw_calls =
        [DrawCall::draw_call_new(0, 0, Mat4::mat4_identity(), 0, indices.len()).expect("dc")];
    let pass = RenderPass {
        target,
        camera,
        light: LightRig::light_rig_default(),
        meshes: &meshes,
        materials: &materials,
        draw_calls: &draw_calls,
    };

    bench_zero_alloc("execute_gpu_runtime_cpu_postprocess_zero_alloc", || {
        let rep = execute_render_pass_into(
            black_box(&pass),
            BackendPreference::ForceGpuRuntimeCpu,
            black_box(&mut fb),
            DepthBufferMut::F64(black_box(&mut zb)),
            black_box(&mut scratch),
        )
        .expect("execute should succeed");
        black_box(rep);
    })
}

fn check_digests_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut fb = PixelContainer::new(64, 64);
    fb.fill(10, 20, 30, 255);
    let mut zb = vec![0.5_f64; 64 * 64];
    bench_zero_alloc("bridge_digests_zero_alloc", || {
        let df = bridge_frame_digest_fnv1a(black_box(&fb));
        let dd = bridge_depth_digest_fnv1a(black_box(&DepthBufferMut::F64(&mut zb)));
        black_box((df, dd));
    })
}

fn check_sabotage_allocation_detected(_ctx: &Ctx) -> Verdict {
    let harness = BenchmarkHarness::new(fast_bench_config(5));
    let budget = zero_budget();
    let counter = AllocCounter::new();
    let leaky = harness.run_with_counter("sabotage_alloc", &budget, &counter, || {
        black_box(Vec::<u8>::with_capacity(1))
    });
    if !matches!(
        leaky.budget_verdict,
        BudgetVerdict::Failed {
            observed: 1,
            limit: 0,
            ..
        }
    ) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "a closure allocating one byte did not fail the zero-allocation budget",
        );
    }
    let clean = harness.run_with_counter("clean_alloc", &budget, &counter, || {
        black_box(1u64) ^ black_box(2u64)
    });
    if !zero_alloc_ok(&clean) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "a non-allocating closure did not pass the zero-allocation budget",
        );
    }
    Verdict::Pass
}

fn write_bench_manifest(outcomes: &[Outcome]) {
    let Some(path) = manifest::output_path("WARRANT_BENCH_OUT") else {
        return;
    };
    let mut tests: Vec<Json> = outcomes
        .iter()
        .map(|o| {
            Json::obj([
                ("id", Json::str(o.id.clone())),
                ("kind", Json::str("bench")),
                (
                    "verdict",
                    Json::str(if o.passed() { "pass" } else { "fail" }),
                ),
            ])
        })
        .collect();
    tests.sort_by_key(|t| t.render());
    let doc = Json::obj([
        ("schema", Json::str("warrant-bench-evidence-v1")),
        ("module", Json::str("render3d-gpu-bridge")),
        ("hw_instruction_counters", Json::str("not-live")),
        (
            "source_sha256",
            Json::str(coding_adventures_sha256::sha256_hex(
                &manifest::source_tree_framing(
                    &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
                )
                .expect("bind bench manifest to src"),
            )),
        ),
        ("tests", Json::Arr(tests)),
    ]);
    manifest::write(&path, &doc).expect("write bench manifest");
}

fn main() {
    let mut suite = Suite::new();

    suite.add(
        "bench/alloc/counting_allocator_is_live",
        check_counting_allocator_is_live,
    );
    suite.add(
        "bench/execute_render_pass_into/software_f64_zero_alloc",
        check_execute_software_f64_zero_alloc,
    );
    suite.add(
        "bench/execute_render_pass_into/auto_fallback_f32_textured_zero_alloc",
        check_execute_auto_fallback_f32_textured_zero_alloc,
    );
    suite.add(
        "bench/execute_render_pass_into/gpu_runtime_cpu_postprocess_zero_alloc",
        check_execute_gpu_runtime_cpu_postprocess_zero_alloc,
    );
    suite.add("bench/bridge_digests/zero_alloc", check_digests_zero_alloc);
    suite.add(
        "bench/sabotage/allocation_detected",
        check_sabotage_allocation_detected,
    );

    std::process::exit(run::main_recorded(&suite, write_bench_manifest));
}
