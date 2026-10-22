// ============================================================================
// Lighting Models
// ============================================================================

fn blinn_phong(normal: vec3<f32>, view_dir: vec3<f32>, light_dir: vec3<f32>, albedo: vec3<f32>) -> vec3<f32> {
    let ambient = uniforms.ambient_light;
    let diffuse = max(dot(normal, light_dir), 0.0) * uniforms.light_intensity;

    let half_dir = normalize(light_dir + view_dir);
    let specular = pow(max(dot(normal, half_dir), 0.0), 32.0) * uniforms.light_intensity;

    return albedo * (ambient + diffuse) + vec3<f32>(specular);
}

// PBR Functions
fn distribution_ggx(n_dot_h: f32, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let denom = n_dot_h * n_dot_h * (a2 - 1.0) + 1.0;
    return a2 / (3.14159265 * denom * denom);
}

fn geometry_schlick_ggx(n_dot_v: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;
    return n_dot_v / (n_dot_v * (1.0 - k) + k);
}

fn geometry_smith(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
    let ggx2 = geometry_schlick_ggx(n_dot_v, roughness);
    let ggx1 = geometry_schlick_ggx(n_dot_l, roughness);
    return ggx1 * ggx2;
}

fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - cos_theta, 5.0);
}

fn pbr(normal: vec3<f32>, view_dir: vec3<f32>, light_dir: vec3<f32>, albedo: vec3<f32>, metallic: f32, roughness: f32) -> vec3<f32> {
    let n_dot_v = max(dot(normal, view_dir), 0.0);
    let n_dot_l = max(dot(normal, light_dir), 0.0);
    let half_dir = normalize(view_dir + light_dir);
    let n_dot_h = max(dot(normal, half_dir), 0.0);
    let h_dot_v = max(dot(half_dir, view_dir), 0.0);

    // Calculate F0 (surface reflection at zero incidence)
    var f0 = vec3<f32>(0.04);
    f0 = mix(f0, albedo, metallic);

    // Cook-Torrance BRDF
    let ndf = distribution_ggx(n_dot_h, roughness);
    let g = geometry_smith(n_dot_v, n_dot_l, roughness);
    let f = fresnel_schlick(h_dot_v, f0);

    let numerator = ndf * g * f;
    let denominator = 4.0 * n_dot_v * n_dot_l + 0.001;
    let specular = numerator / denominator;

    let k_s = f;
    var k_d = vec3<f32>(1.0) - k_s;
    k_d = k_d * (1.0 - metallic);

    let radiance = vec3<f32>(uniforms.light_intensity);

    // Ambient light for PBR
    let ambient = albedo * uniforms.ambient_light;

    return (k_d * albedo / 3.14159265 + specular) * radiance * n_dot_l + ambient;
}
