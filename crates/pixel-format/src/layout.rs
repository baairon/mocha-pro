//! Channel orders and worlds with a row stride, after the meaning of the host's world structure:
//! a width, a height and a `rowbytes` that is a byte count, not a pixel count.

use crate::mode::Depth;
use std::fmt;
use std::ops::Range;

/// The order of the four channels inside one pixel in memory.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Order {
    /// Alpha, red, green, blue.
    Argb,
    /// Blue, green, red, alpha.
    Bgra,
    /// Red, green, blue, alpha.
    Rgba,
}

impl Order {
    /// Every order, in canonical order.
    pub const ALL: [Order; 3] = [Order::Argb, Order::Bgra, Order::Rgba];

    /// The canonical name.
    pub fn name(self) -> &'static str {
        match self {
            Order::Argb => "argb",
            Order::Bgra => "bgra",
            Order::Rgba => "rgba",
        }
    }

    /// Parses a canonical name.
    pub fn parse(s: &str) -> Option<Order> {
        Order::ALL.into_iter().find(|o| o.name() == s)
    }

    /// The memory slot (0 to 3) of channel `chan`, where channels are numbered R=0, G=1, B=2, A=3.
    pub fn slot_of(self, chan: usize) -> usize {
        const ARGB: [usize; 4] = [1, 2, 3, 0];
        const BGRA: [usize; 4] = [2, 1, 0, 3];
        const RGBA: [usize; 4] = [0, 1, 2, 3];
        match self {
            Order::Argb => ARGB[chan],
            Order::Bgra => BGRA[chan],
            Order::Rgba => RGBA[chan],
        }
    }
}

/// A refused layout or access.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayoutError {
    /// `rowbytes` is smaller than `width` pixels.
    RowTooShort { rowbytes: usize, needed: usize },
    /// The buffer is shorter than the layout reaches.
    BufferTooShort { len: usize, needed: usize },
    /// A pixel outside the world.
    OutOfRange { x: u32, y: u32 },
    /// The size arithmetic overflows `usize`.
    Overflow,
}

impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LayoutError::RowTooShort { rowbytes, needed } => {
                write!(
                    f,
                    "rowbytes {rowbytes} is below the {needed} bytes of one row"
                )
            }
            LayoutError::BufferTooShort { len, needed } => {
                write!(
                    f,
                    "buffer of {len} bytes is below the {needed} bytes needed"
                )
            }
            LayoutError::OutOfRange { x, y } => write!(f, "pixel ({x}, {y}) is outside the world"),
            LayoutError::Overflow => write!(f, "size arithmetic overflows"),
        }
    }
}

impl std::error::Error for LayoutError {}

/// Where the pixels of a world lie inside a byte buffer. Rows are `rowbytes` apart and the first
/// row starts at byte `origin`. Samples of 16 and 32 bit are little endian.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldLayout {
    /// Pixels per row.
    pub width: u32,
    /// Rows.
    pub height: u32,
    /// Bytes from the start of one row to the start of the next (may exceed the row's pixels).
    pub rowbytes: usize,
    /// Byte offset of the first pixel of the first row.
    pub origin: usize,
    /// The sample depth.
    pub depth: Depth,
    /// The channel order.
    pub order: Order,
}

impl WorldLayout {
    /// Bytes of one pixel: 4, 8 or 16.
    pub fn bytes_per_pixel(&self) -> usize {
        4 * self.depth.bytes_per_sample()
    }

    /// Bytes of the pixels of one row, without padding.
    pub fn row_len(&self) -> usize {
        self.width as usize * self.bytes_per_pixel()
    }

    /// `row_len` with the multiplication checked.
    fn checked_row_len(&self) -> Result<usize, LayoutError> {
        (self.width as usize)
            .checked_mul(self.bytes_per_pixel())
            .ok_or(LayoutError::Overflow)
    }

    /// The byte offset of the first byte of row `y`, with every step checked. The row's pixels
    /// end at most `bytes_per_pixel` bytes after any pixel offset, which is also checked.
    fn checked_row_start(&self, y: u32) -> Result<usize, LayoutError> {
        (y as usize)
            .checked_mul(self.rowbytes)
            .and_then(|v| v.checked_add(self.origin))
            .ok_or(LayoutError::Overflow)
    }

    /// The bytes the layout reaches from the start of the buffer: the end of the last row's pixels.
    pub fn required_len(&self) -> Result<usize, LayoutError> {
        if self.width == 0 || self.height == 0 {
            return Ok(self.origin);
        }
        let rows = (self.height as usize - 1)
            .checked_mul(self.rowbytes)
            .ok_or(LayoutError::Overflow)?;
        self.origin
            .checked_add(rows)
            .and_then(|v| v.checked_add(self.checked_row_len().ok()?))
            .ok_or(LayoutError::Overflow)
    }

    /// Checks `rowbytes` against the row and the buffer length against the reach.
    pub fn validate(&self, buffer_len: usize) -> Result<(), LayoutError> {
        if self.height > 1 && self.rowbytes < self.row_len() {
            return Err(LayoutError::RowTooShort {
                rowbytes: self.rowbytes,
                needed: self.row_len(),
            });
        }
        let needed = self.required_len()?;
        if buffer_len < needed {
            return Err(LayoutError::BufferTooShort {
                len: buffer_len,
                needed,
            });
        }
        Ok(())
    }

    /// The byte range of the pixels of row `y` (no padding).
    pub fn row_range(&self, y: u32) -> Result<Range<usize>, LayoutError> {
        if y >= self.height {
            return Err(LayoutError::OutOfRange { x: 0, y });
        }
        let start = self.checked_row_start(y)?;
        let end = start
            .checked_add(self.checked_row_len()?)
            .ok_or(LayoutError::Overflow)?;
        Ok(start..end)
    }

    /// The byte offset of the pixel at column `x`, row `y`.
    pub fn pixel_offset(&self, x: u32, y: u32) -> Result<usize, LayoutError> {
        if x >= self.width || y >= self.height {
            return Err(LayoutError::OutOfRange { x, y });
        }
        let base = (x as usize)
            .checked_mul(self.bytes_per_pixel())
            .and_then(|v| v.checked_add(self.checked_row_start(y).ok()?))
            .ok_or(LayoutError::Overflow)?;
        base.checked_add(self.bytes_per_pixel())
            .ok_or(LayoutError::Overflow)?;
        Ok(base)
    }

    /// Reads a pixel as raw sample bit patterns in the channel order R, G, B, A (an `f32` sample
    /// is its `to_bits`). The buffer must have passed `validate`.
    pub fn read_pixel(&self, data: &[u8], x: u32, y: u32) -> Result<[u32; 4], LayoutError> {
        let base = self.pixel_offset(x, y)?;
        let bps = self.depth.bytes_per_sample();
        let mut out = [0u32; 4];
        for (chan, slot) in out.iter_mut().enumerate() {
            let o = base + self.order.slot_of(chan) * bps;
            let bytes = data.get(o..o + bps).ok_or(LayoutError::BufferTooShort {
                len: data.len(),
                needed: o + bps,
            })?;
            *slot = match bps {
                1 => u32::from(bytes[0]),
                2 => u32::from(u16::from_le_bytes([bytes[0], bytes[1]])),
                _ => u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            };
        }
        Ok(out)
    }

    /// Writes a pixel given as raw sample bit patterns in the channel order R, G, B, A; at 8 and
    /// 16 bit the bits above the depth are dropped.
    pub fn write_pixel(
        &self,
        data: &mut [u8],
        x: u32,
        y: u32,
        px: [u32; 4],
    ) -> Result<(), LayoutError> {
        let base = self.pixel_offset(x, y)?;
        let bps = self.depth.bytes_per_sample();
        let len = data.len();
        for (chan, value) in px.iter().enumerate() {
            let o = base + self.order.slot_of(chan) * bps;
            let dst = data
                .get_mut(o..o + bps)
                .ok_or(LayoutError::BufferTooShort {
                    len,
                    needed: o + bps,
                })?;
            match bps {
                1 => dst[0] = *value as u8,
                2 => dst.copy_from_slice(&(*value as u16).to_le_bytes()),
                _ => dst.copy_from_slice(&value.to_le_bytes()),
            }
        }
        Ok(())
    }
}
