//! The resampler helper and the border bounds.
use lens_model::{Bounds, Config, Direction, InverseMethod, Model, ModelMaps, Normalisation};

fn k1(v: f64) -> Model {
    Model::Radial {
        k: [v, 0.0, 0.0],
        p: [0.0; 2],
    }
}

fn square(model: Model) -> Config {
    Config::new(8.0, 8.0, model)
}

#[test]
fn identity_bounds_are_the_rectangle() {
    let c = Config::new(6.0, 8.0, Model::identity());
    for d in [Direction::ToDistorted, Direction::ToUndistorted] {
        let b = c.mapped_bounds(d, 4).unwrap();
        assert_eq!(
            b,
            Bounds {
                min: [0.0, 0.0],
                max: [6.0, 8.0]
            }
        );
    }
}

#[test]
fn barrel_bounds_by_hand() {
    // scale 4, centre 4; corner n = (-1,-1): r2 = 2, g = 2, -> (-2,-2) -> pixel -4; far corner 12
    let b = square(k1(0.5))
        .mapped_bounds(Direction::ToDistorted, 2)
        .unwrap();
    assert_eq!(
        b,
        Bounds {
            min: [-4.0, -4.0],
            max: [12.0, 12.0]
        }
    );
}

#[test]
fn bounds_one_step_uses_corners_only() {
    let b = square(k1(0.5))
        .mapped_bounds(Direction::ToDistorted, 1)
        .unwrap();
    assert_eq!(b.min, [-4.0, -4.0]);
    assert_eq!(b.max, [12.0, 12.0]);
}

#[test]
fn bounds_zero_steps_is_none() {
    assert_eq!(
        square(k1(0.5)).mapped_bounds(Direction::ToDistorted, 0),
        None
    );
}

#[test]
fn pincushion_shrinks_the_bounds() {
    // k1 = -0.25: corner g = 0.5 -> pixel 2; edge middle n = (-1,0): g = 0.75 -> -0.75 -> pixel 1
    let b = square(k1(-0.25))
        .mapped_bounds(Direction::ToDistorted, 2)
        .unwrap();
    assert_eq!(
        b,
        Bounds {
            min: [1.0, 1.0],
            max: [7.0, 7.0]
        }
    );
}

#[test]
fn bounds_use_the_edge_samples_not_only_corners() {
    // A model whose edge midpoint goes further than the corners: k1 positive, k2 negative large.
    let m = Model::Radial {
        k: [0.0, 0.0, 0.0],
        p: [0.0, 0.0],
    };
    let c = square(m);
    let b1 = c.mapped_bounds(Direction::ToDistorted, 1).unwrap();
    let b4 = c.mapped_bounds(Direction::ToDistorted, 4).unwrap();
    assert_eq!(b1, b4);
    let m = Model::Radial {
        k: [1.0, -0.75, 0.0],
        p: [0.0, 0.0],
    };
    let c = square(m);
    // along the top edge n = (t, -1): the x extent peaks inside the edge, so more steps widen it
    let a = c.mapped_bounds(Direction::ToDistorted, 1).unwrap();
    let b = c.mapped_bounds(Direction::ToDistorted, 8).unwrap();
    assert!(b.max[0] >= a.max[0] && b.min[1] <= a.min[1]);
    assert!(b.max != a.max || b.min != a.min);
}

#[test]
fn bounds_fail_when_a_sample_fails() {
    let mut c = Config::new(2.0, 2.0, Model::Division { lambda: 1.0 });
    c.normalisation = Normalisation::Focal(1.0);
    c.centre = [0.0, 0.0];
    c.maps = ModelMaps::DistortedToUndistorted;
    c.inverse_method = InverseMethod::ClosedForm;
    // ToDistorted calls to_distorted = closed form inverse; corner (2,2) is outside the domain
    assert_eq!(c.mapped_bounds(Direction::ToDistorted, 2), None);
}

#[test]
fn bounds_remove_direction_uses_to_undistorted() {
    let mut c = square(k1(0.5));
    c.maps = ModelMaps::DistortedToUndistorted;
    let b = c.mapped_bounds(Direction::ToUndistorted, 2).unwrap();
    assert_eq!(
        b,
        Bounds {
            min: [-4.0, -4.0],
            max: [12.0, 12.0]
        }
    );
}

#[test]
fn bounds_cover_every_sampled_point() {
    let c = square(Model::Radial {
        k: [0.1, 0.02, 0.0],
        p: [0.01, -0.02],
    });
    let b = c.mapped_bounds(Direction::ToDistorted, 16).unwrap();
    for i in 0..=16 {
        let t = 8.0 * f64::from(i) / 16.0;
        for p in [[t, 0.0], [t, 8.0], [0.0, t], [8.0, t]] {
            let m = c.to_distorted(p).unwrap();
            assert!(m[0] >= b.min[0] && m[0] <= b.max[0] && m[1] >= b.min[1] && m[1] <= b.max[1]);
        }
    }
}

#[test]
fn source_map_apply_reads_undistorted_source() {
    let c = Config::new(6.0, 8.0, k1(0.5));
    let f = c.source_map(Direction::ToDistorted);
    // inverse of 4.6875 (n 0.5625) is n 0.5 -> pixel 4.5 within iteration accuracy
    let s = f([4.6875, 4.0]).unwrap();
    assert!((s[0] - 4.5).abs() < 1e-9 && s[1] == 4.0);
}

#[test]
fn source_map_remove_reads_distorted_source() {
    let c = Config::new(6.0, 8.0, k1(0.5));
    let f = c.source_map(Direction::ToUndistorted);
    assert_eq!(f([4.5, 4.0]), Some([4.6875, 4.0]));
}

#[test]
fn source_map_none_outside_domain() {
    let mut c = Config::new(2.0, 2.0, Model::Division { lambda: 1.0 });
    c.normalisation = Normalisation::Focal(1.0);
    c.centre = [0.0, 0.0];
    c.inverse_method = InverseMethod::ClosedForm;
    let f = c.source_map(Direction::ToDistorted);
    assert_eq!(f([1.0, 0.0]), None);
    assert_eq!(f([0.5, 0.0]), Some([1.0, 0.0]));
}

#[test]
fn source_maps_of_two_directions_are_mutual_inverses() {
    let mut c = Config::new(
        16.0,
        16.0,
        Model::Radial {
            k: [0.0625, 0.0, 0.0],
            p: [0.0; 2],
        },
    );
    c.inverse_iterations = 12;
    let apply = c.source_map(Direction::ToDistorted);
    let remove = c.source_map(Direction::ToUndistorted);
    for p in [[3.0, 5.0], [12.0, 2.0], [8.0, 8.0]] {
        let d = remove(p).unwrap();
        let u = apply(d).unwrap();
        assert!((u[0] - p[0]).abs() < 1e-12 && (u[1] - p[1]).abs() < 1e-12);
    }
}

#[test]
fn target_map_matches_config_maps() {
    let c = Config::new(6.0, 8.0, k1(0.5));
    assert_eq!(
        c.target_map(Direction::ToDistorted, [4.5, 4.0]),
        c.to_distorted([4.5, 4.0])
    );
    assert_eq!(
        c.target_map(Direction::ToUndistorted, [4.5, 4.0]),
        c.to_undistorted([4.5, 4.0])
    );
}

#[test]
fn bounds_are_deterministic() {
    let c = square(Model::Radial {
        k: [0.1, 0.02, 0.0],
        p: [0.01, -0.02],
    });
    let a = c.mapped_bounds(Direction::ToUndistorted, 9).unwrap();
    let b = c.mapped_bounds(Direction::ToUndistorted, 9).unwrap();
    assert_eq!(a.min.map(f64::to_bits), b.min.map(f64::to_bits));
    assert_eq!(a.max.map(f64::to_bits), b.max.map(f64::to_bits));
}

#[test]
fn squeezed_bounds_are_wider_on_the_squeezed_axis() {
    let m = Model::Squeezed {
        k: [0.125, 0.0, 0.0],
        squeeze: [1.0, 2.0],
    };
    let b = Config::new(8.0, 8.0, m)
        .mapped_bounds(Direction::ToDistorted, 4)
        .unwrap();
    // corner n = (-1,-1): r2 = 1 + 4 = 5, g = 1.625 -> -1.625 -> pixel 4 - 6.5 = -2.5
    assert_eq!(b.min, [-2.5, -2.5]);
    assert_eq!(b.max, [10.5, 10.5]);
}

#[test]
fn identity_bounds_of_a_wide_rectangle() {
    let c = Config::new(8.0, 4.0, Model::identity());
    let b = c.mapped_bounds(Direction::ToDistorted, 4).unwrap();
    assert_eq!(
        b,
        Bounds {
            min: [0.0, 0.0],
            max: [8.0, 4.0]
        }
    );
}
