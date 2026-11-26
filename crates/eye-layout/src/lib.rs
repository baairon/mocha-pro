//! Clean-room implementation of an eye layout description. std only.
//!
//! Every value that the original plug-in holds in its parameter tables (selector positions, the
//! separator position, labels, counts) is supplied by the caller in a [`RoleTable`]. The crate
//! ships only the made up [`RoleTable::sample`] for tests, examples and documentation.

/// Errors. `Unspecified` names a point the description does not state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutError {
    /// The selector position is a separator of the table.
    SeparatorPosition,
    /// The selector position has no row in the table.
    NoRow(i32),
    /// An eye index other than 0 or 1.
    BadEyeIndex(u32),
    /// Behaviour the description does not state.
    Unspecified(&'static str),
}

/// Errors of building a [`RoleTable`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableError {
    /// A position appears twice among the rows and separators.
    DuplicatePosition(i32),
}

/// Layout kind of a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    /// One eye, the whole frame.
    Whole,
    /// Two eyes in two separate layers.
    TwoLayers,
    /// Two eyes stacked in one frame, one above the other.
    Stacked,
    /// Two eyes side by side in one frame.
    SideBySide,
}

/// Layout kind and the spherical flag of one selector row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutMode {
    pub layout: Layout,
    pub spherical: bool,
}

/// One row of the role table: a selector position and what it stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutRow {
    pub position: i32,
    pub layout: Layout,
    pub spherical: bool,
}

/// The caller supplied table that maps a layout selector position to a [`LayoutMode`].
///
/// Positions listed in `separators` have no row and are refused with
/// [`LayoutError::SeparatorPosition`]; a position in neither list is [`LayoutError::NoRow`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleTable {
    rows: Vec<LayoutRow>,
    separators: Vec<i32>,
}

impl RoleTable {
    /// Builds a table. A position may appear once among the rows and separators.
    pub fn new(rows: Vec<LayoutRow>, separators: Vec<i32>) -> Result<Self, TableError> {
        let mut seen: Vec<i32> = Vec::new();
        for p in rows
            .iter()
            .map(|r| r.position)
            .chain(separators.iter().copied())
        {
            if seen.contains(&p) {
                return Err(TableError::DuplicatePosition(p));
            }
            seen.push(p);
        }
        Ok(RoleTable { rows, separators })
    }

    /// A made up table for tests, examples and documentation. It is not any real table.
    pub fn sample() -> Self {
        let row = |position, layout, spherical| LayoutRow {
            position,
            layout,
            spherical,
        };
        RoleTable::new(
            vec![
                row(201, Layout::Whole, false),
                row(202, Layout::Stacked, false),
                row(203, Layout::SideBySide, false),
                row(205, Layout::TwoLayers, false),
                row(207, Layout::Stacked, true),
                row(208, Layout::SideBySide, true),
            ],
            vec![204, 206],
        )
        .expect("the sample positions are distinct")
    }

    /// The rows of the table in the order supplied.
    pub fn rows(&self) -> &[LayoutRow] {
        &self.rows
    }

    /// The separator positions.
    pub fn separators(&self) -> &[i32] {
        &self.separators
    }

    /// Selector value to layout mode. The host popup value is one based: the position looked
    /// up is the value minus one.
    pub fn parse_selector(&self, popup_value: i32) -> Result<LayoutMode, LayoutError> {
        let p = popup_value
            .checked_sub(1)
            .ok_or(LayoutError::NoRow(i32::MIN))?;
        if self.separators.contains(&p) {
            return Err(LayoutError::SeparatorPosition);
        }
        for row in &self.rows {
            if row.position == p {
                return Ok(LayoutMode {
                    layout: row.layout,
                    spherical: row.spherical,
                });
            }
        }
        Err(LayoutError::NoRow(p))
    }
}

/// Plug-in error of the first preparation call.
pub const FIRST_PREP_ERROR: i32 = 517;
/// Constant given to the helper of the second call for the two layer layout.
pub const TWO_LAYER_HELPER_CONSTANT: i32 = 8;
/// Width and height fraction of one packed eye.
const HALF: f64 = 0.5;
/// Scale double of the clip description for packed layouts.
pub const CLIP_SCALE: f64 = 0.5;

/// How the odd size +1 offset is chosen (a quirk of the description).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OddOffset {
    /// As the description reads: tied to the eye index (index 1 side by side, index 0 stacked).
    ByEyeIndex,
    /// Alternative: tied to the eye position (second eye in memory order).
    ByEyePosition,
}

/// Every inferred statement or open point of the description; defaults are the preferred reading.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Readings {
    /// Which eye gets the odd size offset.
    pub odd_offset: OddOffset,
    /// Clamp an eye index above 1 to 1 instead of an error.
    pub clamp_eye_index: bool,
    /// Eye index a plain (not two layer) request carries: value of a zero argument accessor.
    pub plain_request_eye: u8,
    /// The copy path narrows for the packed layouts (the description reads that it does not).
    pub copy_path_narrows: bool,
    /// The clip scale is present for packed layouts.
    pub clip_scale_enabled: bool,
    /// Variant code answered when the spherical flag is set.
    pub code_spherical: u32,
    /// Variant code answered when the spherical flag is clear.
    pub code_plain: u32,
}

impl Default for Readings {
    fn default() -> Self {
        Readings {
            odd_offset: OddOffset::ByEyeIndex,
            clamp_eye_index: false,
            plain_request_eye: 0,
            copy_path_narrows: false,
            clip_scale_enabled: true,
            code_spherical: 6,
            code_plain: 7,
        }
    }
}

/// Source of an image read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// The input layer.
    Input,
    /// The extra source layer, never split.
    Extra,
    /// The second eye layer.
    SecondLayer,
}

/// Layout map entry of a source: the input and the second layer take the layout, the extra
/// source is always [`Layout::Whole`].
pub fn map_entry(source: Source, layout: Layout) -> Layout {
    match source {
        Source::Input | Source::SecondLayer => layout,
        Source::Extra => Layout::Whole,
    }
}

/// Engine split code: stacked gives 1, side by side gives 2, anything else 0.
pub fn split_code(layout: Layout) -> u8 {
    match layout {
        Layout::Stacked => 1,
        Layout::SideBySide => 2,
        _ => 0,
    }
}

/// Scale of the clip description. `mode_answer` and `adapter_test` are the host adapter answers
/// the description names; both are caller supplied.
pub fn clip_scale(
    layout: Layout,
    mode_answer: i32,
    adapter_test: bool,
    r: &Readings,
) -> Option<f64> {
    if r.clip_scale_enabled && split_code(layout) != 0 && mode_answer == 1 && !adapter_test {
        Some(CLIP_SCALE)
    } else {
        None
    }
}

/// Variant code answer on the After Effects host; `host_check_ok` false returns the host's own
/// code.
pub fn variant_code(mode: LayoutMode, host_check_ok: bool, host_code: u32, r: &Readings) -> u32 {
    if !host_check_ok {
        host_code
    } else if mode.spherical {
        r.code_spherical
    } else {
        r.code_plain
    }
}

/// Rectangle in pixels: origin (x0, y0) and size (w, h). Inclusive end is x0 + w - 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x0: u32,
    pub y0: u32,
    pub w: u32,
    pub h: u32,
}

fn check_eye(eye: u32, r: &Readings) -> Result<u32, LayoutError> {
    if eye <= 1 {
        Ok(eye)
    } else if r.clamp_eye_index {
        Ok(1)
    } else {
        Err(LayoutError::BadEyeIndex(eye))
    }
}

/// Eye rectangle in a host frame of `w` by `h`, eye index `eye`, exchange flag `exchanged`.
/// `bottom_up` is false on After Effects.
pub fn narrow(
    layout: Layout,
    w: u32,
    h: u32,
    eye: u32,
    exchanged: bool,
    bottom_up: bool,
    r: &Readings,
) -> Result<Rect, LayoutError> {
    let c = check_eye(eye, r)?;
    if layout != Layout::Stacked && layout != Layout::SideBySide {
        return Ok(Rect { x0: 0, y0: 0, w, h });
    }
    // step 1
    let s1 = match layout {
        Layout::Stacked if !bottom_up => !exchanged,
        _ => exchanged,
    };
    // step 2
    let v1: u32 = if s1 { 1 - c } else { c };
    let wf = w as f64;
    let hf = h as f64;
    match layout {
        Layout::SideBySide => {
            // step 3
            let lead_fraction = v1 as f64 * HALF;
            let mut x0 = (lead_fraction * wf) as u32;
            let extra = match r.odd_offset {
                OddOffset::ByEyeIndex => c == 1,
                OddOffset::ByEyePosition => v1 == 1,
            };
            if extra && w % 2 == 1 {
                x0 += 1;
            }
            let we = (HALF * wf) as u32;
            Ok(Rect {
                x0,
                y0: 0,
                w: we,
                h,
            })
        }
        _ => {
            // step 4
            let top_fraction = (1 - v1) as f64 * HALF;
            let mut y0 = (top_fraction * hf) as u32;
            let extra = match r.odd_offset {
                OddOffset::ByEyeIndex => c == 0,
                OddOffset::ByEyePosition => v1 == 0,
            };
            if extra && h % 2 == 1 {
                y0 += 1;
            }
            let he = (HALF * hf) as u32;
            Ok(Rect {
                x0: 0,
                y0,
                w,
                h: he,
            })
        }
    }
}

/// Layer an engine input read uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Input,
    SecondLayer,
}

/// The eye field the engine computes for a frame request.
pub fn request_eye(
    layout: Layout,
    exchanged: bool,
    eye: u32,
    r: &Readings,
) -> Result<u8, LayoutError> {
    let c = check_eye(eye, r)?;
    if layout == Layout::TwoLayers {
        Ok((c as u8) ^ (exchanged as u8))
    } else {
        Ok(r.plain_request_eye)
    }
}

/// The plug-in rule: eye 1 on a source whose map entry is the two layer layout becomes the
/// second layer.
pub fn resolve_source(source: Source, eye: u8, layout: Layout) -> Source {
    if eye == 1 && map_entry(source, layout) == Layout::TwoLayers {
        Source::SecondLayer
    } else {
        source
    }
}

/// The input read of the input layer source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputRead {
    pub layer: Layer,
    pub rect: Rect,
}

/// The input read for an eye of a frame of `w` by `h`.
pub fn input_read(
    layout: Layout,
    w: u32,
    h: u32,
    eye: u32,
    exchanged: bool,
    r: &Readings,
) -> Result<InputRead, LayoutError> {
    let c = check_eye(eye, r)?;
    let layer = if layout == Layout::TwoLayers && (c ^ exchanged as u32) == 1 {
        Layer::SecondLayer
    } else {
        Layer::Input
    };
    // After Effects wrapper answers "not bottom up".
    let rect = narrow(layout, w, h, c, exchanged, false, r)?;
    Ok(InputRead { layer, rect })
}

/// The output rectangle (same routine, same eye and flag).
pub fn output_rect(
    layout: Layout,
    w: u32,
    h: u32,
    eye: u32,
    exchanged: bool,
    r: &Readings,
) -> Result<Rect, LayoutError> {
    narrow(layout, w, h, eye, exchanged, false, r)
}

/// The engine drops its cached image when the narrowed size differs from the expected.
pub fn cache_still_valid(narrowed: Rect, expected_w: u32, expected_h: u32) -> bool {
    narrowed.w == expected_w && narrowed.h == expected_h
}

/// Copy an eye of `rect.w` by `rect.h` into `dst` (host frame of `dst_w` pixels per row) at the
/// rectangle, rows in the same order. Pixels outside the rectangle are untouched. A vertical
/// flip and a channel reorder are not part of this routine.
pub fn place_eye<T: Copy>(
    dst: &mut [T],
    dst_w: u32,
    rect: Rect,
    eye: &[T],
) -> Result<(), LayoutError> {
    let (rw, rh, dw) = (rect.w as usize, rect.h as usize, dst_w as usize);
    if eye.len() != rw * rh {
        return Err(LayoutError::Unspecified("eye size differs from rectangle"));
    }
    // Validate the whole rectangle before the first copy: a refusal leaves the frame untouched.
    let (x0, y0) = (rect.x0 as usize, rect.y0 as usize);
    let outside = || LayoutError::Unspecified("rectangle outside the frame");
    let row_end = x0.checked_add(rw).ok_or_else(outside)?;
    if row_end > dw {
        return Err(outside());
    }
    if rh > 0 {
        let last_row = y0.checked_add(rh - 1).ok_or_else(outside)?;
        let end = last_row
            .checked_mul(dw)
            .and_then(|v| v.checked_add(row_end))
            .ok_or_else(outside)?;
        if end > dst.len() {
            return Err(outside());
        }
    }
    for row in 0..rh {
        let d = (y0 + row) * dw + x0;
        dst[d..d + rw].copy_from_slice(&eye[row * rw..(row + 1) * rw]);
    }
    Ok(())
}

/// Eye index of the first engine call (two layers: the output eye selector, 0 or 1).
pub fn first_call_eye(layout: Layout, output_eye: u32) -> Result<u32, LayoutError> {
    if layout == Layout::TwoLayers {
        if output_eye > 1 {
            return Err(LayoutError::BadEyeIndex(output_eye));
        }
        Ok(output_eye)
    } else {
        Ok(0)
    }
}

/// Argument of the second engine call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecondCall {
    /// Whole frame layout: no second call.
    None,
    /// Packed layouts: a pair (x, y).
    Pair(i32, i32),
    /// Two layers: an object from the helper with the constant and the pair (0, 0).
    Helper { constant: i32, pair: (i32, i32) },
}

/// Arguments of the first engine call.
pub const FIRST_CALL_ARGS: (i32, i32) = (0, 0);

/// Second call arguments, `win` and `hin` the unscaled input layer size.
pub fn second_call(layout: Layout, win: i32, hin: i32) -> SecondCall {
    match layout {
        Layout::Whole => SecondCall::None,
        Layout::Stacked => SecondCall::Pair(0, (hin as f64 / 2.0) as i32),
        Layout::SideBySide => SecondCall::Pair((win as f64 / 2.0) as i32, 0),
        Layout::TwoLayers => SecondCall::Helper {
            constant: TWO_LAYER_HELPER_CONSTANT,
            pair: (0, 0),
        },
    }
}

/// Eye index of the second call.
pub const SECOND_CALL_EYE: u32 = 1;

/// Layer copied when the engine is not called.
pub fn copy_path_layer(layout: Layout, output_eye: u32, second_layer_present: bool) -> Layer {
    if layout == Layout::TwoLayers && output_eye == 1 && second_layer_present {
        Layer::SecondLayer
    } else {
        Layer::Input
    }
}

/// Rectangle on the copy path. The description reads that nothing narrows (whole frame).
pub fn copy_path_rect(layout: Layout, w: u32, h: u32, r: &Readings) -> Result<Rect, LayoutError> {
    if r.copy_path_narrows && (layout == Layout::Stacked || layout == Layout::SideBySide) {
        return Err(LayoutError::Unspecified(
            "copy path narrowing is not stated",
        ));
    }
    Ok(Rect { x0: 0, y0: 0, w, h })
}
