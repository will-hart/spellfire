//! A cellular automata system for modelling wildfire.
//! See [https://oneorten.dev/blog/automata_rust_1/]
//! and [https://github.com/XC-Li/Parallel_CellularAutomaton_Wildfire/blob/master/Wild_Fire.py]

use bevy::{
    color::palettes::{
        css::{BLACK, WHITE},
        tailwind::{
            AMBER_700, AMBER_900, GREEN_900, LIME_500, ORANGE_600, ORANGE_700, SLATE_700,
            STONE_500, YELLOW_400, YELLOW_500, YELLOW_600,
        },
    },
    prelude::*,
};

mod lightning;
mod map;
mod wind;

pub use lightning::OnLightningStrike;
pub use map::GameMap;
pub use wind::WindDirection;

pub fn plugin(app: &mut App) {
    app.register_type::<OnSpawnMap>();
    app.register_type::<TerrainCell>();
    app.register_type::<TerrainCellState>();
    app.register_type::<TerrainType>();

    app.add_plugins((map::plugin, lightning::plugin, wind::plugin));
    app.add_observer(spawn_map);
}

// NOTE: slightly weird using this as an event and a resource but game jam
#[derive(Event, Debug, Reflect, Clone, Copy)]
pub struct OnSpawnMap {
    pub size: UVec2,
    pub sprite_size: f32,
}

#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct SpawnedMap;

#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct TerrainCell;

fn spawn_map(trigger: Trigger<OnSpawnMap>, mut commands: Commands) {
    info!("Spawning map");

    let data = trigger.event();
    let size_x = data.size.x;
    let size_y = data.size.y;
    let sprite_size = data.sprite_size;

    let mut map = GameMap::new(sprite_size, size_x as usize, size_y as usize);

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
            for y in 0..size_y {
                for x in 0..size_x {
                    let entity = builder
                        .spawn((
                            TerrainCell,
                            Sprite {
                                custom_size: Some(Vec2::splat(sprite_size)),
                                ..Default::default()
                            },
                            Transform::from_xyz(
                                sprite_size * x as f32,
                                sprite_size * y as f32,
                                0.0,
                            ),
                        ))
                        .id();

                    let cell = &mut map.data[y as usize][x as usize];
                    cell.sprite_entity = Some(entity);
                    cell.dirty = true;
                }
            }
        });

    commands.insert_resource(map);
}

/// A type of terrain
#[derive(Debug, Clone, Copy, Eq, PartialEq, Reflect, Default)]
pub enum TerrainType {
    Dirt,
    #[default]
    Grassland,
    Tree,
    Stone,
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

impl std::fmt::Display for TerrainType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                TerrainType::Dirt => "Earth",
                TerrainType::Stone => "Stone",
                TerrainType::Grassland => "Grass",
                TerrainType::Tree => "Forest",
                TerrainType::Fire => "Fire",
                TerrainType::Smoldering => "Burnt Ground",
            }
        )
    }
}

/// The state of a given cell in the map
#[derive(Debug, Copy, Clone, PartialEq, Reflect, Default)]
pub struct TerrainCellState {
    pub terrain: TerrainType,
    pub wind: Vec2,
    pub moisture: f32,
    pub fuel_load: u8,

    pub sprite_entity: Option<Entity>,
    dirty: bool,
}

impl std::fmt::Display for TerrainCellState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.terrain {
            TerrainType::Grassland | TerrainType::Tree => {
                write!(
                    f,
                    "{}{}{}",
                    if matches!(self.terrain, TerrainType::Grassland) {
                        if self.fuel_load < 3 {
                            "Short "
                        } else if self.fuel_load > 8 {
                            "Long "
                        } else {
                            ""
                        }
                    } else {
                        ""
                    },
                    if self.moisture < 0.3 {
                        "Wet "
                    } else if self.moisture > 0.8 {
                        "Dry "
                    } else {
                        ""
                    },
                    self.terrain,
                )
            }
            TerrainType::Dirt
            | TerrainType::Stone
            | TerrainType::Fire
            | TerrainType::Smoldering => write!(f, "{}", self.terrain),
        }
    }
}

impl TerrainCellState {
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    fn colour(&self) -> Color {
        match self.terrain {
            TerrainType::Dirt => Color::Srgba(Srgba {
                red: 0.37,
                green: 0.27,
                blue: 0.08,
                alpha: 1.0,
            }),
            TerrainType::Grassland => match self.fuel_load {
                0 | 1 => LIME_500.mix(&WHITE, 0.03).into(),
                2 | 3 => LIME_500.into(),
                4 | 5 => LIME_500.mix(&BLACK, 0.025).into(),
                _ => LIME_500.mix(&BLACK, 0.05).into(),
            },
            TerrainType::Tree => match self.fuel_load {
                0..=4 => GREEN_900.mix(&BLACK, 0.025).into(),
                5..=7 => GREEN_900.into(),
                8..=11 => GREEN_900.mix(&WHITE, 0.05).into(),
                _ => GREEN_900.mix(&WHITE, 0.075).into(),
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
                0 | 1 => STONE_500.mix(&BLACK, 0.05).into(),
                2 | 3 => STONE_500.into(),
                4 | 5 => STONE_500.mix(&WHITE, 0.05).into(),
                _ => STONE_500.mix(&WHITE, 0.1).into(),
            },
            TerrainType::Smoldering => SLATE_700.into(),
        }
    }
}
