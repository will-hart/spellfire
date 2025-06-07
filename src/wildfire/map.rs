//! Tools to generate "realistic" maps using simplex/perlin noise maps

use std::time::Duration;

use bevy::{
    input::common_conditions::input_just_pressed, prelude::*, time::common_conditions::on_timer,
};
use fastnoise_lite::FastNoiseLite;
use rand::Rng;

use crate::{
    Pause,
    screens::{BuildingMode, BuildingType, EndlessMode, OnRedrawToolbar, RequiresCityHall, Screen},
    wildfire::{OnSpawnMap, SpawnedMap, TerrainCell, TerrainCellState, TerrainType, WindDirection},
};

/// the amount of cells in the neighbourhood
const NEIGHBOURHOOD_SIZE: usize = 8;

const NEIGHBOUR_COORDINATES: [IVec2; NEIGHBOURHOOD_SIZE] = [
    // Left
    IVec2::new(-1, 0),
    // Top Left
    IVec2::new(-1, 1),
    // Top
    IVec2::new(0, 1),
    // Top Right
    IVec2::new(1, 1),
    // Right
    IVec2::new(1, 0),
    // Bottom Right
    IVec2::new(1, -1),
    // Bottom
    IVec2::new(0, -1),
    // Bottom Left
    IVec2::new(-1, -1),
];

const NEIGHBOUR_VECTOR: [Vec2; NEIGHBOURHOOD_SIZE] = [
    // Left
    Vec2::new(-1., 0.),
    // Top Left
    Vec2::new(-1., 1.),
    // Top
    Vec2::new(0., 1.),
    // Top Right
    Vec2::new(1., 1.),
    // Right
    Vec2::new(1., 0.),
    // Bottom Right
    Vec2::new(1., -1.),
    // Bottom
    Vec2::new(0., -1.),
    // Bottom Left
    Vec2::new(-1., -1.),
];

const NOISE_REDIST_FACTOR: f32 = 1.46;
const NOISE_SCALE: f32 = 0.5;

pub(super) fn plugin(app: &mut App) {
    app.register_type::<GameMap>();

    app.add_systems(
        Update,
        update_map.run_if(
            on_timer(Duration::from_millis(100))
                .and(in_state(Pause(false)))
                .and(resource_exists::<GameMap>),
        ),
    );

    app.add_systems(
        Update,
        update_sprites.run_if(in_state(Pause(false)).and(resource_exists::<GameMap>)),
    );

    app.add_systems(
        Update,
        redraw_map.run_if(
            in_state(Screen::Gameplay)
                .and(in_state(Pause(false)))
                .and(resource_exists::<EndlessMode>)
                .and(input_just_pressed(KeyCode::KeyR)),
        ),
    );
}

fn update_map(mut map: ResMut<GameMap>, wind: Res<WindDirection>) {
    map.update(wind.0);
}

fn update_sprites(mut map: ResMut<GameMap>, mut sprites: Query<&mut Sprite, With<TerrainCell>>) {
    for y in 0..map.size_y {
        for x in 0..map.size_x {
            let cell = &mut map.data[y][x];
            if !cell.dirty {
                continue;
            }

            cell.dirty = false;

            let Some(entity) = cell.sprite_entity else {
                continue;
            };

            if let Ok(mut sprite) = sprites.get_mut(entity) {
                sprite.color = cell.colour();
            }
        }
    }
}

/// TODO: in theory here we could redraw without respawning the sprites
fn redraw_map(
    mut commands: Commands,
    mut mode: ResMut<BuildingMode>,
    spawned_maps: Query<Entity, With<SpawnedMap>>,
    buildings: Query<Entity, With<BuildingType>>,
) {
    commands.init_resource::<RequiresCityHall>();

    for map in spawned_maps {
        commands.entity(map).despawn();
    }

    for building in buildings {
        commands.entity(building).despawn();
    }

    let mut rng = rand::thread_rng();
    commands.trigger(OnSpawnMap::new(rng.r#gen()));
    *mode = BuildingMode::PlaceCityHall;
    commands.trigger(OnRedrawToolbar);
}

pub struct NoiseMap {
    noise: FastNoiseLite,
}

/// Seeds for mapgen that are "known good"
pub const GOOD_SEEDS: [i32; 7] = [
    670947188,
    -787500401,
    1337,
    618039333,
    -1354068758,
    1566845181,
    -63050108,
];

impl NoiseMap {
    /// Creates a new noise map
    pub fn new(seed: i32) -> Self {
        Self {
            noise: FastNoiseLite::with_seed(seed),
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

/// Contains information about the map that the game is being played on.
/// This is stored in a 2d Vec in the `data` field
#[derive(Resource, Reflect, Debug)]
#[reflect(Resource)]
pub struct GameMap {
    pub size_x: usize,
    pub size_y: usize,
    pub sprite_size: f32,
    pub data: Vec<Vec<TerrainCellState>>,
}

impl GameMap {
    pub fn new(seed: i32, sprite_size: f32, size_x: usize, size_y: usize) -> Self {
        let noise_map = NoiseMap::new(seed);
        let mut data = vec![vec![TerrainCellState::default(); size_x]; size_y];

        for (y, row) in data.iter_mut().enumerate().take(size_y) {
            for (x, cell) in row.iter_mut().enumerate().take(size_y) {
                let (terrain, fuel) = noise_map.sample(x, y);
                cell.terrain = terrain;
                cell.fuel_load = fuel;

                match terrain {
                    TerrainType::Grassland | TerrainType::Tree => {
                        cell.moisture = noise_map.moisture(x as f32, y as f32);
                    }
                    _ => {}
                }
            }
        }

        Self {
            data,
            size_x,
            size_y,
            sprite_size,
        }
    }

    /// Gets coordinates of valid cells within a given range of a point
    pub fn cells_within_range(&self, center: IVec2, range: i32) -> impl Iterator<Item = IVec2> {
        ((center.y - range).max(0)..=(center.y + range).max(0)).flat_map(move |y| {
            ((center.x - range).max(0)..=(center.x + range).max(0)).filter_map(move |x| {
                let v = IVec2::new(x, y);

                if v.distance_squared(center) > range * range
                    || x as usize >= self.size_x
                    || y as usize >= self.size_y
                {
                    None
                } else {
                    Some(v)
                }
            })
        })
    }

    /// Checks whether the cell at the given tile coords is on fire
    pub fn is_on_fire(&self, loc: IVec2) -> bool {
        if let Some(cell) = self.get(loc) {
            matches!(cell.terrain, TerrainType::Fire)
        } else {
            false
        }
    }

    /// Returns whether the given coordinates are "valid" (i.e. on the map)
    pub fn is_valid_coords(&self, coords: IVec2) -> bool {
        if coords.x < 0
            || coords.x as usize >= self.size_x
            || coords.y < 0
            || coords.y as usize >= self.size_y
        {
            return false;
        }

        true
    }

    /// Gets an immutable ref to the cell at the given location, if there is one
    pub fn get(&self, loc: IVec2) -> Option<&TerrainCellState> {
        if loc.x < 0 || loc.x >= self.size_x as i32 || loc.y < 0 || loc.y >= self.size_y as i32 {
            return None;
        }

        Some(&self.data[loc.y as usize][loc.x as usize])
    }

    /// Gets a mutable ref to the cell at the given location, if there is one
    pub fn get_mut(&mut self, loc: IVec2) -> Option<&mut TerrainCellState> {
        if loc.x < 0 || loc.x >= self.size_x as i32 || loc.y < 0 || loc.y >= self.size_y as i32 {
            return None;
        }

        Some(&mut self.data[loc.y as usize][loc.x as usize])
    }

    /// Gets the tile coordinates for the given world space Vec2.
    /// The tile may not actually exist.
    pub fn tile_coords(&self, world_pos: Vec2) -> IVec2 {
        let size = self.sprite_size;

        let offset_x = self.size_x as f32 * size * 0.5;
        let offset_y = self.size_y as f32 * size * 0.5;

        let x = ((world_pos.x + offset_x) / size).floor();
        let y = ((world_pos.y + offset_y) / size).floor();

        IVec2::new(x as i32, y as i32)
    }

    /// Converts from tile coordinates to world coordinates. Used for example
    /// in building placement where want to clamp world coords to tile bounds
    pub fn world_coords(&self, tile_pos: IVec2) -> Vec2 {
        let x =
            tile_pos.x as f32 * self.sprite_size - (self.size_x as f32 * self.sprite_size * 0.5);
        let y =
            tile_pos.y as f32 * self.sprite_size - (self.size_y as f32 * self.sprite_size * 0.5);

        Vec2::new(x, y)
    }

    /// Gets a reference to the tile at the given world position, or None if
    /// no cell exists at that location
    pub fn tile_at_world_pos(&self, world_pos: Vec2) -> Option<&TerrainCellState> {
        let tile_coords = self.tile_coords(world_pos);

        if tile_coords.x < 0 || tile_coords.y < 0 {
            return None;
        }

        let row = self.data.get(tile_coords.y as usize)?;
        row.get(tile_coords.x as usize)
    }

    /// Returns the neighbours of this cell in a regular pattern as defined by
    /// [NEIGHBOUR_COORDINATES]. If the cell coordinate of a neighbour is
    /// invalid (i.e. off the grid) then `None` will be returned.
    fn neighbours(&mut self, x: i32, y: i32) -> impl Iterator<Item = Option<IVec2>> {
        let sx = self.size_x as i32;
        let sy = self.size_y as i32;

        NEIGHBOUR_COORDINATES.iter().map(move |coord| {
            if x + coord.x <= 0 || y + coord.y <= 0 || coord.x + x >= sx || coord.y + y >= sy {
                None
            } else {
                Some(IVec2::new(coord.x + x, coord.y + y))
            }
        })
    }

    /// Updates the map, spreading fire etc
    pub fn update(&mut self, global_wind: Vec2) {
        const BURN_DECAY_RATE: f64 = 0.3;
        const FIRE_SPREAD_CHANCE: f64 = 0.2;
        const MOISTURE_DECAY_RATE: f32 = 0.01;

        let mut rng = rand::thread_rng();

        for y in 0..self.size_y {
            for x in 0..self.size_x {
                let self_terrain = self.data[y][x].terrain;

                match self_terrain {
                    TerrainType::Fire => {
                        if rng.gen_bool(BURN_DECAY_RATE) {
                            let new_fuel_load = self.data[y][x].fuel_load.saturating_sub(1);
                            self.data[y][x].fuel_load = new_fuel_load;

                            if self.data[y][x].fuel_load == 0 {
                                self.data[y][x].terrain = TerrainType::Smoldering;
                                self.data[y][x].dirty = true;
                            }
                        }
                    }
                    TerrainType::Grassland | TerrainType::Tree => {
                        let neighbours = self.neighbours(x as i32, y as i32).collect::<Vec<_>>();
                        for (idx, n) in neighbours.iter().enumerate() {
                            let Some(n) = n else {
                                continue;
                            };

                            let neighbour = self.data[n.y as usize][n.x as usize].terrain;

                            // each neighbouring fire has a chance to set this on fire
                            if matches!(neighbour, TerrainType::Fire) {
                                // reduce moisture of `self` for each neighouring fire cell
                                self.data[y][x].moisture =
                                    (self.data[y][x].moisture - MOISTURE_DECAY_RATE).max(0.0);

                                // on some percentage, spread the fire
                                if rng.gen_bool(FIRE_SPREAD_CHANCE) {
                                    let base_probability = self.data[y][x].terrain.burn_rate();

                                    let wind_angle = (self.data[y][x].wind + global_wind)
                                        .angle_to(NEIGHBOUR_VECTOR[idx]);
                                    let wind_strength = self.data[y][x].wind.length();
                                    let wind_factor =
                                        (wind_strength * 0.131 * (wind_angle.cos() - 1.0))
                                            * (0.045 * wind_strength).exp();

                                    let moisture_factor = 1. - self.data[y][x].moisture;

                                    let burn_chance =
                                        (base_probability * moisture_factor * (1. + wind_factor))
                                            .clamp(0.0, 1.0)
                                            as f64;

                                    // check if we "roll" less than burn_chance, modified by a random
                                    // amount to create some noise in the burning
                                    let rng_factor = rng.r#gen::<f64>();
                                    if rng.gen_bool(burn_chance * rng_factor) {
                                        self.data[y][x].terrain = TerrainType::Fire;
                                        self.data[y][x].dirty = true;
                                        break; // no need to set it on fire any other way, lets take a break
                                    }
                                }
                            }
                        }
                    }
                    TerrainType::Dirt | TerrainType::Stone | TerrainType::Smoldering => {
                        //nop
                    }
                }
            }
        }
    }
}
