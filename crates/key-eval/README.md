# key-eval

Evaluation of keyframed values at a time, in f64, with no dependency and no unsafe code.

A `KeyList<N>` holds keys with a time, a value of `N` components and an interpolation kind. `KeyList::evaluate`
returns the value at a time under a `Config`. Every choice that a comparison against reference values might need
to vary is a field of `Config`, each with a documented plain default, and `Config::enumerate` lists the whole
configuration space (144000 combinations with the standard axes).

## Public API

- `Key::hold`, `Key::linear`, `Key::hermite` (in and out slopes per component) and `Key::bezier` (an in and an out
  handle per component, as `[time, value]`). The kind of a key governs the segment that starts at it.
- `KeyList::new(keys, &Config)` sorts by time with a stable sort, refuses non finite fields, and applies the
  coincident time rule. `KeyList::evaluate(&Config, t)` returns `[f64; N]`.
- `Config` fields: `hold_side`, `jump_side`, `snap` with `frames_per_unit`, `vector_mode` (per component or joint),
  `handle_mode` (absolute points or offsets), `clamp` (none, clamp to the segment in time, scale both handles),
  `solver`, `before` and `after` (hold, linear extrapolation by the end slope, cycle, cycle with offset, mirror),
  `coincident` (keep first, keep last, jump) and `empty`.
- `Solver`: `Cardano { polish }`, `Bisection { steps }`, `Newton { steps, start }`. Each does a fixed amount of
  work and never exits early on the data.
- `solve::cbrt` (fixed cube root), `eval::floor`, `eval::round_half_up`, `eval::round_half_even`.

## Determinism

Only addition, subtraction, multiplication, division and square root are used, never a fused multiply add, and no
library transcendental function. Loops have fixed counts. The three real root case of the closed form uses the
triple angle identity and a fixed number of bisections instead of a cosine. Rust does not contract floating point
operations, so the same inputs give the same bits on Linux, macOS and Windows.

## Definitions and sources

- Cubic Bezier curves, the de Casteljau evaluation, the power basis, the convex hull and control polygon
  properties, and the Hermite to Bezier relation (handles at one third of the segment): G. Farin, Curves and
  Surfaces for Computer Aided Geometric Design, A Practical Guide, 5th edition, Morgan Kaufmann, 2002, the
  chapters on Bezier curves and on cubic Hermite interpolation.
- Cubic Hermite basis functions h00, h10, h01, h11: the same text, and the standard cubic Hermite spline
  definition found in numerical analysis texts.
- Closed form roots of a cubic (Cardano 1545, Ars Magna, chapter 11; the depressed cubic, discriminant and the
  trigonometric form for three real roots): W. H. Press et al., Numerical Recipes, 3rd edition, Cambridge
  University Press, 2007, section 5.6. The trigonometric form is replaced here by the triple angle identity
  `cos(3a) = 4 cos(a)^3 - 3 cos(a)`, that is `4y^3 - 3y = cos(3a)` for `y = cos(a)`, solved by bisection.
- Newton iteration for a cube root and for the curve parameter, and bisection: Press et al. 2007, section 9.4 (the
  Newton-Raphson method) and section 9.2 (bisection).
- Keyframe interpolation (constant, linear, Bezier), extrapolation (constant, linear) and the cycle, cycle with
  repeat offset and mirrored repeat behaviours: the open animation software manual pages on F-curves, their
  interpolation and extrapolation, and the cycles modifier, in the documentation of Blender (docs.blender.org,
  Animation and Rigging, Animation Editors, Graph Editor, F-Curves).
- Monotone cubic segments: F. N. Fritsch and R. E. Carlson, Monotone piecewise cubic interpolation, SIAM Journal
  on Numerical Analysis 17(2), 1980, and the control polygon condition in Farin 2002.

The three handle clamping rules, the side rules at key times, the coincident time rules and the frame snapping
rules are choices of this crate, stated in the documentation of `Config`; the crate does not claim they match any
particular program.

## Behaviour notes

- Inside the keyed range a time that equals a key time returns that key's value, except that `hold_side = Left`
  returns the held value of the previous key when the segment before the key is a hold segment.
- Keys with equal times: `KeepFirst` and `KeepLast` drop the others in input order; `Jump` keeps all, and the value
  at the shared time is the first (`jump_side = Left`) or the last (`Right`) of them.
- Cycle rules use the time range from the first to the last key. A range of zero length holds the end value.
- Linear extrapolation uses the slope of the end segment: zero for a hold segment, the chord for a linear one,
  the Hermite out slope of the first key or in slope of the last key, or the slope of the Bezier handle (the chord
  when the handle has no time extent).
- A non finite time gives NaN in every component. An empty list gives `Config::empty`. A single key is constant.
- Snapping uses `t' = round(t * frames_per_unit) / frames_per_unit`. A `frames_per_unit` that is not finite and
  positive disables snapping.
- A Newton solver with a fixed start can step onto a point where the time curve is stationary and stay there;
  that is the stated behaviour of a fixed step solver, and the closed form and bisection do not have it.
- The closed form root is checked: when its time residual exceeds 1e-12 (the cubic coefficient is then a
  rounding error away from zero and the formula has lost its digits), bisection with 64 halvings replaces it
  before the Newton polish.
- For a Bezier segment whose handle times cross, the time curve is not monotone and has more than one root. The
  closed form takes the root inside `[0, 1]` that is smallest; bisection and Newton return a root that their
  fixed iteration reaches.

## Tests

The tests in `tests/` use made up data. Expectations are written as exact values worked by hand in rational
arithmetic where the kind allows (quarter, half and three quarter times of two key segments for each kind), with
independent slow references (Bernstein form with 200 bisections, Hermite to Bezier conversion, `powf`) for the rest,
and a pinned digest over the whole configuration space.
