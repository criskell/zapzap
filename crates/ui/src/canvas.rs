pub struct Canvas<'a> {
    pub px: &'a mut [u32],
    pub w: i32,
    pub h: i32,
    pub y0: i32,
    pub rows: i32,
}

impl Canvas<'_> {
    pub(crate) fn blend(&mut self, x: i32, y: i32, color: u32, alpha: u8) {
        let y = y - self.y0;
        if x < 0 || y < 0 || x >= self.w || y >= self.rows {
            return;
        }
        let dst = &mut self.px[(y * self.w + x) as usize];
        let a = alpha as u32;
        let mix = |shift: u32| {
            let s = (color >> shift) & 0xff;
            let d = (*dst >> shift) & 0xff;
            (s * a + d * (255 - a)) / 255
        };
        *dst = (mix(16) << 16) | (mix(8) << 8) | mix(0);
    }

    /// Blends `color` with a separate coverage for each colour channel (LCD text, RGB stripe order).
    pub(crate) fn blend_lcd(&mut self, x: i32, y: i32, color: u32, coverage: [u8; 3]) {
        let y = y - self.y0;
        if x < 0 || y < 0 || x >= self.w || y >= self.rows {
            return;
        }
        let dst = &mut self.px[(y * self.w + x) as usize];
        let mix = |shift: u32, alpha: u8| {
            let s = (color >> shift) & 0xff;
            let d = (*dst >> shift) & 0xff;
            (s * alpha as u32 + d * (255 - alpha as u32)) / 255
        };
        *dst = (mix(16, coverage[0]) << 16) | (mix(8, coverage[1]) << 8) | mix(0, coverage[2]);
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u32) {
        let x0 = x.max(0);
        let x1 = (x + w).min(self.w);
        for row in y.max(self.y0)..(y + h).min(self.y0 + self.rows) {
            let start = ((row - self.y0) * self.w + x0) as usize;
            let end = ((row - self.y0) * self.w + x1) as usize;
            if start < end {
                self.px[start..end].fill(color);
            }
        }
    }

    /// Rectangle with rounded corners, edges smoothed by coverage.
    pub fn fill_round_rect(&mut self, x: i32, y: i32, w: i32, h: i32, r: i32, color: u32) {
        let r = r.min(w / 2).min(h / 2);
        self.fill_rect(x + r, y, w - 2 * r, h, color);
        self.fill_rect(x, y + r, r, h - 2 * r, color);
        self.fill_rect(x + w - r, y + r, r, h - 2 * r, color);
        let (top, bottom) = (self.y0, self.y0 + self.rows);
        let in_band = |row: i32| row >= top && row < bottom;
        for dy in (0..r).filter(|dy| in_band(y + r - 1 - dy) || in_band(y + h - r + dy)) {
            for dx in 0..r {
                let alpha = corner_coverage(dx, dy, r);
                if alpha != 0 {
                    for (px, py) in [(x + r - 1 - dx, y + r - 1 - dy), (x + w - r + dx, y + r - 1 - dy), (x + r - 1 - dx, y + h - r + dy), (x + w - r + dx, y + h - r + dy)] {
                        self.blend(px, py, color, alpha);
                    }
                }
            }
        }
    }

    /// Circle centred on pixel (`cx`, `cy`), edge smoothed by coverage.
    pub fn fill_circle(&mut self, cx: i32, cy: i32, r: i32, color: u32) {
        for dy in (-r - 1).max(self.y0 - cy)..=(r + 1).min(self.y0 + self.rows - 1 - cy) {
            for dx in -r - 1..=r + 1 {
                let alpha = circle_coverage(dx, dy, r);
                if alpha != 0 {
                    self.blend(cx + dx, cy + dy, color, alpha);
                }
            }
        }
    }
}

/// How much of the pixel at (`dx`, `dy`) from a circle's centre pixel lies inside radius `r`.
/// Integer maths in half pixels: full inside `2r-1`, empty outside `2r+1`, linear in between.
fn circle_coverage(dx: i32, dy: i32, r: i32) -> u8 {
    edge_coverage(4 * (dx * dx + dy * dy), r)
}

/// Same for a corner arc, where (`dx`, `dy`) counts pixels away from the arc's centre corner.
fn corner_coverage(dx: i32, dy: i32, r: i32) -> u8 {
    edge_coverage((2 * dx + 1) * (2 * dx + 1) + (2 * dy + 1) * (2 * dy + 1), r)
}

fn edge_coverage(distance_squared: i32, r: i32) -> u8 {
    let inner = (2 * r - 1).max(0).pow(2);
    let outer = (2 * r + 1).pow(2);
    if distance_squared <= inner {
        255
    } else if distance_squared >= outer {
        0
    } else {
        (255 * (outer - distance_squared) / (outer - inner)) as u8
    }
}
