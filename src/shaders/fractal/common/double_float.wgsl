// ============================================================================
// Double-Float Arithmetic (emulated f64 using two f32)
// A double-float number is stored as (hi, lo) where value = hi + lo
// This provides ~14 decimal digits of precision vs ~7 for f32
// ============================================================================

// Double-float structure for complex numbers
struct df2 {
    hi: vec2<f32>,  // High parts (real, imag)
    lo: vec2<f32>,  // Low parts (real, imag)
}

// Quick two-sum: a + b = s + e, where s = fl(a+b) and e = error.
// CORRECTNESS DEPENDS on the WGSL spec's guarantee that floating-point
// operations are NOT reordered or associated (the error term would vanish
// under fast-math reassociation). Do not simplify the algebra — every
// intermediate `let` here is load-bearing for the error-free transform.
fn two_sum(a: f32, b: f32) -> vec2<f32> {
    let s = a + b;
    let v = s - a;
    let e = (a - (s - v)) + (b - v);
    return vec2<f32>(s, e);
}

// Double-float addition with proper error propagation (QA-020: removed the
// simplified `df_add` — it had zero callers and was silently less accurate
// than `df_add_full`, inviting accidental reuse).
fn df_add_full(a_hi: f32, a_lo: f32, b_hi: f32, b_lo: f32) -> vec2<f32> {
    let s1 = two_sum(a_hi, b_hi);
    let s2 = two_sum(a_lo, b_lo);
    let s3 = two_sum(s1.x, s1.y + s2.x);
    return vec2<f32>(s3.x, s3.y + s2.y);
}

// Two-product: a * b = p + e via Dekker split. Correct on all IEEE-754 f32
// hardware without relying on hardware FMA (which naga/backends may lower to
// `a*b + 0`, collapsing the error term to 0 and silently reducing DF to f32).
// Do NOT let any simplification merge the intermediate `let`s — that destroys
// the error-free transformation. (QA-005)
fn two_prod(a: f32, b: f32) -> vec2<f32> {
    let p = a * b;
    let SPLIT = 4097.0; // 2^12 + 1 for f32 (24-bit mantissa)
    let a_t = a * SPLIT;
    let a_hi = a_t - (a_t - a);
    let a_lo = a - a_hi;
    let b_t = b * SPLIT;
    let b_hi = b_t - (b_t - b);
    let b_lo = b - b_hi;
    let e = ((a_hi * b_hi - p) + a_hi * b_lo + a_lo * b_hi) + a_lo * b_lo;
    return vec2<f32>(p, e);
}

// Double-float multiplication: (a_hi, a_lo) * (b_hi, b_lo)
fn df_mul(a_hi: f32, a_lo: f32, b_hi: f32, b_lo: f32) -> vec2<f32> {
    let p = two_prod(a_hi, b_hi);
    let e = a_hi * b_lo + a_lo * b_hi;
    return two_sum(p.x, p.y + e);
}

// Double-float subtraction
fn df_sub(a_hi: f32, a_lo: f32, b_hi: f32, b_lo: f32) -> vec2<f32> {
    return df_add_full(a_hi, a_lo, -b_hi, -b_lo);
}

// Complex double-float addition: a + b
fn df2_add(a: df2, b: df2) -> df2 {
    let r = df_add_full(a.hi.x, a.lo.x, b.hi.x, b.lo.x);
    let i = df_add_full(a.hi.y, a.lo.y, b.hi.y, b.lo.y);
    return df2(vec2<f32>(r.x, i.x), vec2<f32>(r.y, i.y));
}

// Complex double-float multiplication: a * b
// (a + bi)(c + di) = (ac - bd) + (ad + bc)i
fn df2_mul(a: df2, b: df2) -> df2 {
    // Real part: a.real * b.real - a.imag * b.imag
    let ac = df_mul(a.hi.x, a.lo.x, b.hi.x, b.lo.x);
    let bd = df_mul(a.hi.y, a.lo.y, b.hi.y, b.lo.y);
    let real = df_sub(ac.x, ac.y, bd.x, bd.y);

    // Imag part: a.real * b.imag + a.imag * b.real
    let ad = df_mul(a.hi.x, a.lo.x, b.hi.y, b.lo.y);
    let bc = df_mul(a.hi.y, a.lo.y, b.hi.x, b.lo.x);
    let imag = df_add_full(ad.x, ad.y, bc.x, bc.y);

    return df2(vec2<f32>(real.x, imag.x), vec2<f32>(real.y, imag.y));
}

// Complex double-float square: z^2
fn df2_square(z: df2) -> df2 {
    // (a + bi)^2 = (a^2 - b^2) + 2abi
    let a2 = df_mul(z.hi.x, z.lo.x, z.hi.x, z.lo.x);
    let b2 = df_mul(z.hi.y, z.lo.y, z.hi.y, z.lo.y);
    let real = df_sub(a2.x, a2.y, b2.x, b2.y);

    let ab = df_mul(z.hi.x, z.lo.x, z.hi.y, z.lo.y);
    let imag = vec2<f32>(ab.x * 2.0, ab.y * 2.0);  // 2 * ab

    return df2(vec2<f32>(real.x, imag.x), vec2<f32>(real.y, imag.y));
}

// Get magnitude squared of double-float complex (returns f32, sufficient for escape test)
fn df2_mag_sq(z: df2) -> f32 {
    return z.hi.x * z.hi.x + z.hi.y * z.hi.y;
}
