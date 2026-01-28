use binpack2d::{BinType, Dimension, bin_new};
use egui::Vec2;

/// Return scaling factors to make a set of sizes more similar.
pub fn normalize_sizes(sizes: &[Vec2]) -> Vec<f32> {
    // Mean size
    let count = sizes.len() as f32;
    let mean_dim = sizes.iter().fold(Vec2::ZERO, |acc, v| acc + *v) / count;
    let mean_area = mean_dim.x * mean_dim.y;

    sizes
        .iter()
        .map(|size| {
            let area = size.x * size.y;
            let scale = (mean_area / area).sqrt();
            let adjusted_scale = 1.0 + (scale - 1.0) * 0.8; // 80% toward mean
            adjusted_scale
        })
        .collect()
}

pub fn pack_rects(sizes: &[Vec2]) -> Vec<Vec2> {
    let total_area: f32 = sizes.iter().map(|s| s.x * s.y).sum();
    let mut side = total_area.sqrt() as i32;

    let items: Vec<_> = sizes
        .iter()
        .enumerate()
        .map(|(i, size)| {
            Dimension::with_id(i as isize, size.x.ceil() as i32, size.y.ceil() as i32, 5)
        })
        .collect();

    // Pack until they all fit
    loop {
        // let mut bin = bin_new(BinType::Guillotine, side, side);
        let mut bin = bin_new(BinType::MaxRects, side, side);

        let (mut inserted, rejected) = bin.insert_list(&items);

        if rejected.is_empty() {
            inserted.sort_by_key(|rect| rect.id());
            return inserted
                .into_iter()
                .map(|rect| Vec2::new(rect.x() as f32, rect.y() as f32))
                .collect();
        }

        // Try again with more area
        side = (side as f32 * 1.1) as i32;
    }
}
