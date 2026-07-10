use ndarray::{Array2, ShapeError};
use ttf_parser::OutlineBuilder;

use crate::collect::bezier::get_quadratic_approximation_of_cubic;

/// Quadratic Bézier curve builder
///
/// Notes:
/// - Cubic curves will be converted into quadratic curves in [PathBuilder::curve_to]
/// - Each result point is 3-dimensional, with the 3rd dimension set to zero
/// - Subpaths are seperated by `NAN_POINT` i.e. `(nan, nan, nan)`
pub struct PathBuilder {
    scale: f32,

    /// Final points, get it using [PathBuilder::build]
    points: Vec<[f32; 3]>,

    /// The start point of current subpath, used for [PathBuilder::close]
    subpath_start: Option<[f32; 2]>,
    /// The last point, used for calculating subsequent curves
    last_point: Option<[f32; 2]>,
}

impl PathBuilder {
    pub fn new(scale: f32) -> Self {
        Self {
            scale,
            points: Vec::new(),
            subpath_start: None,
            last_point: None,
        }
    }

    #[inline]
    fn push(&mut self, x: f32, y: f32) {
        self.points.push([x * self.scale, y * self.scale, 0.0]);
    }

    pub fn build_array2(self) -> Result<Array2<f32>, ShapeError> {
        Array2::from_shape_vec(
            (self.points.len(), 3),
            self.points.into_iter().flatten().collect(),
        )
    }
}

impl OutlineBuilder for PathBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        if self.last_point.is_some() {
            self.points.push([f32::NAN, f32::NAN, f32::NAN]);
            self.push(x, y);
        } else {
            self.push(x, y);
        }

        self.subpath_start = Some([x, y]);
        self.last_point = Some([x, y]);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        if let Some(last) = self.last_point {
            let cx = (last[0] + x) / 2.0;
            let cy = (last[1] + y) / 2.0;

            self.push(cx, cy);
            self.push(x, y);

            self.last_point = Some([x, y]);
        }
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.push(x1, y1);
        self.push(x, y);

        self.last_point = Some([x, y]);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        if let Some(last) = self.last_point {
            let h0 = [x1, y1];
            let h1 = [x2, y2];
            let anchor = [x, y];

            let is_close = |p1: [f32; 2], p2: [f32; 2]| -> bool {
                (p1[0] - p2[0]).abs() < 1e-5 && (p1[1] - p2[1]).abs() < 1e-5
            };

            if is_close(last, h0) {
                self.quad_to(x2, y2, x, y);
                return;
            }
            if is_close(h0, h1) {
                self.quad_to(x1, y1, x, y);
                return;
            }
            if is_close(h1, anchor) {
                self.quad_to(x1, y1, x2, y2);
                return;
            }

            let quad_approx = get_quadratic_approximation_of_cubic(last, h0, h1, anchor);

            self.push(quad_approx[1][0], quad_approx[1][1]);
            self.push(quad_approx[2][0], quad_approx[2][1]);
            self.push(quad_approx[3][0], quad_approx[3][1]);
            self.push(quad_approx[4][0], quad_approx[4][1]);

            self.last_point = Some([x, y]);
        }
    }

    #[allow(clippy::collapsible_if)]
    #[rustfmt::skip]
    fn close(&mut self) {
        if let (Some(start), Some(last)) = (self.subpath_start, self.last_point) {
            if (last[0] - start[0]).abs() > f32::EPSILON || (last[1] - start[1]).abs() > f32::EPSILON
            {
                self.line_to(start[0], start[1]);
            }
        }
    }
}
