//! Division guards of colour dodge and colour burn, and the order they are tested in.
use blend_ops::{blend_channel, BlendMode as M, Config, Guard, GuardOrder};

fn cfg() -> Config {
    Config::default()
}

#[test]
fn dodge_default_zero_backdrop() {
    assert_eq!(blend_channel(M::ColorDodge, 0.0, 0.5, &cfg()), 0.0);
}
#[test]
fn dodge_default_one_source() {
    assert_eq!(blend_channel(M::ColorDodge, 0.5, 1.0, &cfg()), 1.0);
}
#[test]
fn dodge_both_default_backdrop_first_gives_zero() {
    assert_eq!(blend_channel(M::ColorDodge, 0.0, 1.0, &cfg()), 0.0);
}
#[test]
fn dodge_both_source_first_gives_one() {
    let c = Config {
        guard_order: GuardOrder::SourceFirst,
        ..cfg()
    };
    assert_eq!(blend_channel(M::ColorDodge, 0.0, 1.0, &c), 1.0);
}
#[test]
fn dodge_backdrop_zero_guard_one() {
    let c = Config {
        dodge_backdrop_zero: Guard::One,
        ..cfg()
    };
    assert_eq!(blend_channel(M::ColorDodge, 0.0, 0.5, &c), 1.0);
}
#[test]
fn dodge_source_one_guard_zero() {
    let c = Config {
        dodge_source_one: Guard::Zero,
        ..cfg()
    };
    assert_eq!(blend_channel(M::ColorDodge, 0.5, 1.0, &c), 0.0);
}
#[test]
fn dodge_clamps_quotient_to_one() {
    assert_eq!(blend_channel(M::ColorDodge, 0.75, 0.5, &cfg()), 1.0);
}
#[test]
fn dodge_quotient_value() {
    assert_eq!(blend_channel(M::ColorDodge, 0.125, 0.75, &cfg()), 0.5);
}
#[test]
fn burn_default_one_backdrop() {
    assert_eq!(blend_channel(M::ColorBurn, 1.0, 0.5, &cfg()), 1.0);
}
#[test]
fn burn_default_zero_source() {
    assert_eq!(blend_channel(M::ColorBurn, 0.5, 0.0, &cfg()), 0.0);
}
#[test]
fn burn_both_default_backdrop_first_gives_one() {
    assert_eq!(blend_channel(M::ColorBurn, 1.0, 0.0, &cfg()), 1.0);
}
#[test]
fn burn_both_source_first_gives_zero() {
    let c = Config {
        guard_order: GuardOrder::SourceFirst,
        ..cfg()
    };
    assert_eq!(blend_channel(M::ColorBurn, 1.0, 0.0, &c), 0.0);
}
#[test]
fn burn_backdrop_one_guard_zero() {
    let c = Config {
        burn_backdrop_one: Guard::Zero,
        ..cfg()
    };
    assert_eq!(blend_channel(M::ColorBurn, 1.0, 0.5, &c), 0.0);
}
#[test]
fn burn_source_zero_guard_one() {
    let c = Config {
        burn_source_zero: Guard::One,
        ..cfg()
    };
    assert_eq!(blend_channel(M::ColorBurn, 0.5, 0.0, &c), 1.0);
}
#[test]
fn burn_clamps_quotient_to_one() {
    assert_eq!(
        blend_channel(M::ColorBurn, 0.25, 0.5, &cfg()),
        0.0 + (1.0 - 1.0f64.min(0.75 / 0.5))
    );
    assert_eq!(blend_channel(M::ColorBurn, 0.25, 0.5, &cfg()), 0.0);
}
#[test]
fn burn_quotient_value() {
    assert_eq!(blend_channel(M::ColorBurn, 0.875, 0.5, &cfg()), 0.75);
}
#[test]
fn guards_do_not_touch_other_modes() {
    let c = Config {
        dodge_backdrop_zero: Guard::One,
        burn_source_zero: Guard::One,
        ..cfg()
    };
    assert_eq!(blend_channel(M::Multiply, 0.0, 0.5, &c), 0.0);
}
