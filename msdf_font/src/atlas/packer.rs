#[derive(Debug)]
pub(super) struct PackedRect {
    pub(super) x: usize,
    pub(super) y: usize,
}

pub(super) struct Packer {
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) rects: Vec<PackedRect>,
}
impl Packer {
    // Padding in px for x and y;
    const PADDING: usize = 1;

    pub(super) fn pack<T>(data: &mut [T], size_fn: impl Fn(&T) -> (usize, usize)) -> Self {
        let mut total_area = 0;

        // Sort by area.
        data.sort_by(|a, b| {
            let size_a = size_fn(a);
            let size_b = size_fn(b);

            (size_b.0 * size_b.1).cmp(&(size_a.0 * size_a.1))
        });

        // Indexing the rects.
        let rects_indexed = data
            .iter()
            .map(|data| {
                let size = size_fn(data);
                total_area += size.0 * size.1;

                (size.0, size.1)
            })
            .collect::<Vec<_>>();

        let width = (total_area as f64).sqrt().ceil() as usize;

        let mut x_cursor = 0;
        let mut y_cursor = 0;
        let mut next_y_pos = 0;
        let packed_rects = rects_indexed
            .into_iter()
            .map(|(w, h)| {
                if x_cursor + w > width {
                    x_cursor = 0;
                    y_cursor = next_y_pos;
                }

                let result = PackedRect {
                    x: x_cursor,
                    y: y_cursor,
                };

                x_cursor += w + Self::PADDING;
                next_y_pos = next_y_pos.max(y_cursor + h + Self::PADDING);

                result
            })
            .collect::<Vec<_>>();

        Self {
            width,
            height: next_y_pos.saturating_sub(Self::PADDING),
            rects: packed_rects,
        }
    }
}
