use glam::Vec2;

use crate::utils::Bitmap;

struct SdfGrids {
    inside: Vec<Vec2>,
    outside: Vec<Vec2>,
}

impl SdfGrids {
    const FORWARD_OFFSETS: [(i32, i32); 4] = [(-1, 0), (-1, -1), (0, -1), (1, -1)];
    const BACKWARD_OFFSETS: [(i32, i32); 4] = [(1, 0), (1, 1), (0, 1), (-1, 1)];

    pub fn forward_sweep(&mut self, w: i32, h: i32) {
        for y in 0..h {
            for x in 0..w {
                for &(dx, dy) in &SdfGrids::FORWARD_OFFSETS {
                    self.eval_neighbor(w, h, x, y, dx, dy);
                }
            }
        }
    }

    pub fn backward_sweep(&mut self, w: i32, h: i32) {
        for y in (0..h).rev() {
            for x in (0..w).rev() {
                for &(dx, dy) in &SdfGrids::BACKWARD_OFFSETS {
                    self.eval_neighbor(w, h, x, y, dx, dy);
                }
            }
        }
    }

    pub fn merge(self, bitmap: &Bitmap, radius: f32) -> Bitmap {
        let inv_radius = 1.0 / radius;
        let sdf_bitmap: Vec<u8> = self.inside
            .into_iter()
            .enumerate()
            .map(|(i, inside_pt)| {
                let is_inside = bitmap.data[i] > 127;
                let dist = if is_inside {
                    self.outside[i].length_squared().sqrt()
                } else {
                    inside_pt.length_squared().sqrt()
                };

                let normalized = (dist.min(radius) * inv_radius) * 0.5;
                let final_sdf = if is_inside { 0.5 + normalized } else { 0.5 - normalized };

                (final_sdf * 255.0).round() as u8
            })
            .collect();

        Bitmap {
            data: sdf_bitmap,
            width: bitmap.width,
            height: bitmap.height
        }
    }

    fn eval_neighbor(&mut self, w: i32, h: i32, x: i32, y: i32, dx: i32, dy: i32) {
        let nx = x + dx;
        let ny = y + dy;

        if nx >= 0 && nx < w && ny >= 0 && ny < h {
            let curr_idx = (y * w + x) as usize;
            let neigh_idx = (ny * w + nx) as usize;
            let offset_vec = Vec2::new(dx as f32, dy as f32);

            let cand_in = unsafe { *self.inside.get_unchecked(neigh_idx) } + offset_vec;
            if cand_in.length_squared() < self.inside[curr_idx].length_squared() {
                self.inside[curr_idx] = cand_in;
            }

            let cand_out = unsafe { *self.outside.get_unchecked(neigh_idx) } + offset_vec;
            if cand_out.length_squared() < self.outside[curr_idx].length_squared() {
                self.outside[curr_idx] = cand_out;
            }
        }
    }
}

/// Generates an signed-distance field (SDF) from a raw bitmap.
pub struct SdfGenerator;
impl SdfGenerator {
    /// Generates an SDF bitmap from a raw `bitmap` and `radius`. This does not modify the original `bitmap`
    pub fn generate(raw_bitmap: &Bitmap, radius: f32) -> Bitmap {
        let padding = radius as u32;
        let sdf_bitmap = SdfGenerator::pad_bitmap(raw_bitmap, padding);
        let mut grids = SdfGenerator::generate_grids(&sdf_bitmap);
    
        let w = sdf_bitmap.width as i32;
        let h = sdf_bitmap.height as i32;

        grids.forward_sweep(w, h);
        grids.backward_sweep(w, h);

        grids.merge(&sdf_bitmap, radius)
    }

    /// Pad the bitmap equally on all sides
    fn pad_bitmap(bitmap: &Bitmap, padding: u32) -> Bitmap {
        let padded_width = bitmap.width + (padding * 2);
        let padded_height = bitmap.height + (padding * 2);
        let total_pixels = (padded_width * padded_height) as usize;

        let mut padded = vec![0u8; total_pixels];
        for r in 0..bitmap.height {
            for c in 0..bitmap.width {
                let src_idx = r * bitmap.width + c;
                let dst_idx = (r + padding) * padded_width + (c + padding);
                padded[dst_idx as usize] = bitmap.data[src_idx as usize];
            }
        }

        Bitmap {
            data: padded,
            width: padded_width,
            height: padded_height
        }
    }

    /// Generate the inside/outside grids representing the coverage of the bitmap
    fn generate_grids(bitmap: &Bitmap) -> SdfGrids {
        let inf = Vec2::splat(9999.0);
        let pixels = (bitmap.width * bitmap.height) as usize;

        let mut inside = vec![inf; pixels];
        let mut outside = vec![inf; pixels];

        for i in 0..pixels {
            if bitmap.data[i] > 127 {
                inside[i] = Vec2::ZERO;
            } else {
                outside[i] = Vec2::ZERO;
            }
        }

        SdfGrids {
            inside, 
            outside,
        }
    }
}