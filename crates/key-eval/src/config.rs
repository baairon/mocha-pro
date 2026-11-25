//! The configuration space of the evaluator. Every choice that a later calibration may need to vary
//! is a field of [`Config`]. The defaults are plain and claim nothing about any program.

/// Which key value is returned when the time equals a key time that has a hold segment before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// The value of the segment that ends at the key (the held value of the previous key).
    Left,
    /// The value of the segment that starts at the key (the value of the key itself).
    Right,
}

/// Snapping of the time to whole frames: `t' = round(t * frames_per_unit) / frames_per_unit`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Snap {
    /// The time is used as given.
    None,
    /// Largest whole frame not above the time.
    Floor,
    /// Nearest whole frame, an exact half going up.
    RoundHalfUp,
    /// Nearest whole frame, an exact half going to the even frame.
    RoundHalfEven,
}

/// How the components of a vector key are evaluated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VectorMode {
    /// Each component is a scalar curve with its own handles and its own curve parameter.
    PerComponent,
    /// One curve parameter per segment, solved from the time handles of component 0, and applied to the
    /// value handles of every component. Only Bezier segments differ between the two modes.
    Joint,
}

/// How the handle fields of a key are read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandleMode {
    /// The fields hold the points `(time, value)` themselves.
    Absolute,
    /// The fields hold offsets from the key point: the out handle of key `a` is `(ta + dt, va + dv)` and
    /// the in handle of key `b` is `(tb + dt, vb + dv)`.
    Offset,
}

/// Rule that keeps the time of a Bezier segment monotonic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clamp {
    /// Handles are used as given.
    None,
    /// The time extent of each handle is clamped to the segment: out handle time to `[ta, tb]`, in handle
    /// time to `[ta, tb]`. Values are unchanged. A cubic Bezier curve whose control polygon has
    /// `x0 <= x1 <= x2 <= x3` has a nondecreasing x(u) (Farin, Curves and Surfaces for CAGD, 5th ed., 2002,
    /// the chapter on Bezier curves, variation diminishing and convex hull properties); this rule imposes
    /// that condition on each handle separately.
    ClampTime,
    /// Both handle offsets (time and value) are multiplied by one common factor
    /// `min(1, L / out_extent, L / in_extent)` where `L = tb - ta` and an extent is the handle's time
    /// distance from its own key, counted only when positive.
    ScaleBoth,
}

/// Start value of the Newton solver.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NewtonStart {
    /// The curve parameter 0.5.
    Half,
    /// The normalized time of the query, `(t - ta) / (tb - ta)`, which is exact for a chord segment.
    Chord,
    /// A fixed curve parameter.
    Fixed(f64),
}

/// The solver of the Bezier curve parameter `u` with `x(u) = t`. Every variant does a fixed amount of work
/// that does not depend on the data.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Solver {
    /// Cardano's closed form (Cardano 1545; see the crate README), with a fixed own cube root, followed by
    /// `polish` Newton steps. With three real roots the closed form uses a cosine free route (see
    /// `solve.rs`). The root in `[0, 1]` closest to the interval and, on a tie, the smallest is taken. When the closed form root is not on the curve (a time residual above 1e-12, which happens when
    /// the cubic coefficient is a rounding error away from zero) bisection with 64 halvings replaces it.
    Cardano {
        /// Number of Newton refinement steps after the closed form.
        polish: u32,
    },
    /// Bisection on `[0, 1]` with a fixed number of halvings; the result is the midpoint of the last
    /// interval.
    Bisection {
        /// Number of halvings.
        steps: u32,
    },
    /// Newton iteration with a fixed number of steps from the stated start, each step clamped to `[0, 1]`.
    Newton {
        /// Number of steps.
        steps: u32,
        /// Start value.
        start: NewtonStart,
    },
}

/// Behaviour outside the keyed range, chosen separately for before the first key and after the last.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outside {
    /// The end key value.
    Hold,
    /// The end value plus the end slope times the distance.
    Linear,
    /// The key range repeats.
    Cycle,
    /// The key range repeats and each repeat is shifted by the last minus the first value.
    CycleOffset,
    /// The key range repeats, every other repeat run backwards.
    Mirror,
}

/// What to do with keys that share a time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coincident {
    /// Of keys with equal time only the first (in input order) is kept.
    KeepFirst,
    /// Of keys with equal time only the last (in input order) is kept.
    KeepLast,
    /// All are kept: the value jumps at that time; see [`Config::jump_side`].
    Jump,
}

/// The value of an empty key list.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Empty {
    /// Every component is 0.
    Zero,
    /// Every component is this value.
    Fill(f64),
}

/// The full configuration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    /// Side at a key time after a hold key (single keys only).
    pub hold_side: Side,
    /// Side at a time shared by several keys: `Left` gives the first of them, `Right` the last.
    pub jump_side: Side,
    /// Time snapping.
    pub snap: Snap,
    /// Frames per time unit used by the snapping; must be finite and positive.
    pub frames_per_unit: f64,
    /// Vector evaluation.
    pub vector_mode: VectorMode,
    /// Handle reading.
    pub handle_mode: HandleMode,
    /// Handle clamping.
    pub clamp: Clamp,
    /// Curve parameter solver.
    pub solver: Solver,
    /// Behaviour before the first key.
    pub before: Outside,
    /// Behaviour after the last key.
    pub after: Outside,
    /// Rule for equal key times, applied by `KeyList::new`.
    pub coincident: Coincident,
    /// Value of an empty list.
    pub empty: Empty,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            hold_side: Side::Right,
            jump_side: Side::Right,
            snap: Snap::None,
            frames_per_unit: 1.0,
            vector_mode: VectorMode::PerComponent,
            handle_mode: HandleMode::Absolute,
            clamp: Clamp::None,
            solver: Solver::Cardano { polish: 2 },
            before: Outside::Hold,
            after: Outside::Hold,
            coincident: Coincident::KeepLast,
            empty: Empty::Zero,
        }
    }
}

/// The values of each axis that [`Config::enumerate`] walks, in order.
pub struct Axes {
    /// Hold sides.
    pub hold_side: Vec<Side>,
    /// Jump sides.
    pub jump_side: Vec<Side>,
    /// Snap modes.
    pub snap: Vec<Snap>,
    /// Vector modes.
    pub vector_mode: Vec<VectorMode>,
    /// Handle modes.
    pub handle_mode: Vec<HandleMode>,
    /// Clamp rules.
    pub clamp: Vec<Clamp>,
    /// Solvers: closed form without and with polish, bisection, Newton from both named starts.
    pub solver: Vec<Solver>,
    /// Outside rules (used for both sides).
    pub outside: Vec<Outside>,
    /// Coincident rules.
    pub coincident: Vec<Coincident>,
    /// Empty rules.
    pub empty: Vec<Empty>,
}

impl Config {
    /// The axes of the configuration space. `frames_per_unit` stays 1.0 (a scale, not a choice).
    pub fn axes() -> Axes {
        Axes {
            hold_side: vec![Side::Left, Side::Right],
            jump_side: vec![Side::Left, Side::Right],
            snap: vec![
                Snap::None,
                Snap::Floor,
                Snap::RoundHalfUp,
                Snap::RoundHalfEven,
            ],
            vector_mode: vec![VectorMode::PerComponent, VectorMode::Joint],
            handle_mode: vec![HandleMode::Absolute, HandleMode::Offset],
            clamp: vec![Clamp::None, Clamp::ClampTime, Clamp::ScaleBoth],
            solver: vec![
                Solver::Cardano { polish: 0 },
                Solver::Cardano { polish: 2 },
                Solver::Bisection { steps: 60 },
                Solver::Newton {
                    steps: 8,
                    start: NewtonStart::Half,
                },
                Solver::Newton {
                    steps: 8,
                    start: NewtonStart::Chord,
                },
            ],
            outside: vec![
                Outside::Hold,
                Outside::Linear,
                Outside::Cycle,
                Outside::CycleOffset,
                Outside::Mirror,
            ],
            coincident: vec![
                Coincident::KeepFirst,
                Coincident::KeepLast,
                Coincident::Jump,
            ],
            empty: vec![Empty::Zero, Empty::Fill(1.0)],
        }
    }

    /// Every combination of the axes (before and after vary independently): the configuration space.
    pub fn enumerate() -> Vec<Config> {
        let ax = Config::axes();
        let mut out = Vec::new();
        for &hold_side in &ax.hold_side {
            for &jump_side in &ax.jump_side {
                for &snap in &ax.snap {
                    for &vector_mode in &ax.vector_mode {
                        for &handle_mode in &ax.handle_mode {
                            for &clamp in &ax.clamp {
                                for &solver in &ax.solver {
                                    for &before in &ax.outside {
                                        for &after in &ax.outside {
                                            for &coincident in &ax.coincident {
                                                for &empty in &ax.empty {
                                                    out.push(Config {
                                                        hold_side,
                                                        jump_side,
                                                        snap,
                                                        frames_per_unit: 1.0,
                                                        vector_mode,
                                                        handle_mode,
                                                        clamp,
                                                        solver,
                                                        before,
                                                        after,
                                                        coincident,
                                                        empty,
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        out
    }
}
