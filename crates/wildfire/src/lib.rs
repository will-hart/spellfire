//! A cellular automata system for modelling wildfire.
//! See [https://oneorten.dev/blog/automata_rust_1/]

use bevy::{
    color::palettes::{
        css::SANDY_BROWN,
        tailwind::{
            EMERALD_200, EMERALD_300, EMERALD_400, EMERALD_500, EMERALD_600, EMERALD_700,
            EMERALD_800, GREEN_200, GREEN_300, GREEN_400, GREEN_500, GREEN_600, GREEN_700,
            GREEN_800, ORANGE_300, ORANGE_500, RED_200, RED_300, RED_400, YELLOW_500, YELLOW_700,
        },
    },
    prelude::*,
};
use bevy_life::{Cell, CellState, CellularAutomatonPlugin};

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

    app.add_plugins(WildfirePlugin::new().with_time_step(1.0));
    app.add_observer(spawn_map);
}

#[derive(Component, Debug, Reflect, Event)]
#[reflect(Component)]
pub struct OnSpawnMap {
    pub size: UVec2,
    pub sprite_size: f32,
}

fn spawn_map(trigger: Trigger<OnSpawnMap>, mut commands: Commands) {
    info!("Spawning map");

    let data = trigger.event();

    let size_x = data.size.x;
    let size_y = data.size.y;
    let sprite_size = data.sprite_size;

    commands
        .spawn((
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
                0 => GREEN_200.into(),
                1 => GREEN_300.into(),
                2 => GREEN_400.into(),
                3 => GREEN_500.into(),
                4 => GREEN_600.into(),
                5 => GREEN_700.into(),
                _ => GREEN_800.into(),
            },
            TerrainType::Tree(size) => match size {
                0 => EMERALD_200.into(),
                1 => EMERALD_300.into(),
                2 => EMERALD_400.into(),
                3 => EMERALD_500.into(),
                4 => EMERALD_600.into(),
                5 => EMERALD_700.into(),
                _ => EMERALD_800.into(),
            },
            TerrainType::Fire(size) => match size {
                0 => RED_200.into(),
                1 => RED_300.into(),
                2 => RED_400.into(),
                3 => ORANGE_500.into(),
                4 => ORANGE_300.into(),
                5 => YELLOW_700.into(),
                _ => YELLOW_500.into(),
            },
            TerrainType::Smoldering => todo!(),
        })
    }
}
