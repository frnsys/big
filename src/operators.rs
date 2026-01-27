use egui::Vec2;
use guillotiere::{AtlasAllocator, size2};

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
    // Pack until they all fit
    let total_area: f32 = sizes.iter().map(|s| s.x * s.y).sum();
    let mut side = total_area.sqrt() as i32;

    loop {
        let mut allocator = AtlasAllocator::new(size2(side, side));
        let mut results = Vec::new();
        let mut success = true;

        for size in sizes {
            let alloc_size = size2(size.x.ceil() as i32, size.y.ceil() as i32);

            if let Some(allocation) = allocator.allocate(alloc_size) {
                let min = allocation.rectangle.min;
                results.push(Vec2::new(min.x as f32, min.y as f32));
            } else {
                success = false;
                break;
            }
        }

        if success {
            return results;
        }

        // Try again with more area
        side = (side as f32 * 1.1) as i32;
    }
}
