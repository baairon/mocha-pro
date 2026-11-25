use soft_edge::*;

macro_rules! idx {
    ($name:ident, $i:expr, $n:expr, $b:expr, $want:expr) => {
        #[test]
        fn $name() {
            assert_eq!(resolve_index($i, $n, $b), $want);
        }
    };
}

idx!(zero_inside, 2, 5, Border::Zero, Some(2));
idx!(zero_left, -1, 5, Border::Zero, None);
idx!(zero_right, 5, 5, Border::Zero, None);
idx!(clamp_left, -3, 5, Border::Clamp, Some(0));
idx!(clamp_right, 9, 5, Border::Clamp, Some(4));
idx!(mirror_m1, -1, 5, Border::Mirror, Some(0));
idx!(mirror_m2, -2, 5, Border::Mirror, Some(1));
idx!(mirror_m5, -5, 5, Border::Mirror, Some(4));
idx!(mirror_m6, -6, 5, Border::Mirror, Some(4));
idx!(mirror_r5, 5, 5, Border::Mirror, Some(4));
idx!(mirror_r6, 6, 5, Border::Mirror, Some(3));
idx!(mirror_r10, 10, 5, Border::Mirror, Some(0));
idx!(mirror_one, -7, 1, Border::Mirror, Some(0));
idx!(norep_m1, -1, 5, Border::MirrorNoRepeat, Some(1));
idx!(norep_m4, -4, 5, Border::MirrorNoRepeat, Some(4));
idx!(norep_m5, -5, 5, Border::MirrorNoRepeat, Some(3));
idx!(norep_r5, 5, 5, Border::MirrorNoRepeat, Some(3));
idx!(norep_r8, 8, 5, Border::MirrorNoRepeat, Some(0));
idx!(norep_one, 3, 1, Border::MirrorNoRepeat, Some(0));
idx!(norep_two, -1, 2, Border::MirrorNoRepeat, Some(1));

#[test]
fn default_config_values() {
    let c = Config::default();
    assert_eq!(c.border, Border::Clamp);
    assert_eq!(c.frac_rule, FracRule::EndTaps);
    assert_eq!(c.trunc_k, 3.0);
    assert_eq!(c.exp_terms, 12);
    assert_eq!(c.radius_to_sigma, 2.0);
    assert_eq!(c.threshold, 0.5);
    assert_eq!(c.edge_offset, 0.5);
    assert_eq!(c.profile, Profile::Linear);
    assert_eq!(c.direction, Direction::Centred);
    assert_eq!(c.exp_method, ExpMethod::Series);
}

#[test]
fn enumerate_size_and_unique() {
    let all = Config::enumerate();
    assert_eq!(all.len(), 4 * 2 * 2 * 3 * 3 * 2);
    let mut keys: Vec<String> = all.iter().map(|c| format!("{c:?}")).collect();
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), all.len());
}

#[test]
fn enumerate_is_deterministic() {
    assert_eq!(Config::enumerate(), Config::enumerate());
}

#[test]
fn enumerate_covers_each_choice() {
    let all = Config::enumerate();
    for b in Border::ALL {
        assert!(all.iter().any(|c| c.border == b));
    }
    for d in [Direction::Inward, Direction::Outward, Direction::Centred] {
        assert!(all.iter().any(|c| c.direction == d));
    }
    for p in [Profile::Linear, Profile::Smoothstep, Profile::Cosine] {
        assert!(all.iter().any(|c| c.profile == p));
    }
    assert!(all.iter().any(|c| c.exp_method == ExpMethod::Recurrence));
    assert!(all.iter().any(|c| c.frac_rule == FracRule::Lerp));
    assert!(all.iter().any(|c| c.edge_offset == 0.0));
}

#[test]
fn enumerate_keeps_other_defaults() {
    let d = Config::default();
    for c in Config::enumerate() {
        assert_eq!(
            (c.trunc_k, c.exp_terms, c.threshold),
            (d.trunc_k, d.exp_terms, d.threshold)
        );
        assert_eq!(c.radius_to_sigma, d.radius_to_sigma);
    }
}

#[test]
fn error_display() {
    assert_eq!(
        Error::SizeMismatch.to_string(),
        "buffer length is not width * height"
    );
    assert_eq!(Error::BadParameter("x").to_string(), "bad parameter: x");
}

#[test]
fn enumerate_edge_offsets_are_zero_and_half() {
    let all = Config::enumerate();
    assert!(all
        .iter()
        .all(|c| c.edge_offset == 0.0 || c.edge_offset == 0.5));
    assert!(all.iter().any(|c| c.edge_offset == 0.5));
}
