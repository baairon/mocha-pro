//! Coarse search by normalised cross correlation over integer translations.
//!
//! Lewis, "Fast Normalized Cross-Correlation", Vision Interface 1995 (the
//! normalised correlation coefficient of a template and an image window); Brunelli, Template
//! Matching Techniques in Computer Vision, 2009, chapter 3. The score of a shift `(dx, dy)` is
//! the weighted correlation coefficient between the template values of the selected pixels and
//! the frame values at the shifted warped positions:
//! `sum w (T - mT) (I - mI) / sqrt(sum w (T - mT)^2 * sum w (I - mI)^2)`. Shifts are scanned row
//! by row (`dy` outer, `dx` inner, increasing); the first best score wins ties.

use crate::align::{FrameLevel, LevelTemplate};
use crate::config::NccConfig;
use crate::model::Pose;

/// Result of a search.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NccResult {
    /// The pose with the best shift applied after the starting pose.
    pub pose: Pose,
    /// The best score.
    pub score: f64,
    /// The shift in pixels of the search level.
    pub shift: (f64, f64),
}

/// Score of one pose, `None` when too few pixels are inside or a variance is zero.
pub fn score(tpl: &LevelTemplate, fr: &FrameLevel, pose: &Pose, min_valid: f64) -> Option<f64> {
    let h = &pose.h;
    let mut tv = Vec::new();
    let mut iv = Vec::new();
    let mut wv = Vec::new();
    for i in 0..tpl.len() {
        let (x, y) = (tpl.x[i], tpl.y[i]);
        let zz = h[6] * x + h[7] * y + h[8];
        if zz.is_nan() || zz <= 1e-9 {
            continue;
        }
        let u = (h[0] * x + h[1] * y + h[2]) / zz;
        let v = (h[3] * x + h[4] * y + h[5]) / zz;
        if let Some(val) = fr.img.sample_inside(u, v) {
            tv.push(tpl.t[i]);
            iv.push(val);
            wv.push(tpl.w[i]);
        }
    }
    if (tv.len() as f64) < min_valid * tpl.len() as f64 || tv.is_empty() {
        return None;
    }
    let (mut sw, mut st, mut si) = (0.0, 0.0, 0.0);
    for k in 0..tv.len() {
        sw += wv[k];
        st += wv[k] * tv[k];
        si += wv[k] * iv[k];
    }
    let (mt, mi) = (st / sw, si / sw);
    let (mut stt, mut sii, mut sti) = (0.0, 0.0, 0.0);
    for k in 0..tv.len() {
        let a = tv[k] - mt;
        let b = iv[k] - mi;
        stt += wv[k] * a * a;
        sii += wv[k] * b * b;
        sti += wv[k] * a * b;
    }
    // A window or template whose weighted variance is at rounding level has no defined score.
    if !(stt > 1e-18 * sw && sii > 1e-18 * sw) {
        return None;
    }
    let den = (stt * sii).sqrt();
    if den.is_nan() || den <= 0.0 {
        return None;
    }
    Some(sti / den)
}

/// Scan the window around `start` (a pose in level coordinates). `None` when no shift scores or
/// the best score is below `cfg.min_score`.
pub fn search(
    tpl: &LevelTemplate,
    fr: &FrameLevel,
    start: &Pose,
    cfg: &NccConfig,
) -> Option<NccResult> {
    let r = cfg.radius as i64;
    let step = cfg.step as i64;
    let mut best: Option<NccResult> = None;
    let mut dy = -r;
    while dy <= r {
        let mut dx = -r;
        while dx <= r {
            let shift = Pose::translation(dx as f64, dy as f64);
            let pose = shift.compose(start);
            if let Some(s) = score(tpl, fr, &pose, cfg.min_valid_fraction) {
                if best.is_none_or(|b| s > b.score) {
                    best = Some(NccResult {
                        pose,
                        score: s,
                        shift: (dx as f64, dy as f64),
                    });
                }
            }
            dx += step;
        }
        dy += step;
    }
    best.filter(|b| b.score >= cfg.min_score)
}
