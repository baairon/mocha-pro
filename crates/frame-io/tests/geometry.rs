use frame_io::{Depth, Geometry, GeometryError, Rate, MAX_FRAME_BYTES};

fn rate() -> Rate {
    Rate { num: 30, den: 1 }
}

#[test]
fn bytes_per_pixel_follow_the_convention() {
    assert_eq!(Depth::Eight.bytes_per_pixel(), 4);
    assert_eq!(Depth::Sixteen.bytes_per_pixel(), 8);
    assert_eq!(Depth::ThirtyTwo.bytes_per_pixel(), 16);
    assert_eq!(Depth::Eight.bytes_per_sample(), 1);
    assert_eq!(Depth::Sixteen.bytes_per_sample(), 2);
    assert_eq!(Depth::ThirtyTwo.bytes_per_sample(), 4);
}

#[test]
fn pixel_format_names_and_layout() {
    assert_eq!(Depth::Eight.pix_fmt(), "rgba");
    assert_eq!(Depth::Sixteen.pix_fmt(), "rgba64le");
    assert_eq!(Depth::ThirtyTwo.pix_fmt(), "gbrapf32le");
    assert!(!Depth::Eight.is_planar());
    assert!(!Depth::Sixteen.is_planar());
    assert!(Depth::ThirtyTwo.is_planar());
}

#[test]
fn depth_from_bits_accepts_only_three_values() {
    for (bits, d) in [
        (8, Depth::Eight),
        (16, Depth::Sixteen),
        (32, Depth::ThirtyTwo),
    ] {
        assert_eq!(Depth::from_bits(bits), Ok(d));
        assert_eq!(d.bits(), bits);
    }
    for bad in [0, 1, 7, 10, 12, 24, 64] {
        assert_eq!(
            Depth::from_bits(bad),
            Err(GeometryError::UnsupportedDepth(bad))
        );
    }
}

#[test]
fn frame_size_is_width_height_bytes_per_pixel() {
    let g = Geometry::new(5, 3, Depth::Sixteen, rate(), None).unwrap();
    assert_eq!(g.pixels(), 15);
    assert_eq!(g.frame_bytes(), 5 * 3 * 8);
    assert_eq!(g.samples(), 60);
    let g = Geometry::new(7, 2, Depth::ThirtyTwo, rate(), Some(3)).unwrap();
    assert_eq!(g.frame_bytes(), 7 * 2 * 16);
    assert_eq!(g.stream_bytes(), Some(3 * 7 * 2 * 16));
    assert_eq!(
        Geometry::new(7, 2, Depth::Eight, rate(), None)
            .unwrap()
            .stream_bytes(),
        None
    );
}

#[test]
fn zero_sides_are_refused() {
    assert_eq!(
        Geometry::new(0, 4, Depth::Eight, rate(), None),
        Err(GeometryError::ZeroWidth)
    );
    assert_eq!(
        Geometry::new(4, 0, Depth::Eight, rate(), None),
        Err(GeometryError::ZeroHeight)
    );
}

#[test]
fn a_frame_of_exactly_four_gib_is_allowed_and_one_pixel_more_is_not() {
    // 65536 x 65536 x 1 byte-per-sample x 4 = 2^34 would be too large; use depth 8 sizes.
    // 32768 x 32768 x 4 bytes = 2^32 bytes = 4 GiB exactly.
    let g = Geometry::new(32768, 32768, Depth::Eight, rate(), None).unwrap();
    assert_eq!(g.frame_bytes(), MAX_FRAME_BYTES);
    // 4 GiB plus one row of 4 * 32768 bytes.
    assert_eq!(
        Geometry::new(32768, 32769, Depth::Eight, rate(), None),
        Err(GeometryError::FrameTooLarge(
            MAX_FRAME_BYTES as u128 + 4 * 32768
        ))
    );
    // Sixteen bit: half the pixels fit.
    assert!(Geometry::new(32768, 16384, Depth::Sixteen, rate(), None).is_ok());
    assert!(Geometry::new(32768, 16385, Depth::Sixteen, rate(), None).is_err());
    assert!(Geometry::new(16384, 16384, Depth::ThirtyTwo, rate(), None).is_ok());
    assert!(Geometry::new(16385, 16384, Depth::ThirtyTwo, rate(), None).is_err());
}

#[test]
fn the_size_check_does_not_overflow() {
    assert!(matches!(
        Geometry::new(u32::MAX, u32::MAX, Depth::ThirtyTwo, rate(), None),
        Err(GeometryError::FrameTooLarge(_))
    ));
}

#[test]
fn rates_are_exact_fractions() {
    let r: Rate = "30000/1001".parse().unwrap();
    assert_eq!((r.num, r.den), (30000, 1001));
    assert_eq!(r.to_string(), "30000/1001");
    let kept: Rate = "60/2".parse().unwrap();
    assert_eq!(kept.to_string(), "60/2");
    assert!(kept.same_value(&"30/1".parse().unwrap()));
    assert!(!kept.same_value(&"30000/1001".parse().unwrap()));
}

#[test]
fn decimal_and_malformed_rates_are_refused() {
    for bad in [
        "29.97",
        "30",
        "/",
        "30/",
        "/1",
        "0/1",
        "1/0",
        "-30/1",
        "30/-1",
        "+30/1",
        " 30/1",
        "30 /1",
        "30/1/1",
        "a/b",
        "",
        "4294967296/1",
    ] {
        assert!(bad.parse::<Rate>().is_err(), "{bad:?} must be refused");
    }
    assert!(Rate::new(0, 1).is_err());
    assert!(Rate::new(1, 0).is_err());
    assert!(Rate::new(1, 1).is_ok());
}

#[test]
fn declared_frame_count_is_checked() {
    let g = Geometry::new(2, 2, Depth::Eight, rate(), Some(4)).unwrap();
    assert!(g.check_count(4).is_ok());
    assert!(g.check_count(3).is_err());
    assert!(g.check_count(5).is_err());
    let open = Geometry::new(2, 2, Depth::Eight, rate(), None).unwrap();
    assert!(open.check_count(0).is_ok());
    assert!(open.check_count(99).is_ok());
}
