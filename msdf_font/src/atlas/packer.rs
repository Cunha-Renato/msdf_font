#[derive(Debug)]
pub(super) struct PackedRect {
    pub(super) x: usize,
    pub(super) y: usize,
    pub(super) index: usize,
}

pub(super) struct Packer {
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) rects: Vec<PackedRect>,
}
impl Packer {
    // Padding in px for x and y;
    const PADDING: usize = 2;

    pub(super) fn pack(rects: Vec<(usize, usize)>) -> Self {
        let mut total_area = 0;

        let mut rects_indexed = rects
            .into_iter()
            .enumerate()
            .map(|(i, (w, h))| {
                total_area += w * h;

                (i, w, h)
            })
            .collect::<Vec<_>>();

        // Sort by area.
        rects_indexed.sort_by(|a, b| (b.1 * b.2).cmp(&(a.1 * a.2)));

        let width = (total_area as f64).sqrt().ceil() as usize;

        let mut x_cursor = 0;
        let mut y_cursor = 0;
        let mut next_y_pos = 0;
        let packed_rects = rects_indexed
            .into_iter()
            .map(|(i, w, h)| {
                if x_cursor + w > width {
                    x_cursor = 0;
                    y_cursor = next_y_pos;
                }

                let result = PackedRect {
                    x: x_cursor,
                    y: y_cursor,
                    index: i,
                };

                x_cursor += w + Self::PADDING;
                next_y_pos = next_y_pos.max(y_cursor + h + Self::PADDING);

                result
            })
            .collect::<Vec<_>>();

        Self {
            width,
            height: next_y_pos,
            rects: packed_rects,
        }
    }
}
