#[derive(Debug, Clone, Copy)]
struct Rect {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

#[derive(Debug)]
pub(super) struct PackedRect {
    pub(super) x: usize,
    pub(super) y: usize,
    pub(super) index: usize,
}

struct Packer {
    width: usize,
    height: usize,
    free_rects: Vec<Rect>,
}
impl Packer {
    fn new(width: usize, height: usize) -> Self {
        Self {
            free_rects: vec![Rect {
                x: 0,
                y: 0,
                width,
                height,
            }],
            width,
            height,
        }
    }

    // Try to insert a rectangle, returns position if successful
    fn insert(&mut self, width: usize, height: usize) -> Option<(usize, usize)> {
        // Find the free rect with the smallest leftover area (best area fit)
        let best = self
            .free_rects
            .iter()
            .enumerate()
            .filter(|(_, r)| r.width >= width && r.height >= height)
            .min_by_key(|(_, r)| r.width * r.height - width * height);

        let (idx, rect) = best?;
        let (x, y) = (rect.x, rect.y);
        let (rx, ry, rw, rh) = (rect.x, rect.y, rect.width, rect.height);

        self.free_rects.remove(idx);

        // Split the used rect into two free rects (guillotine split, longer axis)
        let right_w = rw - width;
        let top_h = rh - height;

        if right_w > 0 || top_h > 0 {
            // Split along the longer remaining axis
            if right_w * rh > top_h * (rw - right_w + width) {
                // Horizontal split: right strip gets full height, top strip gets remaining width
                if right_w > 0 {
                    self.free_rects.push(Rect {
                        x: rx + width,
                        y: ry,
                        width: right_w,
                        height: rh,
                    });
                }
                if top_h > 0 {
                    self.free_rects.push(Rect {
                        x: rx,
                        y: ry + height,
                        width,
                        height: top_h,
                    });
                }
            } else {
                if top_h > 0 {
                    self.free_rects.push(Rect {
                        x: rx,
                        y: ry + height,
                        width: rw,
                        height: top_h,
                    });
                }
                if right_w > 0 {
                    self.free_rects.push(Rect {
                        x: rx + width,
                        y: ry,
                        width: right_w,
                        height,
                    });
                }
            }
        }

        Some((x, y))
    }
}

/// Pack rectangles into the smallest container.
/// Returns the packed positions and the container size (width, height).
/// Input: slice of (width, height) pairs.
pub(super) fn pack_rects(rects: &[(usize, usize)]) -> (Vec<PackedRect>, usize, usize) {
    if rects.is_empty() {
        return (vec![], 0, 0);
    }

    // Sort by area descending for better packing (work on indices)
    let mut indices: Vec<usize> = (0..rects.len()).collect();
    indices.sort_unstable_by(|&a, &b| {
        let area_a = rects[a].0 * rects[a].1;
        let area_b = rects[b].0 * rects[b].1;
        area_b.cmp(&area_a)
    });

    // Total area gives a lower bound on container size
    let total_area: usize = rects.iter().map(|(w, h)| w * h).sum();
    let max_side = rects.iter().map(|(w, h)| *(w.max(h))).max().unwrap_or(1);

    // Binary search on container width, height = ceil(total_area / width)
    // Start with a square root estimate and search around it
    let mut lo = max_side;
    let mut hi = rects
        .iter()
        .map(|(w, _)| w)
        .sum::<usize>()
        .max(rects.iter().map(|(_, h)| h).sum::<usize>());

    let try_pack = |width: usize| -> Option<(Vec<PackedRect>, usize)> {
        let height = ((total_area as f64 / width as f64).ceil() as usize)
            .max(rects.iter().map(|(_, h)| *h).max().unwrap_or(1));

        let mut packer = Packer::new(width, height * 2); // give extra vertical room
        let mut packed = Vec::with_capacity(rects.len());
        let mut max_y = 0;

        for &i in &indices {
            let (w, h) = rects[i];
            if let Some((x, y)) = packer.insert(w, h) {
                max_y = max_y.max(y + h);
                packed.push(PackedRect { index: i, x, y });
            } else {
                return None; // doesn't fit
            }
        }

        Some((packed, max_y))
    };

    // Binary search for minimum width that allows all rects to fit
    let mut best: Option<(Vec<PackedRect>, usize, usize)> = None;

    while lo <= hi {
        let mid = (lo + hi) / 2;

        if let Some((packed, height)) = try_pack(mid) {
            best = Some((packed, mid, height));
            hi = mid - 1;
        } else {
            lo = mid + 1;
        }
    }

    best.unwrap_or_else(|| {
        // Fallback: stack everything vertically
        let width = rects.iter().map(|(w, _)| *w).max().unwrap_or(1);
        let mut y = 0;
        let packed = indices
            .iter()
            .map(|&i| {
                let (_, h) = rects[i];
                let r = PackedRect { index: i, x: 0, y };
                y += h;
                r
            })
            .collect();
        (packed, width, y)
    })
}
