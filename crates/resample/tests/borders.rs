use resample::{wrap, Border};

fn seq(rule: Border, n: usize, range: std::ops::RangeInclusive<i64>) -> Vec<Option<usize>> {
    range.map(|i| wrap(i, n, rule)).collect()
}

#[test]
fn inside_indices_are_unchanged_for_every_rule() {
    for rule in [
        Border::Zero,
        Border::Clamp,
        Border::Repeat,
        Border::Mirror,
        Border::MirrorNoEdge,
    ] {
        for i in 0..3 {
            assert_eq!(wrap(i, 3, rule), Some(i as usize));
        }
    }
}

#[test]
fn zero_rule_three_pixel_row() {
    let want = [
        None,
        None,
        None,
        Some(0),
        Some(1),
        Some(2),
        None,
        None,
        None,
    ];
    assert_eq!(seq(Border::Zero, 3, -3..=5), want);
}

#[test]
fn clamp_rule_three_pixel_row() {
    let want: Vec<_> = [0, 0, 0, 0, 1, 2, 2, 2, 2].into_iter().map(Some).collect();
    assert_eq!(seq(Border::Clamp, 3, -3..=5), want);
}

#[test]
fn repeat_rule_three_pixel_row() {
    let want: Vec<_> = [0, 1, 2, 0, 1, 2, 0, 1, 2].into_iter().map(Some).collect();
    assert_eq!(seq(Border::Repeat, 3, -3..=5), want);
}

#[test]
fn mirror_rule_three_pixel_row() {
    let want: Vec<_> = [2, 1, 0, 0, 1, 2, 2, 1, 0].into_iter().map(Some).collect();
    assert_eq!(seq(Border::Mirror, 3, -3..=5), want);
}

#[test]
fn mirror_no_edge_rule_three_pixel_row() {
    let want: Vec<_> = [1, 2, 1, 0, 1, 2, 1, 0, 1].into_iter().map(Some).collect();
    assert_eq!(seq(Border::MirrorNoEdge, 3, -3..=5), want);
}

#[test]
fn far_indices() {
    assert_eq!(wrap(-100, 3, Border::Repeat), Some(2));
    assert_eq!(wrap(100, 3, Border::Repeat), Some(1));
    assert_eq!(wrap(-100, 3, Border::Mirror), Some(2));
    assert_eq!(wrap(100, 3, Border::Mirror), Some(1));
    assert_eq!(wrap(-100, 3, Border::MirrorNoEdge), Some(0));
    assert_eq!(wrap(100, 3, Border::MirrorNoEdge), Some(0));
    assert_eq!(wrap(101, 3, Border::MirrorNoEdge), Some(1));
    assert_eq!(wrap(102, 3, Border::MirrorNoEdge), Some(2));
    assert_eq!(wrap(103, 3, Border::MirrorNoEdge), Some(1));
    assert_eq!(wrap(-100, 3, Border::Clamp), Some(0));
    assert_eq!(wrap(100, 3, Border::Clamp), Some(2));
    assert_eq!(wrap(i64::MAX / 2, 3, Border::Zero), None);
}

#[test]
fn one_sample_axis() {
    for i in [-5, -1, 1, 4] {
        assert_eq!(wrap(i, 1, Border::Zero), None);
        for rule in [
            Border::Clamp,
            Border::Repeat,
            Border::Mirror,
            Border::MirrorNoEdge,
        ] {
            assert_eq!(wrap(i, 1, rule), Some(0), "{rule:?} {i}");
        }
    }
}

#[test]
fn two_sample_axis() {
    let want: Vec<_> = [1, 0, 0, 1, 1, 0].into_iter().map(Some).collect();
    assert_eq!(seq(Border::Mirror, 2, -2..=3), want);
    let want: Vec<_> = [0, 1, 0, 1, 0, 1].into_iter().map(Some).collect();
    assert_eq!(seq(Border::MirrorNoEdge, 2, -2..=3), want);
}
