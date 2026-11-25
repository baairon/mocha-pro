//! Pixel orders, strides, origins and storage depth mapping.

use frame_io::Depth as Storage;
use pixel_format::{Depth, LayoutError, Order, WorldLayout};

fn world(
    depth: Depth,
    order: Order,
    width: u32,
    height: u32,
    rowbytes: usize,
    origin: usize,
) -> WorldLayout {
    WorldLayout {
        width,
        height,
        rowbytes,
        origin,
        depth,
        order,
    }
}

#[test]
fn slots_of_each_order() {
    // channels R=0 G=1 B=2 A=3 and the memory slot of each
    let slots = |o: Order| [0, 1, 2, 3].map(|c| o.slot_of(c));
    assert_eq!(slots(Order::Argb), [1, 2, 3, 0]);
    assert_eq!(slots(Order::Bgra), [2, 1, 0, 3]);
    assert_eq!(slots(Order::Rgba), [0, 1, 2, 3]);
}

#[test]
fn eight_bit_bytes_in_each_order() {
    for (order, bytes) in [
        (Order::Argb, [40, 10, 20, 30]),
        (Order::Bgra, [30, 20, 10, 40]),
        (Order::Rgba, [10, 20, 30, 40]),
    ] {
        let w = world(Depth::U8, order, 1, 1, 4, 0);
        let mut buf = [0u8; 4];
        w.write_pixel(&mut buf, 0, 0, [10, 20, 30, 40]).unwrap();
        assert_eq!(buf, bytes, "{order:?}");
        assert_eq!(w.read_pixel(&buf, 0, 0).unwrap(), [10, 20, 30, 40]);
    }
}

#[test]
fn sixteen_bit_is_little_endian() {
    let w = world(Depth::U16 { max: 32768 }, Order::Argb, 1, 1, 8, 0);
    let mut buf = [0u8; 8];
    w.write_pixel(&mut buf, 0, 0, [0x0102, 0x0304, 0x0506, 0x0708])
        .unwrap();
    // A, R, G, B
    assert_eq!(buf, [0x08, 0x07, 0x02, 0x01, 0x04, 0x03, 0x06, 0x05]);
    assert_eq!(
        w.read_pixel(&buf, 0, 0).unwrap(),
        [0x0102, 0x0304, 0x0506, 0x0708]
    );
}

#[test]
fn float_samples_round_trip_as_bits() {
    let w = world(Depth::F32, Order::Bgra, 1, 1, 16, 0);
    let px = [
        1.0f32.to_bits(),
        (-0.0f32).to_bits(),
        f32::NAN.to_bits(),
        0.25f32.to_bits(),
    ];
    let mut buf = [0u8; 16];
    w.write_pixel(&mut buf, 0, 0, px).unwrap();
    assert_eq!(&buf[0..4], &px[2].to_le_bytes()); // blue first
    assert_eq!(&buf[12..16], &px[3].to_le_bytes()); // alpha last
    assert_eq!(w.read_pixel(&buf, 0, 0).unwrap(), px);
}

#[test]
fn eight_and_sixteen_bit_writes_drop_high_bits() {
    let w8 = world(Depth::U8, Order::Rgba, 1, 1, 4, 0);
    let mut b8 = [0u8; 4];
    w8.write_pixel(&mut b8, 0, 0, [0x1FF, 0x100, 0x07F, 0x2AB])
        .unwrap();
    assert_eq!(b8, [0xFF, 0x00, 0x7F, 0xAB]);
    let w16 = world(Depth::U16 { max: 65535 }, Order::Rgba, 1, 1, 8, 0);
    let mut b16 = [0u8; 8];
    w16.write_pixel(&mut b16, 0, 0, [0x1_0001, 0, 0, 0])
        .unwrap();
    assert_eq!(&b16[0..2], &[1, 0]);
}

#[test]
fn rowbytes_stride_and_origin_place_pixels() {
    // 3 x 2 pixels of 8 bit, rows 16 bytes apart, first pixel at byte 5
    let w = world(Depth::U8, Order::Rgba, 3, 2, 16, 5);
    assert_eq!(w.pixel_offset(0, 0).unwrap(), 5);
    assert_eq!(w.pixel_offset(2, 0).unwrap(), 13);
    assert_eq!(w.pixel_offset(0, 1).unwrap(), 21);
    assert_eq!(w.pixel_offset(2, 1).unwrap(), 29);
    assert_eq!(w.row_range(1).unwrap(), 21..33);
    assert_eq!(w.row_len(), 12);
    assert_eq!(w.required_len().unwrap(), 33);
}

#[test]
fn padding_bytes_are_not_touched() {
    let w = world(Depth::U8, Order::Rgba, 2, 2, 12, 0);
    let mut buf = vec![0xEEu8; 12 * 2];
    for y in 0..2 {
        for x in 0..2 {
            w.write_pixel(&mut buf, x, y, [1, 2, 3, 4]).unwrap();
        }
    }
    for y in 0..2usize {
        assert_eq!(&buf[y * 12 + 8..y * 12 + 12], &[0xEE; 4]);
        assert_eq!(&buf[y * 12..y * 12 + 8], &[1, 2, 3, 4, 1, 2, 3, 4]);
    }
}

#[test]
fn validate_checks_row_and_buffer() {
    let w = world(Depth::U8, Order::Rgba, 4, 3, 16, 2);
    assert_eq!(w.required_len().unwrap(), 2 + 2 * 16 + 16);
    assert!(w.validate(50).is_ok());
    assert_eq!(
        w.validate(49),
        Err(LayoutError::BufferTooShort {
            len: 49,
            needed: 50
        })
    );
    let narrow = world(Depth::U8, Order::Rgba, 4, 3, 15, 0);
    assert_eq!(
        narrow.validate(1000),
        Err(LayoutError::RowTooShort {
            rowbytes: 15,
            needed: 16
        })
    );
    // a single row may have any rowbytes
    let one = world(Depth::U8, Order::Rgba, 4, 1, 0, 0);
    assert!(one.validate(16).is_ok());
}

#[test]
fn empty_world_reaches_only_its_origin() {
    let w = world(Depth::F32, Order::Argb, 0, 5, 64, 7);
    assert_eq!(w.required_len().unwrap(), 7);
    assert!(w.validate(7).is_ok());
    let h = world(Depth::U8, Order::Argb, 5, 0, 64, 9);
    assert_eq!(h.required_len().unwrap(), 9);
}

#[test]
fn out_of_range_access_is_refused() {
    let w = world(Depth::U8, Order::Rgba, 2, 2, 8, 0);
    let mut buf = [0u8; 16];
    assert_eq!(
        w.pixel_offset(2, 0),
        Err(LayoutError::OutOfRange { x: 2, y: 0 })
    );
    assert_eq!(
        w.read_pixel(&buf, 0, 2),
        Err(LayoutError::OutOfRange { x: 0, y: 2 })
    );
    assert_eq!(
        w.write_pixel(&mut buf, 5, 5, [0; 4]),
        Err(LayoutError::OutOfRange { x: 5, y: 5 })
    );
    assert_eq!(w.row_range(2), Err(LayoutError::OutOfRange { x: 0, y: 2 }));
}

#[test]
fn short_buffer_is_refused_on_access() {
    let w = world(Depth::U8, Order::Rgba, 2, 1, 8, 0);
    let mut short = [0u8; 6];
    assert!(matches!(
        w.read_pixel(&short, 1, 0),
        Err(LayoutError::BufferTooShort { .. })
    ));
    assert!(matches!(
        w.write_pixel(&mut short, 1, 0, [1, 2, 3, 4]),
        Err(LayoutError::BufferTooShort { .. })
    ));
}

#[test]
fn size_overflow_is_reported() {
    let w = world(Depth::F32, Order::Rgba, 1000, u32::MAX, usize::MAX / 2, 0);
    assert_eq!(w.required_len(), Err(LayoutError::Overflow));
}

#[test]
fn bytes_per_pixel_by_depth() {
    let bpp = |d| world(d, Order::Rgba, 1, 1, 0, 0).bytes_per_pixel();
    assert_eq!(bpp(Depth::U8), 4);
    assert_eq!(bpp(Depth::U16 { max: 1 }), 8);
    assert_eq!(bpp(Depth::F32), 16);
}

#[test]
fn storage_depth_is_reused_from_frame_io() {
    assert_eq!(Depth::U8.storage(), Storage::Eight);
    assert_eq!(Depth::U16 { max: 7 }.storage(), Storage::Sixteen);
    assert_eq!(Depth::F32.storage(), Storage::ThirtyTwo);
    assert_eq!(Depth::from_storage(Storage::Eight, 9), Depth::U8);
    assert_eq!(
        Depth::from_storage(Storage::Sixteen, 9),
        Depth::U16 { max: 9 }
    );
    assert_eq!(Depth::from_storage(Storage::ThirtyTwo, 9), Depth::F32);
    assert_eq!(Depth::U16 { max: 9 }.bytes_per_sample(), 2);
    assert_eq!(Depth::F32.bytes_per_sample(), 4);
    assert_eq!(Depth::U8.bytes_per_sample(), 1);
}

#[test]
fn depth_full_scale_values() {
    assert_eq!(Depth::U8.max_value(), 255);
    assert_eq!(Depth::U16 { max: 32768 }.max_value(), 32768);
    assert_eq!(Depth::F32.max_value(), 0);
    assert!(Depth::F32.is_float());
    assert!(!Depth::U8.is_float());
}

#[test]
fn order_names_parse_back() {
    for o in Order::ALL {
        assert_eq!(Order::parse(o.name()), Some(o));
    }
    assert_eq!(Order::parse("abgr"), None);
}

#[test]
fn layout_errors_display() {
    assert!(LayoutError::Overflow.to_string().contains("overflow"));
    assert!(LayoutError::OutOfRange { x: 1, y: 2 }
        .to_string()
        .contains("(1, 2)"));
}

#[test]
fn offsets_that_overflow_usize_are_refused_not_wrapped() {
    // rowbytes so large that origin + y * rowbytes does not fit in usize
    let w = world(Depth::U8, Order::Rgba, 1, 4, usize::MAX / 2 + 1, 0);
    assert_eq!(w.pixel_offset(0, 1), Ok(usize::MAX / 2 + 1));
    assert_eq!(w.pixel_offset(0, 2), Err(LayoutError::Overflow));
    assert_eq!(w.row_range(2), Err(LayoutError::Overflow));
    let mut buf = [0u8; 8];
    assert_eq!(w.read_pixel(&buf, 0, 2), Err(LayoutError::Overflow));
    assert_eq!(
        w.write_pixel(&mut buf, 0, 3, [1, 2, 3, 4]),
        Err(LayoutError::Overflow)
    );
    // a pixel whose last byte would pass usize::MAX
    let w = world(Depth::U8, Order::Rgba, 1, 1, 4, usize::MAX - 1);
    assert_eq!(w.pixel_offset(0, 0), Err(LayoutError::Overflow));
    assert_eq!(w.row_range(0), Err(LayoutError::Overflow));
    assert_eq!(w.read_pixel(&buf, 0, 0), Err(LayoutError::Overflow));
}
