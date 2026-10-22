// Floor distance estimation
fn floor_de(pos: vec3<f32>) -> f32 {
    return pos.y - uniforms.floor_height;
}

// Checkered pattern for floor
fn checkered(pos: vec3<f32>) -> vec3<f32> {
    let scale = 1.0;
    let ix = floor(pos.x / scale);
    let iz = floor(pos.z / scale);
    return select(uniforms.floor_color2, uniforms.floor_color1, (i32(ix) + i32(iz)) % 2 == 0);
}

struct SceneResult {
    distance: f32,
    material_id: u32, // 0 = fractal, 1 = floor
}

// ENH-004 Stage 2: pipeline-overridable constant that selects a single 3D DE
// branch at compile time. The generic 3D pipeline (`fs_main_3d` built without
// an override) leaves this at 0xFFFF, so `ty` below falls back to the runtime
// uniform and every DE branch is present. A specialized pipeline is compiled
// per fractal type with SPECIALIZED_TYPE set to that type's GPU index (13..24,
// 35..37); then the `select` resolves to a compile-time constant, naga folds
// the switch, and the other ~14 DE functions are dead-code-eliminated from
// that pipeline — a per-type register footprint. Semantically a no-op: a
// specialized pipeline is only ever bound while rendering that exact type, so
// SPECIALIZED_TYPE == uniforms.fractal_type at runtime either way (goldens stay
// bit-identical). 0xFFFF is the "unset" sentinel (no 3D type uses it).
override SPECIALIZED_TYPE: u32 = 0xFFFFu;

fn scene_de_with_material(pos: vec3<f32>, conservative: bool) -> SceneResult {
    var result: SceneResult;
    var fractal_dist = 1000.0;

    // 3D fractals start at type 13 (after 13 2D types: 0-12). QA-030: switch
    // is the direct expression of "dispatch on fractal_type" and lets naga
    // build a jump table instead of a linear scan; the default keeps the
    // 1000.0 sentinel for any 2D type that reaches ray_march (shouldn't happen).
    // ENH-004 Stage 2: dispatch on `ty`, which collapses to SPECIALIZED_TYPE
    // (compile-time) when a specialized pipeline overrides the constant.
    let ty = select(uniforms.fractal_type, SPECIALIZED_TYPE, SPECIALIZED_TYPE != 0xFFFFu);
    switch ty {
        case 13u: { fractal_dist = mandelbulb_de(pos); }
        case 14u: { fractal_dist = menger_sponge_de(pos); }
        case 15u: { fractal_dist = sierpinski_pyramid_de(pos); }
        case 16u: { fractal_dist = julia_set_3d_de(pos); }
        case 17u: { fractal_dist = mandelbox_de(pos); }
        case 18u: { fractal_dist = octahedral_ifs_de(pos); }
        case 19u: { fractal_dist = icosahedral_ifs_de(pos); }
        case 20u: { fractal_dist = apollonian_gasket_de(pos); }
        case 21u: { fractal_dist = kleinian_de(pos); }
        case 22u: { fractal_dist = hybrid_mandelbulb_julia_de(pos); }
        case 23u: { fractal_dist = quaternion_cubic_de(pos); }
        case 24u: { fractal_dist = sierpinski_gasket_de(pos); }
        // 3D Strange Attractors (types 35-37)
        case 35u: { fractal_dist = pickover_attractor_de(pos, conservative); }
        case 36u: { fractal_dist = lorenz_attractor_de(pos); }
        case 37u: { fractal_dist = rossler_attractor_de(pos); }
        default: { fractal_dist = 1000.0; }
    }

    var floor_dist = 1000.0;
    if (uniforms.show_floor == 1u) {
        floor_dist = floor_de(pos);
    }

    if (floor_dist < fractal_dist) {
        result.distance = floor_dist;
        result.material_id = 1u;
    } else {
        result.distance = fractal_dist;
        result.material_id = 0u;
    }

    return result;
}

// Exact estimate. Callers that read the magnitude as a proximity signal
// (normals, ambient occlusion, soft shadows) must use this, not the
// conservative bound the ray marcher is allowed to take.
fn scene_de(pos: vec3<f32>) -> f32 {
    return scene_de_with_material(pos, false).distance;
}
