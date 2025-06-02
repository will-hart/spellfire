//! A cellular automata system for modelling wildfire.
//! See [https://oneorten.dev/blog/automata_rust_1/]
//! and [https://github.com/XC-Li/Parallel_CellularAutomaton_Wildfire/blob/master/Wild_Fire.py]

use bevy::{
    color::palettes::{
        css::SANDY_BROWN,
        tailwind::{
            AMBER_700, AMBER_900, GRAY_200, GRAY_300, GRAY_400, GRAY_500, GREEN_400, GREEN_500,
            GREEN_600, GREEN_700, LIME_300, LIME_400, LIME_500, LIME_700, ORANGE_600, ORANGE_700,
            SLATE_800, YELLOW_400, YELLOW_500, YELLOW_600,
        },
    },
    prelude::*,
};
use bevy_life::{Cell, CellState, CellularAutomatonPlugin, LifeSystemSet};
use rand::Rng;

use crate::{Pause, wildfire::mapgen::NoiseMap};

mod lightning;
mod mapgen;
mod wind;

pub use lightning::OnLightningStrike;

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

pub type WildfirePlugin = CellularAutomatonPlugin<TerrainCell, TerrainCellState>;

pub fn plugin(app: &mut App) {
    app.register_type::<OnSpawnMap>();
    app.register_type::<TerrainCell>();
    app.register_type::<TerrainCellState>();
    app.register_type::<TerrainType>();

    app.configure_sets(
        PostUpdate,
        (LifeSystemSet::NewCells, LifeSystemSet::RemovedCells)
            .distributive_run_if(in_state(Pause(false)))
            .chain(),
    );
    app.configure_sets(
        Update,
        LifeSystemSet::CellUpdate.run_if(in_state(Pause(false))),
    );
    app.add_plugins((
        WildfirePlugin::new().with_time_step(0.1),
        lightning::plugin,
        wind::plugin,
    ));
    app.add_observer(spawn_map);
}

// NOTE: slightly weird using this as an event and a resource but game jam
#[derive(Resource, Event, Debug, Reflect, Clone, Copy)]
#[reflect(Resource)]
pub struct OnSpawnMap {
    pub size: UVec2,
    pub sprite_size: f32,
}

#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct SpawnedMap;

fn spawn_map(trigger: Trigger<OnSpawnMap>, mut commands: Commands) {
    info!("Spawning map");

    let data = trigger.event();

    let size_x = data.size.x;
    let size_y = data.size.y;
    let sprite_size = data.sprite_size;

    let noise = NoiseMap::new();

    commands
        .spawn((
            Name::new("Spawned Map"),
            SpawnedMap,
            Transform::from_xyz(
                -(size_x as f32 * sprite_size) / 2.,
                -(size_y as f32 * sprite_size) / 2.,
                0.,
            ),
            Visibility::default(),
        ))
        .with_children(|builder| {
            for y in 0..=size_y {
                for x in 0..=size_x {
                    let (terrain, fuel_load) = noise.sample(x, y);

                    builder.spawn((
                        Sprite {
                            custom_size: Some(Vec2::splat(sprite_size)),
                            ..Default::default()
                        },
                        Transform::from_xyz(sprite_size * x as f32, sprite_size * y as f32, 0.0),
                        TerrainCell {
                            coords: IVec2::new(x as i32, y as i32),
                        },
                        TerrainCellState {
                            terrain,
                            moisture: 0.7,
                            wind: Vec2::ONE * 2.0,
                            fuel_load,
                        },
                    ));
                }
            }
        });

    commands.insert_resource(*data);
}

/// The coordinates for a cellin the terrain map
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct TerrainCell {
    pub coords: IVec2,
}

impl TerrainCell {
    /// Gets the neighbour coordinates for this cell
    pub fn neighbour_coords(&self) -> impl ExactSizeIterator<Item = IVec2> {
        NEIGHBOUR_COORDINATES.map(|n| n + self.coords).into_iter()
    }
}

impl std::ops::Deref for TerrainCell {
    type Target = IVec2;

    fn deref(&self) -> &Self::Target {
        &self.coords
    }
}

impl Cell for TerrainCell {
    type Coordinates = IVec2;

    fn coords(&self) -> &Self::Coordinates {
        &self.coords
    }

    fn neighbor_coordinates(&self) -> impl ExactSizeIterator<Item = Self::Coordinates> + '_ {
        self.neighbour_coords()
    }
}

/// A type of terrain
#[derive(Debug, Clone, Copy, Eq, PartialEq, Reflect)]
pub enum TerrainType {
    Dirt,
    Stone,
    Grassland,
    Tree,
    Fire,
    Smoldering,
}

impl TerrainType {
    pub fn burn_rate(&self) -> f32 {
        match self {
            TerrainType::Fire
            | TerrainType::Smoldering
            | TerrainType::Dirt
            | TerrainType::Stone => 0.0,
            TerrainType::Grassland => 0.8,
            TerrainType::Tree => 0.4,
        }
    }
}

/// The state of a given cell in the map
#[derive(Debug, Copy, Clone, PartialEq, Component, Reflect)]
#[reflect(Component)]
pub struct TerrainCellState {
    pub terrain: TerrainType,
    pub wind: Vec2,
    pub moisture: f32,
    pub fuel_load: u8,
}

impl CellState for TerrainCellState {
    fn new_cell_state<'a>(&self, neighbor_cells: impl Iterator<Item = &'a Self>) -> Self {
        match self.terrain {
            TerrainType::Fire => {
                let mut item = *self;
                if rand::rng().random_bool(0.4) {
                    item.fuel_load = item.fuel_load.saturating_sub(1);

                    if self.fuel_load == 0 {
                        item.terrain = TerrainType::Smoldering;
                    }
                }

                item
            }
            TerrainType::Grassland | TerrainType::Tree => {
                let mut item = *self;
                let mut rng = rand::rng();

                for (idx, n) in neighbor_cells.enumerate() {
                    // each neighbouring fire has a chance to set this on fire
                    if rng.random_bool(0.18) && matches!(n.terrain, TerrainType::Fire) {
                        let base_probability = self.terrain.burn_rate();

                        let wind_angle = self.wind.angle_to(NEIGHBOUR_VECTOR[idx]);
                        let wind_strength = self.wind.length();
                        let wind_factor = (wind_strength * 0.131 * (wind_angle.cos() - 1.0))
                            * (0.045 * wind_strength).exp();

                        let moisture_factor = 1. - self.moisture;

                        let burn_chance =
                            (base_probability * (1. + moisture_factor) * (1. + wind_factor))
                                .clamp(0.0, 1.0) as f64
                                * rng.random::<f64>();
                        if rng.random_bool(burn_chance) {
                            item.terrain = TerrainType::Fire;
                        }
                    }
                }

                item
            }
            TerrainType::Dirt | TerrainType::Stone | TerrainType::Smoldering => *self,
        }
    }

    fn color(&self) -> Option<bevy::prelude::Color> {
        Some(match self.terrain {
            TerrainType::Dirt => SANDY_BROWN.into(),
            TerrainType::Grassland => match self.fuel_load {
                0 | 1 => LIME_300.into(),
                2 | 3 => LIME_400.into(),
                4 | 5 => LIME_500.into(),
                _ => LIME_700.into(),
            },
            TerrainType::Tree => match self.fuel_load {
                0..=4 => GREEN_400.into(),
                5..=7 => GREEN_500.into(),
                8..=11 => GREEN_600.into(),
                _ => GREEN_700.into(),
            },
            TerrainType::Fire => match self.fuel_load {
                0 => AMBER_900.into(),
                1 => AMBER_700.into(),
                2 => ORANGE_700.into(),
                3 => ORANGE_600.into(),
                4 => YELLOW_600.into(),
                5 => YELLOW_500.into(),
                _ => YELLOW_400.into(),
            },
            TerrainType::Stone => match self.fuel_load {
                0 | 1 => GRAY_500.into(),
                2 | 3 => GRAY_400.into(),
                4 | 5 => GRAY_300.into(),
                _ => GRAY_200.into(),
            },
            TerrainType::Smoldering => SLATE_800.into(),
        })
    }
}
