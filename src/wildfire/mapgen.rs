//! Tools to generate "realistic" maps using simplex/perlin noise maps

use bevy::prelude::*;
use fastnoise_lite::FastNoiseLite;
use rand::Rng;

use crate::wildfire::{TerrainCellState, TerrainType};

const NOISE_REDIST_FACTOR: f32 = 1.46;
const NOISE_SCALE: f32 = 0.5;

pub struct NoiseMap {
    noise: FastNoiseLite,
}

impl NoiseMap {
    /// Creates a new noise map
    pub fn new() -> Self {
        Self {
            noise: FastNoiseLite::new(),
        }
    }

    #[inline]
    fn noise(&self, x: f32, y: f32) -> f32 {
        let result = self.noise.get_noise_2d(x, y);
        result.remap(-0.5, 0.5, 0.0, 1.0).clamp(0.0, 1.0)
    }

    /// Returns a moisture level from 0-1. For now just aliases `noise`
    /// with a constant offset
    #[inline]
    pub fn moisture(&self, x: f32, y: f32) -> f32 {
        const MOISTURE_OFFSET: f32 = 17.5;
        self.noise(x + MOISTURE_OFFSET, y + MOISTURE_OFFSET)
    }

    /// Samples the noise map and returns a terrain type and fuel load
    pub fn sample(&self, x: usize, y: usize) -> (TerrainType, u8) {
        let x = x as f32;
        let y = y as f32;

        let noise_base = self.noise(NOISE_SCALE * x, NOISE_SCALE * y);
        let noise = 1.0 * noise_base
            + 0.5 * self.noise(NOISE_SCALE * 2.0 * x, NOISE_SCALE * 2.0 * y)
            + 0.25 * self.noise(NOISE_SCALE * 4.0 * x, NOISE_SCALE * 4.0 * y);
        let noise = (noise / (1.0 + 0.5 + 0.25)).powf(NOISE_REDIST_FACTOR);

        const DIRT: f32 = 0.03;
        const GRASS: f32 = 0.5;
        const TREE: f32 = 0.75;

        // note trees are placed in a separate pass
        if noise < DIRT {
            (TerrainType::Dirt, 0)
        } else if noise < GRASS {
            let fuel_load = (12.0 * (noise - DIRT) / (GRASS - DIRT)).clamp(1.0, 12.0) as u8;
            (TerrainType::Grassland, fuel_load)
        } else if noise < TREE {
            let fuel_load = (24.0 * (noise - GRASS) / (TREE - GRASS)).clamp(1.0, 24.0) as u8;
            (TerrainType::Tree, fuel_load)
        } else {
            let rock_and_stone = 10.0 * (noise - TREE) / (1.0 - TREE);
            (TerrainType::Stone, rock_and_stone.clamp(1.0, 10.0) as u8)
        }
    }
}

#[derive(Resource, Reflect, Debug)]
#[reflect(Resource)]
pub struct Map {
    pub data: Vec<Vec<TerrainCellState>>,
}

impl Map {
    pub fn new(size_x: usize, size_y: usize) -> Self {
        let noise_map = NoiseMap::new();
        let mut data = vec![vec![TerrainCellState::default(); size_x]; size_y];

        for y in 0..size_y {
            for x in 0..size_x {
                let (terrain, fuel) = noise_map.sample(x, y);
                data[y][x].terrain = terrain;
                data[y][x].fuel_load = fuel;

                match terrain {
                    TerrainType::Grassland | TerrainType::Tree => {
                        data[y][x].moisture = noise_map.moisture(x as f32, y as f32);
                    }
                    _ => {}
                }
            }
        }

        Self { data }
    }
}
