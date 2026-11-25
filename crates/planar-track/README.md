# planar-track

A Rust library with no dependencies for direct intensity based tracking of a planar region through a sequence of grey level images. Every method in it is taken from the published literature cited below and is written here from the description. It has no unsafe code, no thread and no platform specific code.

## What it does

Given a template image and a region of it, the library finds, for each frame of a sequence, the planar motion that makes the warped frame agree with the template over the region. It does this by minimising the sum of squared grey level differences with the Lucas-Kanade family of methods, coarse to fine over a Gaussian pyramid.

The pieces, each in its own module:

- `image`: grey images of `f64`, the pixel centre convention, bilinear sampling, central difference gradients.
- `pyramid`: the Gaussian pyramid, with a configurable number of levels, scale factor and kernel size.
- `model`: the motion models (translation, translation and scale, similarity, affine, projective homography) and the pose type.
- `region`: the tracked region, from a rectangle, a convex quadrilateral or a mask.
- `edt`: the linear time Euclidean distance transform used for boundary weights.
- `align`: one pyramid level of alignment, with the four update rules, Levenberg-Marquardt damping and the illumination model.
- `ncc`: an optional coarse search by normalised cross correlation.
- `config`: every choice as a field of `Config`, and `ConfigSpace`, the enumeration of the discrete choices.
- `tracker`: frame to frame tracking forward and backward, constant velocity prediction, a report for every frame, and the correction functions.

## Conventions

Pixel centre. The sample `data[y * width + x]` lies at the point `(x, y)` with integer coordinates, so the first pixel centre is `(0, 0)` and an image covers `0 <= x <= width - 1`, `0 <= y <= height - 1`. Grey values are `f64`; the nominal range is 0 to 1, and the inlier threshold of the configuration is in those units.

Bilinear sampling. The value at `(x, y)` is the weighted mean of the four surrounding samples with weights `(1-fx)(1-fy)`, `fx(1-fy)`, `(1-fx)fy`, `fx fy`, where `fx` and `fy` are the fractional parts. `sample_clamped` replicates the edge samples outside the image; `sample_inside` refuses a point outside the covered domain. The aligner uses `sample_inside`: a template pixel whose warped position is outside the frame takes no part in a step and counts against the valid fraction.

Gradients. Central differences, `(I(x+1) - I(x-1)) / 2`, with the edge samples replicated. The template and the frames use the same rule.

Pyramid. The kernel of `taps = 2m + 1` taps is the binomial row of order `taps - 1` divided by `2^(taps - 1)`: three taps give `[1 2 1] / 4`, five give `[1 4 6 4 1] / 16`. It is the Gaussian of variance `(taps - 1) / 4` in the binomial approximation and needs no transcendental function. Level `l + 1` has `max(1, floor(size / scale + 1/2))` pixels per axis. Its pixel `i` is the bilinear sample of the blurred level `l` at `s (i + 1/2) - 1/2`, where `s` is the real size ratio. Each level keeps the affine map to level 0 coordinates. A level that would be smaller than `min_size` is not built.

Pose and parameters. A pose is a 3 by 3 matrix `H` with `x_frame = H x_template` in homogeneous coordinates, normalised to `h33 = 1`. `a.compose(b)` is the matrix product `a * b`, so `b` acts first. Every model is a linear subspace of `q = (tx, ty, a11, a12, a21, a22, g1, g2)`, which gives

```
[ 1 + a11   a12      tx ]
[ a21       1 + a22  ty ]
[ g1        g2       1  ]
```

and the parameters `p` of a model are mapped by a fixed matrix, `q = M p`:

| Model | Parameters | Relation to `q` |
| --- | --- | --- |
| translation | `tx, ty` | the rest zero |
| translation and scale | `tx, ty, s` | `a11 = a22 = s` |
| similarity | `tx, ty, a, b` | `a11 = a22 = a`, `a21 = b`, `a12 = -b` |
| affine | `tx, ty, a11, a12, a21, a22` | `g1 = g2 = 0` |
| homography | all eight | `p = q` |

All parameters are zero at the identity. A pose outside a model is projected onto it by least squares (`Pose::params`).

Level coordinates. A pose is stored in level 0 coordinates of the template and the frame. At a pyramid level it is conjugated by the level maps, `H_l = F_l^-1 H T_l`, where `T_l` and `F_l` take level coordinates to level 0 coordinates for the template and the frame.

## Alignment

On one level the residual of a template pixel `x` is `r(x) = I(W(x)) - ((1 + gain) T(x) + offset)`, and the cost is `sum w r^2` over the selected pixels with weights `w`. A step `s` solves `(A + lambda D) s = -b`, with `A = C^T W C`, `b = C^T W r`, where the rows of `C` are the pose columns of the rule and, when the illumination is solved jointly, `-T` and `-1`.

The update rule is a field of the configuration:

- Forward additive: the Jacobian of the warp at the current parameters times the frame gradient, the parameters updated by addition (Baker and Matthews, section 2.2).
- Forward compositional: the frame gradient times the Jacobian of the warp with respect to position times the Jacobian at the identity; the pose becomes `H * H(s)` (section 3.1).
- Inverse compositional: the template gradient times the Jacobian at the identity, computed once per level; the pose becomes `H * H(s)^-1` (section 3.2).
- Efficient second order minimisation: the mean of the forward compositional and the inverse compositional columns, the pose composed as in the forward compositional rule (Benhimane and Malis). The group exponential of that paper is replaced by the parameterised warp above, which agrees with it to first order.

Damping is `A + lambda I` (Levenberg) or `A + lambda diag(A)` (Marquardt). Lambda starts at `lambda_init` on every level, is multiplied by `lambda_up` after a rejected step and by `lambda_down` after an accepted one (never below `lambda_min`). The acceptance test is a strict fall of the weighted mean squared residual, a fall or tie, or a gain ratio of actual to predicted reduction at least a given value. Each level has its own limits (`LevelParams`, coarsest first, the last entry repeating): an iteration limit, a limit on consecutive rejections, a tolerance on the motion of the four region corners in one step, a tolerance on the relative fall of the cost, and a lambda ceiling. A rejected step that would move the corners by less than the step tolerance also ends the level as converged.

Illumination. The model is template side: the template is replaced by `(1 + gain) T + offset`. `Illumination::Joint` solves gain and offset in the same system as the pose. `Illumination::Alternate` sets them in closed form (weighted least squares for the current pose) at the start of every iteration and solves the pose alone. `Off` keeps both at zero. The estimates are reported for every frame; bilinear sampling lowers the contrast of a frame slightly, so the gain reads a little low.

Pixel selection and weights. Of the mask pixels of a level, the `min_fraction` with the largest template gradient magnitude are kept (ties go to the earlier pixel in scan order; at least one pixel is kept). With a boundary weight other than `Off`, each kept pixel gets a weight from its Euclidean distance `d` to the nearest pixel outside the mask, with `t = min(1, d / ramp)`: linear `t`, smoothstep `t^2 (3 - 2t)`, or quadratic `t^2`, never below `min_weight`.

Correlation search. When enabled, at the chosen level the integer translations of the starting pose within a window are scanned (`dy` outer, `dx` inner, increasing), each scored by the weighted correlation coefficient between the template and the warped frame over the selected pixels; the first best score above `min_score` is taken and the Levenberg-Marquardt iteration of that level starts from it.

## Tracking and correction

A `Tracker` holds a fixed template and region and a list of frames. Each frame has an optional pose with its source (`Known` or `Tracked`) and a `FrameResult`: the pose, the root mean square residual at the finest level, the inlier fraction, the valid fraction, the iterations, whether the finest level converged and why it stopped, the gain and offset, and the correlation score when the search ran.

- `add_frame` appends a frame.
- `set_pose` sets a known pose at a frame (the residual figures are measured for it).
- `track(from, to)` solves the frames after `from` up to `to`, forward or backward, starting from the pose of `from`. A frame that holds a known pose is kept and tracking continues from it. `track_forward` and `track_backward` run to the ends.
- `solve_frame` solves one frame from a given first guess.
- `predict` returns the first guess for the next frame: the neighbouring pose, or, with `Velocity::ConstantImageSpace`, that pose moved once more by the image space motion between the two frames before it, `P1 * P0^-1`.
- `clear_pose` forgets a frame's pose.

To repair a sequence that lost the region at some frame, set the correct pose at that frame with `set_pose` and call `track` again from it; the frames after it are solved afresh. The template is not updated during tracking.

## Configuration

`Config` has a field for every choice described above, each with a documented default: a four level pyramid of scale 2 with a five tap kernel, an affine model, the inverse compositional rule, no illumination model, Marquardt damping with `lambda_init = 1e-4`, up factor 10 and down factor 0.1, half of the template pixels, no boundary weight, no search, no motion prediction. `Config::validate` checks every range. `ConfigSpace` enumerates the product of the discrete choices (model, rule, illumination mode, damping form, acceptance test, boundary weight, motion prediction, search switch); `ConfigSpace::dimensions` names them, `size` counts them and `get` and `iter` give each configuration.

## Arithmetic

Only IEEE addition, subtraction, multiplication, division, square root, floor and ceiling are used, in a fixed order, with no fused multiply add and no library transcendental function. The result of a run is the same on Linux, macOS and Windows.

## Tests

The tests render a deterministic textured plane through known homographies and track it. They check, for each of the five models and each of the four rules, that the corners of the region come out within 0.15 pixels of the truth; single steps by exact equality against values worked out by hand; the pyramid, the gradient and the bilinear sampler on small fixed images; the distance transform against brute force; brightness changes absorbed by the illumination model; a large motion that needs the correlation search; and a lost track repaired by the correction functions.

## Limits

The template is fixed for the whole sequence. There is no robust loss, no occlusion handling beyond the pixel weights and the inlier report, and no sub-pixel correlation search; the integer search only gives the Levenberg-Marquardt iteration a start.

## References

- B. D. Lucas and T. Kanade, "An Iterative Image Registration Technique with an Application to Stereo Vision", IJCAI 1981.
- S. Baker and I. Matthews, "Lucas-Kanade 20 Years On: A Unifying Framework", International Journal of Computer Vision 56(3), 2004: section 2.1 and 2.2 (the warp, the forward additive algorithm), section 3.1 (forward compositional), section 3.2 (inverse compositional), sections 3.1.3 and 3.2.3 (requirements on the warps), section 4.1 (Gauss-Newton), section 4.5 (Levenberg-Marquardt).
- S. Benhimane and E. Malis, "Real-time image-based tracking of planes using Efficient Second-order Minimization", IEEE/RSJ International Conference on Intelligent Robots and Systems, 2004.
- K. Levenberg, "A Method for the Solution of Certain Non-Linear Problems in Least Squares", Quarterly of Applied Mathematics 2(2), 1944; D. W. Marquardt, "An Algorithm for Least-Squares Estimation of Nonlinear Parameters", SIAM Journal on Applied Mathematics 11(2), 1963.
- P. J. Burt and E. H. Adelson, "The Laplacian Pyramid as a Compact Image Code", IEEE Transactions on Communications 31(4), 1983 (the Gaussian pyramid).
- J.-Y. Bouguet, "Pyramidal Implementation of the Affine Lucas Kanade Feature Tracker", Intel Corporation, 2001 (coarse to fine tracking).
- G. D. Hager and P. N. Belhumeur, "Efficient Region Tracking With Parametric Models of Geometry and Illumination", IEEE Transactions on Pattern Analysis and Machine Intelligence 20(10), 1998 (the linear illumination model).
- P. F. Felzenszwalb and D. P. Huttenlocher, "Distance Transforms of Sampled Functions", Theory of Computing 8(19), 2012 (the linear time Euclidean distance transform).
- J. P. Lewis, "Fast Normalized Cross-Correlation", Vision Interface 1995; R. Brunelli, "Template Matching Techniques in Computer Vision: Theory and Practice", Wiley, 2009 (normalised cross correlation).
