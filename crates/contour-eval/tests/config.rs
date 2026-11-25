//! The configuration space.

use contour_eval::{
    eval, Config, Contour, EvalForm, FlatTest, Kind, OpenEnd, Point, Precision, TangentMode,
    WeightMap, XVariant,
};

#[test]
fn canonical_values_are_the_documented_ones() {
    let c = Config::canonical();
    assert_eq!(c.form, EvalForm::Bernstein);
    assert_eq!(c.precision, Precision::F64);
    assert_eq!(c.tangents, TangentMode::Relative);
    assert_eq!(c.weight_map, WeightMap { a: 1.0, b: 0.0 });
    assert_eq!(c.open_end, OpenEnd::RepeatEnd);
    assert!(c.closing_span);
    assert!(!c.repeat_first);
    assert_eq!(c.flat_test, FlatTest::Sampled);
}

#[test]
fn enumerate_has_768_configs() {
    assert_eq!(Config::enumerate().len(), 768);
}

#[test]
fn enumerate_has_no_duplicates() {
    let all = Config::enumerate();
    for i in 0..all.len() {
        for j in (i + 1)..all.len() {
            assert_ne!(all[i], all[j], "{i} {j}");
        }
    }
}

#[test]
fn enumerate_order_is_fixed_and_contains_canonical() {
    let a = Config::enumerate();
    let b = Config::enumerate();
    assert_eq!(a, b);
    assert!(a.contains(&Config::canonical()));
    assert_eq!(a[0].form, EvalForm::Bernstein);
}

#[test]
fn enumerate_covers_each_choice() {
    let all = Config::enumerate();
    assert!(all.iter().any(|c| c.form == EvalForm::Horner));
    assert!(all.iter().any(|c| c.form == EvalForm::DeCasteljau));
    assert!(all.iter().any(|c| c.precision == Precision::F32));
    assert!(all.iter().any(|c| c.tangents == TangentMode::Absolute));
    assert!(all.iter().any(|c| c.open_end == OpenEnd::RepeatEndSharp));
    assert!(all.iter().any(|c| !c.closing_span));
    assert!(all.iter().any(|c| c.repeat_first));
    assert!(all.iter().any(|c| c.flat_test == FlatTest::ControlPolygon));
    for m in Config::WEIGHT_MAPS {
        assert!(all.iter().any(|c| c.weight_map == m));
    }
}

#[test]
fn every_config_evaluates_a_valid_contour() {
    let pts: Vec<Point> = (0..4)
        .map(|i| Point {
            tan_in: (-1.0, 0.0),
            tan_out: (1.0, 0.0),
            s: 0.5,
            ..Point::new(f64::from(i) * 4.0, f64::from(i % 2) * 4.0)
        })
        .collect();
    for kind in [
        Kind::Bezier,
        Kind::XSpline {
            variant: XVariant::General,
        },
    ] {
        for closed in [false, true] {
            let c = Contour {
                kind,
                closed,
                points: pts.clone(),
            };
            for g in Config::enumerate() {
                assert!(eval(&c, 0, 0.5, &g).is_ok(), "{g:?}");
            }
        }
    }
}

#[test]
fn weight_map_list_is_affine_and_generic() {
    assert_eq!(Config::WEIGHT_MAPS[0], WeightMap::IDENTITY);
    for m in Config::WEIGHT_MAPS {
        assert!(m.a != 0.0);
    }
}

#[test]
fn config_choices_change_results_where_they_should() {
    let c = Contour {
        kind: Kind::Bezier,
        closed: false,
        points: vec![
            Point {
                tan_out: (3.0, 10.0),
                ..Point::new(1.0, 2.0)
            },
            Point {
                tan_in: (9.0, 12.0),
                ..Point::new(13.0, 4.0)
            },
        ],
    };
    let rel = Config::canonical();
    let mut abs = rel;
    abs.tangents = TangentMode::Absolute;
    assert_ne!(
        eval(&c, 0, 0.5, &rel).unwrap(),
        eval(&c, 0, 0.5, &abs).unwrap()
    );
}
