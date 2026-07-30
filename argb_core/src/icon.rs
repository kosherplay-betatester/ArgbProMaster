//! The ArgbProMaster identity mark: a thermal flame — cold cyan at its base
//! melting through purple into crimson at the tip — burning on a dark
//! rounded chip with a temperature-gradient ring. Rendered procedurally so
//! every surface (window icon, tray, the .ico embedded in the executables,
//! the installer) draws pixel-identical art at any size, with no image
//! assets or decoders in the dependency tree.

use crate::engine::palette_color;

/// The flame's temperature journey (bottom → tip), straight from the
/// app's signature "Thermal Alert" palette.
fn flame_stops() -> Vec<(f32, [u8; 3])> {
    vec![
        (0.0, [0, 255, 200]),
        (0.42, [180, 0, 255]),
        (0.78, [255, 30, 30]),
        (1.0, [255, 160, 80]),
    ]
}

fn ring_stops() -> Vec<(f32, [u8; 3])> {
    vec![(0.0, [0, 255, 200]), (0.5, [180, 0, 255]), (1.0, [255, 10, 10])]
}

/// Signed distance to a rounded square centered at the origin with half
/// extent `a` and corner radius `r` (negative = inside).
fn rounded_rect(u: f32, v: f32, a: f32, r: f32) -> f32 {
    let qx = (u.abs() - (a - r)).max(0.0);
    let qy = (v.abs() - (a - r)).max(0.0);
    (qx * qx + qy * qy).sqrt() - r
}

/// Distance to the flame silhouette: a rounded base blended into a concave
/// taper whose tip curves gracefully to the right — the classic licking
/// flame. `scale` < 1 renders the inner core.
fn flame_sdf(u: f32, v: f32, scale: f32) -> (f32, f32) {
    let u = u / scale;
    let v = v / scale;
    let base_c = (0.0f32, -0.30f32);
    let base_r = 0.30f32;
    let tip_v = 0.76f32;
    let junc_v = base_c.1 + 0.04;

    let dc = ((u - base_c.0).powi(2) + (v - base_c.1).powi(2)).sqrt() - base_r;
    let mut d = dc;
    if v >= junc_v && v <= tip_v {
        let t = ((v - junc_v) / (tip_v - junc_v)).clamp(0.0, 1.0);
        // The tip leans right with a slight S — a flame, not a droplet.
        let sway = 0.17 * t.powf(2.0) - 0.04 * (std::f32::consts::PI * t).sin();
        // Concave sides: fast taper with a low bulge near the base.
        let hw = base_r * (1.0 - t).powf(1.30) * (1.0 + 0.65 * t * (1.0 - t));
        d = d.min(((u - sway).abs() - hw) * 0.85);
    }
    // Height along the flame, 0 at the bottom of the base, 1 at the tip.
    let h = ((v - (base_c.1 - base_r)) / (tip_v - (base_c.1 - base_r))).clamp(0.0, 1.0);
    (d * scale, h)
}

/// Non-premultiplied source-over composite.
fn over(dst: &mut [f32; 4], rgb: [f32; 3], alpha: f32) {
    let sa = alpha.clamp(0.0, 1.0);
    if sa <= 0.0 {
        return;
    }
    let da = dst[3];
    let out_a = sa + da * (1.0 - sa);
    if out_a <= 0.0 {
        return;
    }
    for c in 0..3 {
        dst[c] = (rgb[c] * sa + dst[c] * da * (1.0 - sa)) / out_a;
    }
    dst[3] = out_a;
}

/// Render the app icon as straight (non-premultiplied) RGBA8.
pub fn render(size: u32) -> Vec<u8> {
    let flame = flame_stops();
    let ring = ring_stops();
    let n = size as f32;
    let half = (n - 1.0) / 2.0;
    let px = 2.0 / n; // one pixel in normalized units
    let aa = |d: f32| (0.5 - d / (1.6 * px)).clamp(0.0, 1.0);

    let mut out = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let u = (x as f32 - half) / half;
            let v = (half - y as f32) / half; // v grows upward

            let mut p = [0.0f32; 4]; // transparent

            // Dark chip with a subtle vertical sheen.
            let d_chip = rounded_rect(u, v, 0.96, 0.30);
            let chip_a = aa(d_chip);
            if chip_a > 0.0 {
                let sheen = 0.5 + 0.5 * v;
                let bg = [
                    10.0 + 13.0 * sheen,
                    12.0 + 15.0 * sheen,
                    24.0 + 27.0 * sheen,
                ];
                over(&mut p, bg, chip_a);

                // Ambient glow the flame casts on the chip.
                let g = (-((u * u + (v + 0.05) * (v + 0.05)) * 2.4)).exp();
                let glow_c = palette_color(&flame, (0.5 + 0.5 * v).clamp(0.0, 1.0));
                over(&mut p, glow_c, 0.22 * g * chip_a);
            }

            // Temperature ring hugging the chip edge.
            let d_ring = d_chip.abs() - 0.045;
            let ring_a = aa(d_ring) * chip_a.max(aa(d_chip + 0.05));
            if ring_a > 0.0 {
                let t = ((v + 1.0) / 2.0).clamp(0.0, 1.0);
                over(&mut p, palette_color(&ring, t), 0.92 * ring_a);
            }

            // The flame body — rim-shaded so it reads sculpted, not flat —
            // then its white-hot core, sitting high like a real flame's.
            let (d_f, h) = flame_sdf(u, v - 0.06, 1.0);
            let fa = aa(d_f);
            if fa > 0.0 {
                let c = palette_color(&flame, h);
                let depth = 0.82 + 0.18 * (-d_f / 0.16).clamp(0.0, 1.0);
                over(&mut p, [c[0] * depth, c[1] * depth, c[2] * depth], fa);
            }
            let (d_c, hc) = flame_sdf(u, v - 0.13, 0.46);
            let ca = aa(d_c);
            if ca > 0.0 {
                let base = palette_color(&flame, hc);
                let core = [
                    base[0] + (255.0 - base[0]) * 0.82,
                    base[1] + (255.0 - base[1]) * 0.82,
                    base[2] + (255.0 - base[2]) * 0.82,
                ];
                over(&mut p, core, ca * 0.95);
            }

            // A tiny ARGB strip glowing under the flame — adaptive detail
            // that only exists at sizes where it stays crisp.
            if size >= 48 {
                for (i, lx) in [-0.52f32, -0.26, 0.0, 0.26, 0.52].iter().enumerate() {
                    let dd = ((u - lx).powi(2) + (v + 0.74).powi(2)).sqrt() - 0.055;
                    let la = aa(dd) * chip_a;
                    if la > 0.0 {
                        let c = palette_color(&flame, i as f32 / 4.0);
                        over(&mut p, c, la);
                    }
                    // soft glow under each LED
                    let g = (-(((u - lx).powi(2) + (v + 0.74).powi(2)) * 90.0)).exp();
                    if g > 0.01 {
                        let c = palette_color(&flame, i as f32 / 4.0);
                        over(&mut p, c, 0.35 * g * chip_a);
                    }
                }
            }

            out.push((p[0].round().clamp(0.0, 255.0)) as u8);
            out.push((p[1].round().clamp(0.0, 255.0)) as u8);
            out.push((p[2].round().clamp(0.0, 255.0)) as u8);
            out.push((p[3] * 255.0).round().clamp(0.0, 255.0) as u8);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_expected_sizes_with_transparent_corners() {
        for size in [16u32, 32, 64, 256] {
            let rgba = render(size);
            assert_eq!(rgba.len(), (size * size * 4) as usize);
            // The extreme corner sits outside the rounded chip.
            assert_eq!(rgba[3], 0, "corner must be transparent at {size}px");
            // The flame center is opaque and vividly colored.
            let cx = ((size / 2) * size + size / 2) as usize * 4;
            assert_eq!(rgba[cx + 3], 255, "center must be opaque at {size}px");
        }
    }

    #[test]
    fn deterministic() {
        assert_eq!(render(48), render(48));
    }
}
