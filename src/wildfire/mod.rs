//! A cellular automata system for modelling wildfire.
//! See [https://oneorten.dev/blog/automata_rust_1/]

use bevy::{
    color::palettes::{
        css::SANDY_BROWN,
        tailwind::{
            AMBER_700, AMBER_900, AMBER_950, GREEN_600, GREEN_700, GREEN_800, GREEN_900, LIME_400,
            LIME_500, LIME_600, LIME_700, ORANGE_600, ORANGE_700, YELLOW_400, YELLOW_500,
            YELLOW_600,
        },
    },
    prelude::*,
};
use bevy_life::{Cell, CellState, CellularAutomatonPlugin, LifeSystemSet};

mod lightning;
pub use lightning::OnLightningStrike;

use crate::Pause;

const NEIGHBOR_COORDINATES: [IVec2; 8] = [
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
    app.add_plugins((WildfirePlugin::new().with_time_step(0.1), lightning::plugin));
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
                            terrain: TerrainType::Grassland(4),
                            wind: Vec2::ZERO,
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
        NEIGHBOR_COORDINATES.map(|n| n + self.coords).into_iter()
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
    Grassland(u8),
    Tree(u8),
    Fire(u8),
    Smoldering,
}

/// The state of a given cell in the map
#[derive(Debug, Copy, Clone, PartialEq, Component, Reflect)]
#[reflect(Component)]
pub struct TerrainCellState {
    pub terrain: TerrainType,
    pub wind: Vec2,
}

impl CellState for TerrainCellState {
    fn new_cell_state<'a>(&self, neighbor_cells: impl Iterator<Item = &'a Self>) -> Self {
        let firey_neighbours = if matches!(
            self.terrain,
            TerrainType::Grassland(_) | TerrainType::Tree(_)
        ) {
            neighbor_cells
                .filter(|c| matches!(c.terrain, TerrainType::Fire(_)))
                .count() as f32
        } else {
            0.
        };

        match self.terrain {
            TerrainType::Grassland(size) | TerrainType::Tree(size) => {
                if firey_neighbours >= 4.0 / (size as f32) {
                    return TerrainCellState {
                        terrain: TerrainType::Fire(size),
                        wind: self.wind,
                    };
                } else {
                    return TerrainCellState {
                        terrain: TerrainType::Tree(size),
                        wind: self.wind,
                    };
                }
            }
            TerrainType::Fire(1) => {
                return TerrainCellState {
                    terrain: TerrainType::Smoldering,
                    wind: self.wind,
                };
            }
            TerrainType::Fire(size) => {
                return Self {
                    terrain: TerrainType::Fire(size - 1),
                    wind: self.wind,
                };
            }
            TerrainType::Smoldering | TerrainType::Dirt => {
                // nop
            }
        }

        *self
    }

    fn color(&self) -> Option<bevy::prelude::Color> {
        Some(match self.terrain {
            TerrainType::Dirt => SANDY_BROWN.into(),
            TerrainType::Grassland(size) => match size {
                0 | 1 => LIME_400.into(),
                2 | 3 => LIME_500.into(),
                4 | 5 => LIME_600.into(),
                _ => LIME_700.into(),
            },
            TerrainType::Tree(size) => match size {
                0 | 1 => GREEN_600.into(),
                2 | 3 => GREEN_700.into(),
                4 | 5 => GREEN_800.into(),
                _ => GREEN_900.into(),
            },
            TerrainType::Fire(size) => match size {
                0 => AMBER_900.into(),
                1 => AMBER_700.into(),
                2 => ORANGE_700.into(),
                3 => ORANGE_600.into(),
                4 => YELLOW_600.into(),
                5 => YELLOW_500.into(),
                _ => YELLOW_400.into(),
            },
            TerrainType::Smoldering => AMBER_950.into(),
        })
    }
}
