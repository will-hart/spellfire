//! Tools to generate "realistic" maps using simplex/perlin noise maps

use bevy::math::FloatExt;
use fastnoise_lite::{FastNoiseLite, FractalType, NoiseType};

use crate::wildfire::TerrainType;

pub struct NoiseMap {
    noise: FastNoiseLite,
}

impl NoiseMap {
    /// Creates a new noise map
    pub fn new() -> Self {
        let mut noise = FastNoiseLite::new();
        noise.set_noise_type(Some(NoiseType::Perlin));
        noise.set_fractal_type(Some(FractalType::FBm));
        noise.set_fractal_octaves(Some(3));
        noise.set_fractal_lacunarity(Some(1.9));

        Self { noise }
    }

    /// Samples the noise map and returns a terrain type and fuel load
    pub fn sample(&self, x: u32, y: u32) -> (TerrainType, u8) {
        // from experimentation, the noise values range roughly -0.5 -> 0.5
        let noise = self.noise.get_noise_2d(x as f32, y as f32);
        let noise = ((noise + 0.45) / 0.9).remap(0.0, 0.9, 0.0, 1.0);

        if noise < 0.15 {
            (TerrainType::Dirt, 0)
        } else if noise < 0.35 {
            let fuel_load = 18.0 * (noise - 0.15) / 0.2;
            (TerrainType::Tree, fuel_load.clamp(1.0, 18.0) as u8)
        } else if noise < 0.7 {
            let fuel_load = 12.0 * (noise - 0.35) / 0.35;
            (TerrainType::Grassland, fuel_load.clamp(2.0, 10.0) as u8)
        } else {
            let rock_and_stone = 10.0 * (noise - 0.7) / 0.3;
            (TerrainType::Stone, rock_and_stone.clamp(1.0, 10.0) as u8)
        }
    }
}
