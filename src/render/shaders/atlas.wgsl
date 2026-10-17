// Deep Space Atlas — procedural scene and post-processing shader.
//
// The atlas does not load textures. Every scene is drawn analytically from
// the uniform block: a starfield, a stellar surface, an orbital system, a
// nebula, a spiral galaxy, a quasar and a lensed black hole. All colour is
// produced in linear space and tone-mapped in the final pass.

struct Uniforms {
    // scene width, scene height, aspect, time (seconds)
    res: vec4<f32>,
    // mode_a, mode_b, transition blend, quality (0 mobile .. 1 desktop)
    modes: vec4<f32>,
    // primary rgb, energy
    col_a: vec4<f32>,
    col_b: vec4<f32>,
    // yaw, pitch, distance, tan(fov/2)
    cam_a: vec4<f32>,
    cam_b: vec4<f32>,
    // accent rgb, highlight
    accent_a: vec4<f32>,
    accent_b: vec4<f32>,
    // pointer x, pointer y, drift, motion scale
    pointer: vec4<f32>,
    // bloom threshold, bloom strength, grain, vignette
    post: vec4<f32>,
    // phase a, phase b, seed, transition veil
    phase: vec4<f32>,
    // output width, output height, 1/width, 1/height
    screen: vec4<f32>,
    // 1/scene w, 1/scene h, 1/bloom w, 1/bloom h
    texel: vec4<f32>,
    // composition centre x, y, scale, mobile flag
    composition: vec4<f32>,
    // encode sRGB, reduced motion, boot fade, reserved
    flags: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var scene_tex: texture_2d<f32>;
@group(0) @binding(2) var bloom_tex: texture_2d<f32>;
@group(0) @binding(3) var samp: sampler;

// ---------------------------------------------------------------------------
// Output of the fullscreen triangle.

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_fullscreen(@builtin(vertex_index) index: u32) -> VsOut {
    var corners = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let xy = corners[index];
    var out: VsOut;
    out.pos = vec4<f32>(xy, 0.0, 1.0);
    out.uv = vec2<f32>(xy.x * 0.5 + 0.5, 1.0 - (xy.y * 0.5 + 0.5));
    return out;
}

// ---------------------------------------------------------------------------
// Hashing and procedural noise (Dave Hoskins style hashes).

fn hash11(x: f32) -> f32 {
    var p = fract(x * 0.1031);
    p = p + p * (p + 33.33);
    return fract(p * (p + p));
}

fn hash21(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn hash31(p: vec3<f32>) -> vec3<f32> {
    var p3 = fract(p * 0.1031);
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.xxy + p3.yzz) * p3.zyx);
}

fn noise3(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let w = f * f * (3.0 - 2.0 * f);

    let n000 = hash31(i + vec3<f32>(0.0, 0.0, 0.0)).x;
    let n100 = hash31(i + vec3<f32>(1.0, 0.0, 0.0)).x;
    let n010 = hash31(i + vec3<f32>(0.0, 1.0, 0.0)).x;
    let n110 = hash31(i + vec3<f32>(1.0, 1.0, 0.0)).x;
    let n001 = hash31(i + vec3<f32>(0.0, 0.0, 1.0)).x;
    let n101 = hash31(i + vec3<f32>(1.0, 0.0, 1.0)).x;
    let n011 = hash31(i + vec3<f32>(0.0, 1.0, 1.0)).x;
    let n111 = hash31(i + vec3<f32>(1.0, 1.0, 1.0)).x;

    let nx00 = mix(n000, n100, w.x);
    let nx10 = mix(n010, n110, w.x);
    let nx01 = mix(n001, n101, w.x);
    let nx11 = mix(n011, n111, w.x);
    let nxy0 = mix(nx00, nx10, w.y);
    let nxy1 = mix(nx01, nx11, w.y);
    return mix(nxy0, nxy1, w.z);
}

fn noise2(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let w = f * f * (3.0 - 2.0 * f);

    let a = hash21(i);
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));

    return mix(mix(a, b, w.x), mix(c, d, w.x), w.y);
}

fn fbm2(p: vec2<f32>) -> f32 {
    var value = 0.0;
    var amp = 0.5;
    var q = p;
    for (var i = 0; i < 4; i = i + 1) {
        value = value + noise2(q) * amp;
        q = q * 2.03 + vec2<f32>(13.1, 7.7);
        amp = amp * 0.5;
    }
    return value;
}

fn fbm3(p: vec3<f32>) -> f32 {
    var value = 0.0;
    var amp = 0.5;
    var q = p;
    for (var i = 0; i < 4; i = i + 1) {
        value = value + noise3(q) * amp;
        q = q * 2.02 + vec3<f32>(11.3, 5.1, 7.9);
        amp = amp * 0.5;
    }
    return value;
}

// ---------------------------------------------------------------------------
// Camera.

fn camera_pos(yaw: f32, pitch: f32, dist: f32) -> vec3<f32> {
    let cp = cos(pitch);
    return vec3<f32>(cp * sin(yaw), sin(pitch), cp * cos(yaw)) * dist;
}

fn centered_uv(uv: vec2<f32>) -> vec2<f32> {
    // Screen-space coordinates, origin at the composition centre, y down.
    return (uv - u.composition.xy) * vec2<f32>(u.res.z, 1.0) * u.composition.z;
}

fn camera_ray(ro: vec3<f32>, fov: f32, p: vec2<f32>) -> vec3<f32> {
    let forward = normalize(-ro);
    var right = cross(vec3<f32>(0.0, 1.0, 0.0), forward);
    if (length(right) < 0.0001) {
        right = vec3<f32>(1.0, 0.0, 0.0);
    }
    right = normalize(right);
    let up = cross(forward, right);
    let dir = forward + right * (p.x * fov) - up * (p.y * fov);
    return normalize(dir);
}

// ---------------------------------------------------------------------------
// Star fields.

fn star_grid(dir: vec3<f32>, density: f32, seed: f32, time: f32, motion: f32) -> vec3<f32> {
    let a = abs(dir);
    var uv = vec2<f32>(0.0, 0.0);
    var face = 0.0;

    if (a.x >= a.y && a.x >= a.z) {
        uv = dir.yz / max(a.x, 0.0001);
        face = select(1.0, 0.0, dir.x > 0.0);
    } else if (a.y >= a.z) {
        uv = dir.xz / max(a.y, 0.0001);
        face = select(3.0, 2.0, dir.y > 0.0);
    } else {
        uv = dir.xy / max(a.z, 0.0001);
        face = select(5.0, 4.0, dir.z > 0.0);
    }

    let grid = uv * density;
    let cell = floor(grid);
    let fr = grid - cell;
    let h = hash31(vec3<f32>(cell.x, cell.y, face * 37.0 + seed));

    // Only the brightest cells become stars, giving an irregular sky.
    let keep = step(0.865, h.x);
    let mag = pow(max(h.x - 0.865, 0.0) / 0.135, 2.2) * keep;

    let centre = vec2<f32>(0.18 + 0.64 * h.y, 0.18 + 0.64 * h.z);
    let d = fr - centre;
    let r2 = 0.00035 + 0.0018 * h.y * h.y;
    let core = exp(-dot(d, d) / r2);

    let twinkle = 1.0 + 0.16 * sin(time * (0.7 + h.y * 2.4) + h.z * 37.0) * motion;
    let tint = mix(vec3<f32>(0.60, 0.75, 1.0), vec3<f32>(1.0, 0.84, 0.63), h.y * h.y);
    var col = tint * mag * core * 1.9 * twinkle;

    // Diffraction glints on the brightest few.
    let glint = step(0.965, h.x);
    if (glint > 0.0) {
        let cross = exp(-abs(d.x) * 130.0) * exp(-abs(d.y) * 21.0)
            + exp(-abs(d.y) * 130.0) * exp(-abs(d.x) * 21.0);
        col = col + tint * cross * mag * 0.55;
    }
    return col;
}

fn deep_stars(rd: vec3<f32>, time: f32, seed: f32, motion: f32) -> vec3<f32> {
    var col = star_grid(rd, 31.0, seed, time, motion) * 1.05;
    col = col + star_grid(rd, 71.0, seed + 11.0, time, motion) * 0.62;
    col = col + star_grid(rd, 143.0, seed + 29.0, time, motion) * 0.38;

    // Faint galactic haze, slightly banded.
    let bn = normalize(vec3<f32>(sin(seed * 1.7), 0.62, cos(seed * 1.3)));
    let x = dot(rd, bn) * 2.4;
    let band = exp(-x * x);
    let dust = fbm3(rd * 3.1 + vec3<f32>(seed, seed * 0.7, 0.0));
    col = col + mix(vec3<f32>(0.05, 0.09, 0.22), vec3<f32>(0.16, 0.12, 0.24), dust)
        * band * (0.06 + 0.20 * dust);
    return col;
}

// ---------------------------------------------------------------------------
// Shared helpers.

fn sphere_t(ro: vec3<f32>, rd: vec3<f32>, centre: vec3<f32>, radius: f32) -> f32 {
    let oc = ro - centre;
    let b = dot(oc, rd);
    let c = dot(oc, oc) - radius * radius;
    let h = b * b - c;
    if (h < 0.0) {
        return -1.0;
    }
    let t = -b - sqrt(h);
    return t;
}

fn origin_distance(ro: vec3<f32>, rd: vec3<f32>) -> f32 {
    return length(cross(ro, rd));
}

fn aces(x: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

// ---------------------------------------------------------------------------
// 0 — Prologue: a quiet field, dust, a faint warm breath of light.

fn scene_prologue(rd: vec3<f32>, p: vec2<f32>, time: f32, accent: vec3<f32>, phase: f32) -> vec3<f32> {
    let motion = u.pointer.w;
    var col = deep_stars(rd, time, 3.7 + phase, motion) * 0.88;

    // A single soft breath of dust, kept close to the floor of perception.
    let q = p * 1.55 + vec2<f32>(sin(time * 0.013), cos(time * 0.011)) * 0.07 * motion;
    let w = vec2<f32>(fbm2(q * 1.1), fbm2(q * 1.1 + 9.2));
    let n = fbm2(q * 2.0 + w * 1.5);
    let dist = length((p - vec2<f32>(-0.44, 0.14)) * vec2<f32>(0.78, 1.15));
    let cloud = exp(-dist * 2.15) * smoothstep(0.34, 0.82, n);
    col = col + mix(vec3<f32>(0.012, 0.030, 0.085), vec3<f32>(0.055, 0.105, 0.225), n)
        * cloud * 0.22;

    let warm = exp(-length((p - vec2<f32>(0.54, -0.30)) * vec2<f32>(1.15, 1.55)) * 2.9);
    col = col + vec3<f32>(0.34, 0.17, 0.085) * warm * smoothstep(0.35, 0.80, n) * 0.055;

    // The hairline of an instrument, not a decoration.
    let arc_r = length((p - vec2<f32>(0.56, -0.30)) * vec2<f32>(1.0, 1.3));
    let arc = exp(-pow((arc_r - 0.96) * 520.0, 2.0));
    col = col + accent * arc * 0.032;

    col = col + accent * exp(-length(p * vec2<f32>(0.52, 0.82)) * 3.2) * 0.010;
    return col;
}

// ---------------------------------------------------------------------------
// 1 — Star: a granulated photosphere, a thin chromosphere and corona.

fn hash33(p: vec3<f32>) -> vec3<f32> {
    var p3 = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    p3 = p3 + dot(p3, p3.yxz + 33.33);
    return fract((p3.xxy + p3.yzz) * p3.zyx);
}

fn worley3(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    var md = 1.0;
    for (var z = -1; z <= 1; z = z + 1) {
        for (var y = -1; y <= 1; y = y + 1) {
            for (var x = -1; x <= 1; x = x + 1) {
                let g = vec3<f32>(f32(x), f32(y), f32(z));
                let o = hash33(i + g);
                let r = g + o - f;
                md = min(md, dot(r, r));
            }
        }
    }
    return sqrt(md);
}

fn scene_star(ro: vec3<f32>, rd: vec3<f32>, p: vec2<f32>, time: f32, accent: vec3<f32>, colr: vec3<f32>, phase: f32) -> vec3<f32> {
    let motion = u.pointer.w;
    var col = deep_stars(rd, time, 8.1 + phase, motion) * 0.34;
    let t = sphere_t(ro, rd, vec3<f32>(0.0, 0.0, 0.0), 1.0);
    let b = origin_distance(ro, rd);

    if (t > 0.0) {
        let hit = ro + rd * t;
        let n = normalize(hit);
        let mu = clamp(dot(n, -rd), 0.0, 1.0);

        let slow = time * 0.010 * motion;
        // Two scales of convection cells, with dark intergranular lanes.
        let warp = fbm3(n * 3.4 + vec3<f32>(slow * 0.7, slow * 0.3, 0.0)) - 0.5;
        let celln = 1.0 - worley3(n * 18.0 + warp * 2.2 + vec3<f32>(slow, -slow * 0.6, slow * 0.4));
        let soft = fbm3(n * 12.0 + warp * 1.2);
        var fine = 0.55;
        if (u.modes.w > 0.6) {
            fine = 1.0 - worley3(n * 46.0 + warp * 1.0 + vec3<f32>(0.0, slow * 1.7, 0.0));
        }
        let gran = smoothstep(0.15, 0.85, celln * 0.52 + soft * 0.34 + fine * 0.14);
        let lanes = smoothstep(0.05, 0.60, fine);
        let plasma = fbm3(n * 11.0 + vec3<f32>(slow * 2.0, 0.0, 0.0));

        let spotn = fbm3(n * 3.8 + vec3<f32>(17.0, 3.0, 8.0));
        let spot_mask = smoothstep(0.635, 0.680, spotn);
        let penumbra = smoothstep(0.595, 0.635, spotn);

        let limb = pow(mu, 0.95);
        var surf = mix(vec3<f32>(0.36, 0.015, 0.000), vec3<f32>(1.0, 0.70, 0.30), limb);
        surf = surf * (0.64 + 0.46 * gran) * (0.76 + 0.24 * lanes);
        surf = surf * (0.90 + 0.20 * plasma);
        surf = surf * (1.0 - penumbra * 0.28 - spot_mask * 0.50);

        // Faculae: bright knots in the lanes.
        let fac = smoothstep(0.72, 0.96, fbm3(n * 30.0 + vec3<f32>(3.0))) * (0.35 + 0.65 * (1.0 - lanes));
        surf = surf + vec3<f32>(1.0, 0.52, 0.14) * fac * 0.30;

        let rim = pow(1.0 - mu, 12.0);
        surf = surf + vec3<f32>(1.0, 0.24, 0.02) * rim * 0.34;
        col = surf * 1.10;
    }

    let outside = max(b - 1.0, 0.0);
    let ang = atan2(p.y, p.x);

    // Corona: a warm, uneven crown rather than a uniform halo.
    let rays = fbm2(vec2<f32>(ang * 3.2, time * 0.02)) * fbm2(vec2<f32>(ang * 9.0 + 4.0, 0.0));
    let corona = (exp(-outside * 8.2) + 0.22 * exp(-outside * 2.4)) * (0.18 + 0.82 * rays);
    col = col + mix(colr, vec3<f32>(1.0, 0.48, 0.12), 0.72) * corona * 0.26;

    // The chromosphere: a hot line with fine spicules.
    let spic = 0.55 + 0.45 * sin(ang * 150.0 + fbm2(vec2<f32>(ang * 4.0, time * 0.07)) * 12.0);
    col = col + vec3<f32>(1.0, 0.36, 0.06) * exp(-abs(b - 1.006) * 460.0) * 0.52 * spic;

    // A few uneven prominences, not a necklace.
    let sector = smoothstep(0.40, 0.76, fbm2(vec2<f32>(ang * 0.9 + 4.0, time * 0.03)));
    let sn = fbm2(vec2<f32>(ang * 2.6, time * 0.05));
    let stream = pow(max(0.0, sin(ang * 7.0 + sn * 11.0)), 3.0);
    let streamer = stream * exp(-abs(b - 1.11) * 60.0) * sector;
    col = col + mix(colr, accent, 0.4) * streamer * 0.18;

    let prom = smoothstep(0.58, 0.90, fbm2(vec2<f32>(ang * 1.8 + 2.0, time * 0.04)));
    col = col + vec3<f32>(1.0, 0.24, 0.03) * prom * exp(-abs(b - 1.035) * 170.0) * 0.62;

    let halo = exp(-outside * 2.4);
    col = col + accent * halo * (0.010 + 0.016 * fbm2(vec2<f32>(ang * 1.7, time * 0.02)));

    return col;
}

// ---------------------------------------------------------------------------
// 2 — System: a star, inclined orbits and three worlds.

fn orbit_planet_position(radius: f32, angle: f32, incl: f32) -> vec3<f32> {
    let x = cos(angle) * radius;
    let z = sin(angle) * radius;
    return vec3<f32>(x, -z * sin(incl), z * cos(incl));
}



fn planet_albedo(n: vec3<f32>, base: vec3<f32>, kind: f32) -> vec3<f32> {
    let m = fbm3(n * 3.8 + vec3<f32>(kind * 13.0, 0.0, 0.0));
    let bands = 0.5 + 0.5 * sin(n.y * 16.0 + m * 5.0);
    var c = base * (0.70 + 0.55 * m);
    if (kind < 0.5) {
        let maria = smoothstep(0.42, 0.70, fbm3(n * 7.0 + vec3<f32>(2.0, 5.0, 1.0)));
        c = mix(c, base * 0.52, maria * 0.75);
    } else if (kind < 1.5) {
        let ice = smoothstep(0.58, 0.92, abs(n.y) + m * 0.16);
        c = mix(c, vec3<f32>(0.72, 0.86, 0.96), ice * 0.55);
        let sea = smoothstep(0.35, 0.62, fbm3(n * 5.0 + vec3<f32>(9.0, 1.0, 3.0)));
        c = mix(c, vec3<f32>(0.06, 0.16, 0.30), sea * 0.45);
    } else {
        c = mix(c, base * 1.22, bands * 0.35);
        let storm = smoothstep(0.62, 0.82, fbm3(n * 6.0 + vec3<f32>(4.0, 0.0, 7.0)));
        c = mix(c, vec3<f32>(0.70, 0.51, 0.36), storm * 0.55);
    }
    return c;
}

fn shade_planet(
    hit: vec3<f32>,
    centre: vec3<f32>,
    base: vec3<f32>,
    kind: f32,
    accent: vec3<f32>,
    rd: vec3<f32>,
) -> vec3<f32> {
    let n = normalize(hit - centre);
    let light = normalize(-centre);
    let diff = dot(n, light);
    let day = smoothstep(-0.08, 0.55, diff);
    let rim = pow(1.0 - max(dot(n, -rd), 0.0), 2.6);
    let albedo = planet_albedo(n, base, kind);
    var c = albedo * (0.015 + 0.92 * day);
    c = c + accent * rim * 0.30;
    return c;
}

fn scene_system(ro: vec3<f32>, rd: vec3<f32>, p: vec2<f32>, time: f32, accent: vec3<f32>, colr: vec3<f32>) -> vec3<f32> {
    let motion = u.pointer.w;
    var col = deep_stars(rd, time * 0.4, 21.3, motion) * 0.34;
    let star_t = sphere_t(ro, rd, vec3<f32>(0.0, 0.0, 0.0), 0.50);

    let r0 = 1.45;
    let r1 = 2.30;
    let r2 = 3.30;
    let i0 = 0.08;
    let i1 = 0.20;
    let i2 = 0.34;

    let a0 = 1.7 + time * 0.40 * motion;
    let a1 = 4.3 + time * 0.26 * motion;
    let a2 = 0.4 + time * 0.16 * motion;

    let p0 = orbit_planet_position(r0, a0, i0);
    let p1 = orbit_planet_position(r1, a1, i1);
    let p2 = orbit_planet_position(r2, a2, i2);

    // Orbit lines and their faint dust.
    var line = 0.0;
    var dust = 0.0;
    for (var k = 0; k < 3; k = k + 1) {
        var radius = r0;
        var incl = i0;
        if (k == 1) {
            radius = r1;
            incl = i1;
        } else if (k == 2) {
            radius = r2;
            incl = i2;
        }
        let n = vec3<f32>(0.0, cos(incl), sin(incl));
        let denom = dot(rd, n);
        if (abs(denom) > 0.0004) {
            let tl = -dot(ro, n) / denom;
            if (tl > 0.0) {
                let hp = ro + rd * tl;
                let rr = length(hp);
                let aa = 0.0062;
                let ring = exp(-pow((rr - radius) / aa, 2.0));
                let star_occl = select(1.0, 0.0, star_t > 0.0 && star_t < tl);
                line = line + ring * star_occl;
                dust = dust + exp(-pow((rr - radius) / 0.22, 2.0)) * star_occl;
            }
        }
    }
    col = col + accent * line * 0.105;
    col = col + accent * dust * 0.018;

    // Central star.
    if (star_t > 0.0) {
        let hit = ro + rd * star_t;
        let n = normalize(hit);
        let mu = clamp(dot(n, -rd), 0.0, 1.0);
        let gran = fbm3(n * 21.0 + vec3<f32>(time * 0.02));
        let fine = fbm3(n * 47.0 + vec3<f32>(time * 0.03, 0.0, 0.0));
        var star = mix(vec3<f32>(0.40, 0.040, 0.002), vec3<f32>(1.0, 0.52, 0.10), pow(mu, 0.65));
        star = star * (0.55 + 0.65 * gran + 0.15 * fine);
        col = star * 0.62;
    }
    let sb = origin_distance(ro, rd);
    let glow = exp(-max(sb - 0.50, 0.0) * 8.0) * smoothstep(0.50, 0.56, sb);
    col = col + mix(colr, vec3<f32>(1.0, 0.62, 0.24), 0.65) * glow * 0.42;
    col = col + vec3<f32>(1.0, 0.38, 0.08) * exp(-abs(sb - 0.505) * 220.0) * 0.30;

    // Worlds, front to back.
    let planet_t0 = sphere_t(ro, rd, p0, 0.125);
    let planet_t1 = sphere_t(ro, rd, p1, 0.180);
    let planet_t2 = sphere_t(ro, rd, p2, 0.240);

    if (planet_t2 > 0.0 && !(star_t > 0.0 && star_t < planet_t2)) {
        let hit = ro + rd * planet_t2;
        var pc = shade_planet(hit, p2, vec3<f32>(0.62, 0.55, 0.43), 2.0, accent, rd);
        let ring_n = vec3<f32>(0.0, cos(i2), sin(i2));
        let rd_dot = dot(rd, ring_n);
        if (abs(rd_dot) > 0.0002) {
            let tr = -dot(ro - p2, ring_n) / rd_dot;
            let rp = ro + rd * tr;
            let rr = length(rp - p2);
            if (tr > 0.0 && rr > 0.32 && rr < 0.62 && !(star_t > 0.0 && star_t < tr)) {
                let band = smoothstep(0.32, 0.35, rr) * (1.0 - smoothstep(0.57, 0.62, rr));
                let gaps = 0.45 + 0.55 * abs(sin(rr * 88.0));
                pc = pc + vec3<f32>(0.62, 0.53, 0.40) * band * gaps * 0.68;
            }
        }
        col = pc;
    }
    if (planet_t1 > 0.0 && !(star_t > 0.0 && star_t < planet_t1)) {
        let hit = ro + rd * planet_t1;
        col = shade_planet(hit, p1, vec3<f32>(0.10, 0.28, 0.55), 1.0, accent, rd) * 1.10;
    }
    if (planet_t0 > 0.0 && !(star_t > 0.0 && star_t < planet_t0)) {
        let hit = ro + rd * planet_t0;
        col = shade_planet(hit, p0, vec3<f32>(0.54, 0.23, 0.13), 0.0, accent, rd) * 1.10;
    }

    return col;
}

// ---------------------------------------------------------------------------
// 3 — Nebula: layered emission and dust.

fn scene_nebula(ro: vec3<f32>, rd: vec3<f32>, p: vec2<f32>, time: f32, accent: vec3<f32>, colr: vec3<f32>, phase: f32) -> vec3<f32> {
    let motion = u.pointer.w;
    var col = deep_stars(rd, time, 42.0 + phase, motion) * 0.50;

    var q = p * 1.10;
    q = q + vec2<f32>(sin(time * 0.012), cos(time * 0.009)) * 0.05 * motion;
    q = q + u.pointer.xy * 0.035;

    // A gentle warp keeps the cloud organic without folding it.
    let warp = vec2<f32>(fbm2(q * 0.70 + 2.0), fbm2(q * 0.70 + 11.0));
    let base = fbm2(q * 1.45 + warp * 0.75);
    let ridge = 1.0 - abs(2.0 * fbm2(q * 2.6 + warp * 0.35 + 5.0) - 1.0);
    let fine = fbm2(q * 5.2 - warp * 0.40 + 27.0);
    let body = base * 0.72 + ridge * 0.28;
    let dust_n = fbm2(q * 3.1 + warp * 0.50 + vec2<f32>(5.0, 31.0));

    let centre = vec2<f32>(0.06, 0.02);
    let falloff = exp(-length((q - centre) * vec2<f32>(0.80, 1.05)) * 1.22);
    let density = smoothstep(0.46, 0.74, body + fine * 0.12) * falloff;
    let dust = smoothstep(0.42, 0.66, dust_n) * smoothstep(0.30, 0.70, body);

    let cool = mix(vec3<f32>(0.012, 0.035, 0.16), vec3<f32>(0.10, 0.025, 0.20), smoothstep(0.35, 0.80, fine));
    let warm = mix(vec3<f32>(0.22, 0.035, 0.045), vec3<f32>(0.55, 0.13, 0.055), smoothstep(0.50, 0.90, body));
    var gas = mix(cool, warm, smoothstep(0.58, 0.88, body) * 0.62);
    gas = gas + accent * pow(max(ridge - 0.55, 0.0), 2.0) * 0.55;
    col = col + gas * density * 0.85;

    // Dust absorbs the emission behind it.
    col = col * (1.0 - dust * 0.75);

    // A thin ionisation rim.
    let rim = smoothstep(0.58, 0.68, body) * (1.0 - smoothstep(0.68, 0.80, body));
    col = col + mix(colr, vec3<f32>(0.30, 0.62, 1.0), 0.65) * rim * falloff * 0.10;

    // A small, hot nursery.
    let nursery = p - vec2<f32>(-0.16, -0.02);
    let nursery_glow = exp(-length(nursery) * 18.0);
    col = col + vec3<f32>(1.0, 0.66, 0.34) * nursery_glow * 0.12;
    col = col + vec3<f32>(1.0, 0.92, 0.80) * exp(-length(nursery) * 90.0) * 0.42;

    let sparkle = star_grid(rd, 46.0, 71.0 + phase, time, motion);
    let sparkle_weight = select(0.35, 0.60, u.modes.w > 0.6);
    col = col + sparkle * (0.18 + density * sparkle_weight);

    return col;
}

// ---------------------------------------------------------------------------
// 4 — Galaxy: a differentially rotating spiral seen at an angle.



fn galaxy_sample(sp: vec3<f32>, time: f32, motion: f32) -> vec4<f32> {
    let radius = length(sp.xz);
    if (radius > 6.4) {
        return vec4<f32>(0.0);
    }
    let angle = atan2(sp.z, sp.x);
    let spin = time * 0.045 * motion;
    let phase = (angle - spin) * 2.0 + log(radius + 0.12) * 4.2;
    let arm = 0.5 + 0.5 * sin(phase);
    let spur = 0.5 + 0.5 * sin(phase * 2.0 + 1.7);
    let arm_mask = pow(arm, 3.2) + 0.20 * pow(spur, 6.0);

    let clump = fbm3(vec3<f32>(sp.x * 1.6, sp.y * 5.5, sp.z * 1.6) + vec3<f32>(0.0, 0.0, time * 0.012));
    let feather = fbm3(vec3<f32>(sp.x * 7.0, sp.y * 8.0, sp.z * 7.0) + vec3<f32>(4.0, 0.0, 0.0));
    let dust_n = fbm3(sp * 6.0 + vec3<f32>(11.0));
    let dust = smoothstep(0.46, 0.64, dust_n) * smoothstep(5.6, 2.0, radius) * (0.30 + 0.70 * arm_mask);
    let knot = pow(smoothstep(0.64, 0.96, fbm3(sp * 4.6 + vec3<f32>(21.0, 0.0, 3.0))), 3.0);

    var density = exp(-radius * 0.58) * (0.20 + 0.80 * arm_mask) * (0.30 + 0.70 * clump);
    density = density + exp(-radius * 2.2) * 0.16;
    density = density * (0.70 + 0.30 * feather);
    density = density * smoothstep(6.4, 4.2, radius);
    density = density * (1.0 - dust * 0.72);

    var c = mix(vec3<f32>(0.06, 0.10, 0.30), vec3<f32>(0.42, 0.55, 1.0), arm_mask);
    c = mix(c, vec3<f32>(1.0, 0.60, 0.26), exp(-radius * 1.8) * 0.90);
    c = c + vec3<f32>(0.95, 0.25, 0.40) * knot * arm_mask * 1.40;
    c = c * (0.45 + 0.55 * clump) * (0.70 + 0.30 * feather);

    return vec4<f32>(c * density, density);
}

fn scene_galaxy(ro: vec3<f32>, rd: vec3<f32>, time: f32, accent: vec3<f32>, colr: vec3<f32>) -> vec3<f32> {
    let motion = u.pointer.w;
    var col = deep_stars(rd, time * 0.5, 63.0, motion) * 0.34;

    // A thin, inclined disk: three nearby plane hits give it a little body.
    let denom = rd.y;
    if (abs(denom) > 0.012) {
        for (var i = 0; i < 3; i = i + 1) {
            let off = (f32(i) - 1.0) * 0.10;
            let t = -(ro.y + off) / denom;
            if (t > 0.0) {
                let sp = ro + rd * t;
                let s = galaxy_sample(sp, time, motion);
                let w = select(0.28, 0.44, i == 1);
                col = col + s.rgb * w * 1.65;
            }
        }

        // Resolved star field inside the disk.
        let t0 = -ro.y / denom;
        if (t0 > 0.0) {
            let sp0 = ro + rd * t0;
            let prad = length(sp0.xz);
            if (prad < 5.2) {
                let grid = sp0.xz * 72.0;
                let cell = floor(grid);
                let f = fract(grid);
                let h = hash21(cell);
                let h2 = hash21(cell + vec2<f32>(3.7, 8.1));
                let pos = vec2<f32>(0.22 + 0.56 * h, 0.22 + 0.56 * h2);
                let dd = length(f - pos);
                let keep = select(0.92, 0.88, u.modes.w > 0.6);
                let dot = exp(-dd * dd * 200.0) * step(keep, h) * smoothstep(5.2, 1.0, prad);
                let tint = mix(vec3<f32>(0.70, 0.82, 1.0), vec3<f32>(1.0, 0.80, 0.52), h2);
                col = col + tint * dot * 1.05;
            }
        }
    }

    // Bulge and nucleus.
    let closest = ro + rd * max(-dot(ro, rd), 0.0);
    let dcen = length(closest);
    let bulge = exp(-dcen * 2.4);
    col = col + mix(vec3<f32>(1.0, 0.72, 0.40), colr, 0.30) * bulge * 0.58;
    col = col + vec3<f32>(1.0, 0.86, 0.62) * exp(-dcen * 16.0) * 0.90;
    col = col + accent * exp(-dcen * 1.2) * 0.024;

    return col;
}

// ---------------------------------------------------------------------------
// 5 — Quasar: a point of light, a fast disk and two jets.

fn scene_quasar(ro: vec3<f32>, rd: vec3<f32>, p: vec2<f32>, time: f32, accent: vec3<f32>, colr: vec3<f32>) -> vec3<f32> {
    let motion = u.pointer.w;
    var col = deep_stars(rd, time, 91.0, motion) * 0.46;

    // A faint host galaxy, almost lost in the glare.
    let host = exp(-length(p * vec2<f32>(2.0, 1.1)) * 2.2);
    col = col + vec3<f32>(0.14, 0.14, 0.24) * host * 0.08;

    // Collimated jets along the projected symmetry axis.
    let axis = normalize(vec2<f32>(0.10, 1.0));
    let along = dot(p, axis);
    let across = dot(p, vec2<f32>(-axis.y, axis.x)) + 0.004 * sin(along * 22.0 - time * 0.7);
    let width = 0.0070 + abs(along) * 0.012;
    let knot = 0.55 + 0.45 * sin(along * 34.0 - time * 1.1) * sin(along * 11.0 + 1.7);
    let jet = exp(-across * across / (width * width)) * exp(-abs(along) * 7.5) * knot;
    col = col + mix(accent, vec3<f32>(0.70, 0.88, 1.0), 0.70) * jet * 0.50;

    // The accretion disk: a thin, streaky sheet with no grooves.
    let denom = rd.y;
    if (abs(denom) > 0.0004) {
        let t = -ro.y / denom;
        if (t > 0.0) {
            let hit = ro + rd * t;
            let radius = length(hit.xz);
            if (radius > 0.06 && radius < 2.35) {
                let angle = atan2(hit.z, hit.x);
                let spin = angle - time * 0.55 * motion / (radius + 0.35);
                let streak = fbm3(vec3<f32>(hit.x * 2.3, 0.0, hit.z * 2.3) + vec3<f32>(0.0, 0.0, time * 0.02));
                let fine = fbm3(vec3<f32>(hit.x * 8.5, 0.0, hit.z * 8.5) + vec3<f32>(3.0, 0.0, 1.0));
                let swirl = fbm2(vec2<f32>(spin * 2.0, radius * 0.55));
                let band = smoothstep(0.06, 0.28, radius) * (1.0 - smoothstep(2.00, 2.35, radius));
                let hot = exp(-(radius - 0.26) * 14.0);
                let tang = normalize(vec3<f32>(-hit.z, 0.0, hit.x));
                let mu = dot(tang, -rd);
                let beam = pow(1.0 + 0.55 * mu, 2.1);
                let temp = smoothstep(0.55, 2.30, radius);
                var dcol = mix(vec3<f32>(0.82, 0.90, 1.0), vec3<f32>(1.0, 0.40, 0.08), temp);
                dcol = dcol * (0.10 + 0.68 * streak + 0.38 * fine) * (0.80 + 0.20 * swirl) * band * beam;
                dcol = dcol + vec3<f32>(0.82, 0.90, 1.0) * hot * 0.40;
                col = col + dcol * 0.52;
            }
        }
    }

    // Core glare: a point, a halo, and nothing fashionable.
    let b = origin_distance(ro, rd);
    col = col + colr * exp(-b * 7.0) * 0.62;
    col = col + vec3<f32>(1.0, 0.96, 0.86) * exp(-b * b * 420.0) * 2.40;
    col = col + accent * exp(-b * 2.8) * 0.030;

    return col;
}

// ---------------------------------------------------------------------------
// 6 — Black hole: shadow, photon ring, lensed stars and a bright disk.




fn disk_light(hit: vec3<f32>, rd: vec3<f32>, time: f32, inner: f32, outer: f32) -> vec3<f32> {
    let radius = length(hit.xz);
    if (radius < inner || radius > outer) {
        return vec3<f32>(0.0);
    }
    let angle = atan2(hit.z, hit.x);
    let spin = angle + time * 0.32 / (radius + 0.5);
    let rings = fbm2(vec2<f32>(spin * 2.5, log(radius + 0.1) * 6.5));
    let fine = fbm2(vec2<f32>(spin * 10.0, log(radius + 0.1) * 17.0));
    let streak = fbm3(vec3<f32>(hit.x * 3.0, 0.0, hit.z * 3.0) + vec3<f32>(0.0, 0.0, time * 0.02));
    let edge = smoothstep(inner, inner + 0.16, radius)
        * (1.0 - smoothstep(outer - 0.70, outer, radius));
    let tang = normalize(vec3<f32>(-hit.z, 0.0, hit.x));
    let mu = dot(tang, -rd);
    let beam = pow(1.0 + 0.45 * mu, 2.0);
    let temp = smoothstep(inner, outer, radius);
    var c = mix(vec3<f32>(0.72, 0.82, 1.0), vec3<f32>(1.0, 0.36, 0.08), temp);
    let clump = fbm3(vec3<f32>(hit.x * 4.0, 0.0, hit.z * 4.0));
    let texture = 0.10 + 0.62 * rings + 0.28 * fine + 0.22 * clump;
    c = c * texture * (0.76 + 0.24 * streak) * edge * beam;
    c = c + vec3<f32>(0.66, 0.80, 1.0) * exp(-(radius - inner) * 12.0) * 0.26;
    return c * 0.42;
}

fn scene_black_hole(ro: vec3<f32>, rd: vec3<f32>, p: vec2<f32>, time: f32, accent: vec3<f32>, colr: vec3<f32>) -> vec3<f32> {
    let motion = u.pointer.w;
    let rs = 0.68;
    let shadow = 2.62 * rs;
    let b = max(origin_distance(ro, rd), 0.001);

    // Approximate lensing: bend rays toward the hole, but keep the field readable.
    let closest = ro + rd * max(-dot(ro, rd), 0.0);
    let toward = normalize(-closest + vec3<f32>(0.0001));
    let alpha = clamp(1.02 * rs / max(b, 0.50), 0.0, 0.88);
    let lensed = normalize(rd - toward * alpha);

    // Sharp foreground stars plus a lensed layer for depth.
    var col = deep_stars(rd, time * 0.6, 117.0, motion) * 0.18;
    col = col + deep_stars(lensed, time * 0.6, 117.0, motion) * 0.50;

    let halo_r = exp(-pow((b - shadow) * 2.2, 2.0));
    col = col + accent * halo_r * 0.024;

    // Event horizon.
    let shadow_mask = 1.0 - smoothstep(shadow - 0.018, shadow + 0.006, b);
    col = col * (1.0 - shadow_mask);

    // Photon ring: thin, warm, and quieter than the disk.
    let ring = exp(-pow((b - shadow) * 70.0, 2.0));
    let ring_wide = exp(-pow((b - shadow) * 7.0, 2.0));
    let tilt = 0.84 + 0.16 * sin(atan2(p.y, p.x) + 0.8);
    col = col + mix(vec3<f32>(1.0, 0.86, 0.62), accent, 0.25) * ring * 0.38 * tilt;
    col = col + accent * ring_wide * 0.040;

    // Direct image of the disk.
    let inner = 3.15 * rs;
    let outer = 5.90 * rs;
    let denom = rd.y;
    if (abs(denom) > 0.0005) {
        let t = -ro.y / denom;
        if (t > 0.0) {
            let hit = ro + rd * t;
            let behind = t > max(-dot(ro, rd), 0.0) && b < shadow;
            if (!behind) {
                col = col + disk_light(hit, rd, time, inner, outer);
            }
        }
    }

    // A weakly bent ray gives the far side of the disk its arc over the shadow.
    let lensed_d = normalize(rd - toward * (alpha * 0.14));
    let denom2 = lensed_d.y;
    if (abs(denom2) > 0.0005) {
        let t2 = -ro.y / denom2;
        if (t2 > 0.0) {
            let hit2 = ro + lensed_d * t2;
            var c2 = disk_light(hit2, lensed_d, time, inner, outer);
            let gate = smoothstep(shadow * 0.92, shadow * 1.04, b)
                * (1.0 - smoothstep(shadow * 1.8, shadow * 2.6, b));
            c2 = c2 * 0.78 * gate;
            col = col + c2;
        }
    }

    // A soft glow where the far side of the disk is folded over the shadow.
    let top = smoothstep(0.05, -0.45, p.y);
    let arc = exp(-pow((b - shadow * 1.18) / (shadow * 0.30), 2.0));
    col = col + mix(vec3<f32>(1.0, 0.55, 0.22), accent, 0.45) * arc * top * 0.20;

    // Infalling streams.
    let r = max(length(p), 0.001);
    let ang = atan2(p.y, p.x);
    let spiral = sin(ang * 5.0 + log(r + 0.05) * 7.0 - time * 0.95 * motion);
    let stream = pow(max(spiral, 0.0), 15.0) * (1.0 - smoothstep(2.2, 5.8, r)) * smoothstep(shadow * 0.92, shadow * 1.4, r);
    col = col + mix(accent, vec3<f32>(1.0, 0.68, 0.36), 0.5) * stream * 0.060;

    col = col + vec3<f32>(0.72, 0.40, 0.20) * exp(-abs(b - shadow) * 30.0) * 0.055;

    return col;
}

// ---------------------------------------------------------------------------
// 7 — Epilogue: return to the quiet dark.

fn scene_epilogue(rd: vec3<f32>, p: vec2<f32>, time: f32, accent: vec3<f32>, phase: f32) -> vec3<f32> {
    let motion = u.pointer.w;
    var col = deep_stars(rd, time * 0.45, 131.0 + phase, motion) * 0.9;

    let q = p * 1.35 + u.pointer.xy * 0.04;
    let warp = vec2<f32>(fbm2(q + 3.0), fbm2(q + 13.0));
    let n = fbm2(q * 1.7 + warp * 1.2);
    let dist = length((p - vec2<f32>(0.46, 0.10)) * vec2<f32>(0.75, 1.25));
    let cloud = exp(-dist * 1.8) * (0.15 + 0.85 * n);
    col = col + mix(vec3<f32>(0.09, 0.14, 0.30), vec3<f32>(0.28, 0.36, 0.56), n) * cloud * 0.22;

    // The last warm light, low and distant.
    let horizon = exp(-length((p - vec2<f32>(-0.10, -0.78)) * vec2<f32>(1.0, 2.2)) * 2.1);
    col = col + vec3<f32>(0.62, 0.33, 0.18) * horizon * (0.10 + 0.10 * n);

    // One small, certain star.
    let one = exp(-length((p - vec2<f32>(0.28, -0.12))) * 140.0);
    col = col + vec3<f32>(1.0, 0.93, 0.82) * one * 0.9;
    col = col + accent * exp(-length((p - vec2<f32>(0.28, -0.12))) * 25.0) * 0.05;

    return col;
}

// ---------------------------------------------------------------------------
// Scene dispatch.

fn scene_at(mode: f32, cam: vec4<f32>, palette: vec4<f32>, accent: vec4<f32>, uv: vec2<f32>, phase: f32) -> vec3<f32> {
    let p = centered_uv(uv);
    let time = u.res.w;
    let colr = palette.rgb * palette.w;
    let acc = accent.rgb * accent.w;

    if (mode < 0.5) {
        return scene_prologue(camera_ray(camera_pos(cam.x, cam.y, cam.z), cam.w, p), p, time, acc, phase);
    } else if (mode < 1.5) {
        let ro = camera_pos(cam.x, cam.y, cam.z);
        return scene_star(ro, camera_ray(ro, cam.w, p), p, time, acc, colr, phase);
    } else if (mode < 2.5) {
        let ro = camera_pos(cam.x, cam.y, cam.z);
        return scene_system(ro, camera_ray(ro, cam.w, p), p, time, acc, colr);
    } else if (mode < 3.5) {
        let ro = camera_pos(cam.x, cam.y, cam.z);
        return scene_nebula(ro, camera_ray(ro, cam.w, p), p, time, acc, colr, phase);
    } else if (mode < 4.5) {
        let ro = camera_pos(cam.x, cam.y, cam.z);
        return scene_galaxy(ro, camera_ray(ro, cam.w, p), time, acc, colr);
    } else if (mode < 5.5) {
        let ro = camera_pos(cam.x, cam.y, cam.z);
        return scene_quasar(ro, camera_ray(ro, cam.w, p), p, time, acc, colr);
    } else if (mode < 6.5) {
        let ro = camera_pos(cam.x, cam.y, cam.z);
        return scene_black_hole(ro, camera_ray(ro, cam.w, p), p, time, acc, colr);
    }
    return scene_epilogue(camera_ray(camera_pos(cam.x, cam.y, cam.z), cam.w, p), p, time, acc, phase);
}

@fragment
fn fs_scene(in: VsOut) -> @location(0) vec4<f32> {
    // A very gentle entrance from black, and a slightly deeper exposure while
    // the two scenes are cross-fading.
    let fade = u.flags.z;
    let t = u.modes.z;
    let veil = 1.0 - u.phase.w * sin(t * 3.14159265);

    let a = scene_at(u.modes.x, u.cam_a, u.col_a, u.accent_a, in.uv, u.phase.x);
    var col = a;
    if (t > 0.001) {
        let b = scene_at(u.modes.y, u.cam_b, u.col_b, u.accent_b, in.uv, u.phase.y);
        let k = smoothstep(0.0, 1.0, t);
        col = mix(a, b, k);
    }

    return vec4<f32>(col * fade * veil, 1.0);
}

// ---------------------------------------------------------------------------
// Post-processing.

fn gaussian(tex: texture_2d<f32>, uv: vec2<f32>, step: vec2<f32>) -> vec3<f32> {
    var c = textureSampleLevel(tex, samp, uv, 0.0).rgb * 0.227027;
    c = c + textureSampleLevel(tex, samp, uv + step * 1.384615, 0.0).rgb * 0.316216;
    c = c + textureSampleLevel(tex, samp, uv - step * 1.384615, 0.0).rgb * 0.316216;
    c = c + textureSampleLevel(tex, samp, uv + step * 3.230769, 0.0).rgb * 0.070270;
    c = c + textureSampleLevel(tex, samp, uv - step * 3.230769, 0.0).rgb * 0.070270;
    return c;
}

@fragment
fn fs_down(in: VsOut) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let ts = u.texel.xy * 1.35;
    var c = textureSampleLevel(scene_tex, samp, uv, 0.0).rgb * 0.4;
    c = c + textureSampleLevel(scene_tex, samp, uv + vec2<f32>(ts.x, ts.y), 0.0).rgb * 0.15;
    c = c + textureSampleLevel(scene_tex, samp, uv + vec2<f32>(-ts.x, ts.y), 0.0).rgb * 0.15;
    c = c + textureSampleLevel(scene_tex, samp, uv + vec2<f32>(ts.x, -ts.y), 0.0).rgb * 0.15;
    c = c + textureSampleLevel(scene_tex, samp, uv + vec2<f32>(-ts.x, -ts.y), 0.0).rgb * 0.15;

    let luma = max(c.r, max(c.g, c.b));
    let threshold = u.post.x;
    let knee = 0.35;
    let soft = clamp((luma - threshold + knee) / (2.0 * knee), 0.0, 1.0);
    let contribution = max(soft * soft * (luma - threshold), 0.0) / max(luma, 0.0001);
    return vec4<f32>(c * contribution, 1.0);
}

@fragment
fn fs_blur_h(in: VsOut) -> @location(0) vec4<f32> {
    let step = vec2<f32>(u.texel.z * 1.15, 0.0);
    return vec4<f32>(gaussian(scene_tex, in.uv, step), 1.0);
}

@fragment
fn fs_blur_v(in: VsOut) -> @location(0) vec4<f32> {
    let step = vec2<f32>(0.0, u.texel.w * 1.15);
    return vec4<f32>(gaussian(scene_tex, in.uv, step), 1.0);
}

fn encode_srgb(c: vec3<f32>) -> vec3<f32> {
    let lo = c * 12.92;
    let hi = 1.055 * pow(max(c, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(hi, lo, c <= vec3<f32>(0.0031308));
}

@fragment
fn fs_composite(in: VsOut) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let scene = textureSampleLevel(scene_tex, samp, uv, 0.0).rgb;
    let bloom = textureSampleLevel(bloom_tex, samp, uv, 0.0).rgb;
    var c = scene + bloom * u.post.y;

    // Transition veil keeps the cross-fade from feeling busy.
    c = c * (1.0 - 0.22 * sin(u.modes.z * 3.14159265) * u.phase.w);

    // Exposure and filmic shoulder.
    c = c * 1.0;
    c = aces(c);

    // Vignette, applied after the tone map for a calm edge.
    let d = length((uv - 0.5) * vec2<f32>(u.res.z, 1.0));
    let vig = 1.0 - smoothstep(0.45, 1.12, d) * u.post.w;
    c = c * vig;

    if (u.flags.x > 0.5) {
        c = encode_srgb(c);
    }

    // Grain lives in display space so that the shadows stay smooth.
    let tick = floor(u.res.w * 6.0);
    let g = hash21(uv * u.screen.xy + vec2<f32>(tick * 19.0, tick * 31.0)) - 0.5;
    c = c + g * u.post.z * (0.35 + 0.65 * (1.0 - c));

    return vec4<f32>(clamp(c, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
