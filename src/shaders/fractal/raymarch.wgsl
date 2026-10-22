// ============================================================================
// Ray Marching
// ============================================================================

// Sphere intersection for bounding sphere acceleration
// Returns distance to entry point, or -1.0 if no intersection
fn intersect_sphere(origin: vec3<f32>, direction: vec3<f32>, center: vec3<f32>, radius: f32) -> f32 {
    let oc = origin - center;
    let a = dot(direction, direction);
    let b = 2.0 * dot(oc, direction);
    let c = dot(oc, oc) - radius * radius;
    let discriminant = b * b - 4.0 * a * c;

    if (discriminant < 0.0) {
        return -1.0; // No intersection
    }

    let sqrt_discriminant = sqrt(discriminant);
    let t1 = (-b - sqrt_discriminant) / (2.0 * a);
    let t2 = (-b + sqrt_discriminant) / (2.0 * a);

    // Return closest positive intersection
    if (t1 > 0.0) {
        return t1;
    } else if (t2 > 0.0) {
        return t2;
    }

    return -1.0; // Ray origin is beyond sphere
}

struct RayMarchResult {
    hit: bool,
    distance: f32,
    steps: u32,
    position: vec3<f32>,
    material_id: u32,
}

// Conservative per-type bounding-sphere radius for empty-space skipping (QA-014).
// Values are world-space extents pre-scale; the return includes a 1.5x safety
// margin and max(scale, 1) so bigger-than-default `fractal_scale` (which most
// DE functions divide IN by, effectively multiplying world extent) is honoured.
// Err generous: too tight a radius visibly clips geometry; too loose just
// skips less empty space. The pre-QA-014 formula was ~3,250 units for default
// params, guaranteeing the camera was always inside the sphere and the skip
// never fired.
fn bounding_radius_for_type(fractal_type: u32, fractal_scale: f32) -> f32 {
    // Per-family conservative extents (world units, pre-scale). See DE funcs:
    var r: f32 = 2.5;  // safe default for unhandled types
    switch fractal_type {
        // 3D escape-time / IFS fractals (types 13..=24):
        case 13u: { r = 1.5; }                       // Mandelbulb3D  (unit bulb, |z|<2 bailout)
        case 14u: { r = 1.8; }                       // MengerSponge3D (unit cube)
        case 15u: { r = 2.0; }                       // SierpinskiPyramid3D
        case 16u: { r = 2.0; }                       // JuliaSet3D (quat Julia, |q|<2 bailout)
        case 17u: { r = 4.0 * abs(fractal_scale); }  // Mandelbox3D (folds scale with slider)
        case 18u: { r = 2.5; }                       // OctahedralIFS3D
        case 19u: { r = 2.5; }                       // IcosahedralIFS3D
        case 20u: { r = 2.0; }                       // ApollonianGasket3D
        case 21u: { r = 4.0; }                       // Kleinian3D (amazing surface)
        case 22u: { r = 2.0; }                       // HybridMandelbulbJulia3D
        case 23u: { r = 2.0; }                       // QuaternionCubic3D (|q|<2 bailout)
        case 24u: { r = 2.0; }                       // SierpinskiGasket3D
        // 3D strange attractors (types 35..=37): point-cloud extents, each
        // centred on the origin by its DE (Lorenz z-23.5 shift, Rossler
        // envelope-midpoint shift); multiplied by fractal_scale in the DE, so
        // apply max(scale,1) below.
        case 35u: { r = 30.0; }                      // Pickover3D
        case 36u: { r = 60.0; }                      // Lorenz3D (wingspan ~43)
        case 37u: { r = 30.0; }                      // Rossler3D
        default: { r = 2.5; }                        // 2D types / unknown — safe fallback
    }
    return r * max(fractal_scale, 1.0) * 1.5;  // 1.5x safety margin
}

fn ray_march(origin: vec3<f32>, direction: vec3<f32>) -> RayMarchResult {
    var result: RayMarchResult;
    result.hit = false;
    result.distance = 0.0;
    result.steps = 0u;
    result.material_id = 0u;

    // Bounding sphere acceleration: skip empty space when camera is outside sphere.
    // Per-type conservative radius (QA-014): the old iteration-count formula
    // yielded ~3,250 units and the camera was always inside the sphere, so the
    // skip never fired. The fractal's spatial extent is independent of iteration
    // count; radius is now keyed to fractal_type with a safety margin.
    var total_distance = 0.0;
    let bounding_radius = bounding_radius_for_type(uniforms.fractal_type, uniforms.fractal_scale);
    let bounding_center = vec3<f32>(0.0);

    // Check if camera is outside the bounding sphere
    let camera_to_center = length(origin - bounding_center);
    if (camera_to_center > bounding_radius) {
        // Camera is outside - use sphere intersection to skip empty space if ray hits sphere
        let sphere_hit = intersect_sphere(origin, direction, bounding_center, bounding_radius);
        if (sphere_hit > 0.0) {
            // Check if we would hit the floor before the sphere
            var floor_hit = -1.0;
            if (uniforms.show_floor == 1u && abs(direction.y) > 0.0001) {
                let t = (uniforms.floor_height - origin.y) / direction.y;
                if (t > 0.0) {
                    floor_hit = t;
                }
            }

            // Only skip to sphere if floor is farther away or doesn't exist
            if (floor_hit < 0.0 || sphere_hit < floor_hit) {
                total_distance = sphere_hit;
            }
        }
        // If ray misses the sphere, continue with normal ray marching (for floor, etc.)
    }
    // If camera is inside sphere, start ray marching from camera position (total_distance = 0.0)

    for (var i = 0u; i < uniforms.max_steps; i = i + 1u) {
        result.steps = i;
        let pos = origin + direction * total_distance;
        let scene_result = scene_de_with_material(pos, true);
        let dist = scene_result.distance;

        // Determine step size based on mode
        var step_size: f32;
        if (uniforms.use_adaptive_step == 1u) {
            // Adaptive stepping: use distance estimate
            step_size = dist * uniforms.step_multiplier;
        } else {
            // Fixed stepping: constant step size
            step_size = uniforms.fixed_step_size;
        }

        total_distance = total_distance + step_size;

        if (dist < uniforms.min_distance) {
            result.hit = true;
            result.distance = total_distance;
            result.position = pos;
            result.material_id = scene_result.material_id;
            break;
        }

        if (total_distance > uniforms.max_distance) {
            break;
        }
    }

    return result;
}
