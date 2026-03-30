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

    pub(super) fn pack<T>(data: &mut [T], size_fn: impl Fn(&T) -> [usize; 2]) -> Self {
        // Sort by height descending.
        data.sort_by(|a, b| size_fn(b)[1].cmp(&size_fn(a)[1]));

        let sizes: Vec<[usize; 2]> = data.iter().map(|d| size_fn(d)).collect();

        // Estimate atlas width from total area, but ensure it's at least as wide
        // as the widest single rect to prevent underflow.
        let total_area: usize = sizes
            .iter()
            .map(|s| (s[0] + Self::PADDING) * (s[1] + Self::PADDING))
            .sum();

        let max_width = sizes.iter().map(|s| s[0]).max().unwrap_or(0);
        let desired_width = ((total_area as f64).sqrt().ceil() as usize).max(max_width);

        let mut x_cursor = 0;
        let mut y_cursor = 0;
        let mut next_y_pos = 0;
        let mut actual_width = 0;

        let packed_rects = sizes
            .into_iter()
            .map(|size| {
                let w = size[0];
                let h = size[1];

                if x_cursor + w > desired_width {
                    x_cursor = 0;
                    y_cursor = next_y_pos;
                }

                let result = PackedRect {
                    x: x_cursor,
                    y: y_cursor,
                };

                x_cursor += w + Self::PADDING;
                actual_width = actual_width.max(x_cursor - Self::PADDING);
                next_y_pos = next_y_pos.max(y_cursor + h + Self::PADDING);

                result
            })
            .collect();

        Self {
            width: actual_width,
            height: next_y_pos.saturating_sub(Self::PADDING),
            rects: packed_rects,
        }
    }
}
