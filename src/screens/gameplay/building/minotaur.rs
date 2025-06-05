//! Logic + code for placing mana forge buildings

use bevy::{prelude::*, sprite::Anchor};
use rand::Rng;

use crate::{
    Pause,
    input::MousePosition,
    screens::{
        PlayerResources, Screen,
        gameplay::{
            BuildingMode,
            building::{
                BuildingAssets, BuildingLocation, BuildingType, ParentManaForge,
                mana_forge::ManaForge,
            },
        },
    },
    wildfire::{GameMap, TerrainType},
};

pub(super) fn plugin(app: &mut App) {
    app.register_type::<Minotaur>();

    app.add_systems(
        Update,
        (
            produce_from_minotaur,
            while_placing_minotaur.run_if(resource_exists::<ParentManaForge>),
        )
            .run_if(
                in_state(Pause(false))
                    .and(in_state(Screen::Gameplay))
                    .and(resource_exists::<PlayerResources>),
            ),
    );
}

#[derive(Debug, Clone, Copy)]
pub struct SpawnMinotaur(pub Vec2);

impl Command for SpawnMinotaur {
    fn apply(self, world: &mut World) -> () {
        let _ = world.run_system_cached_with(spawn_minotaur, self);
    }
}

#[cfg_attr(target_os = "macos", hot)]
fn spawn_minotaur(
    In(config): In<SpawnMinotaur>,
    mut commands: Commands,
    mut resources: ResMut<PlayerResources>,
    mut building_mode: ResMut<BuildingMode>,
    parent_forge: Res<ParentManaForge>,
    buildings: Res<BuildingAssets>,
    map: Res<GameMap>,
    forges: Query<&Transform, With<ManaForge>>,
) {
    if resources.mana < 30 {
        warn!("Not enough resources to spawn minotaur");
        return;
    }

    let Some(parent) = parent_forge.0 else {
        warn!("No parent mana forge, skipping minotaur placement");
        return;
    };

    resources.mana -= 30;
    commands.remove_resource::<ParentManaForge>();

    let coords = map.tile_coords(config.0);
    info!("Spawning minotaur at {coords}");

    let Ok(parent_tx) = forges.get(parent) else {
        warn!("Unable to find parent mana forge");
        return;
    };

    commands.entity(parent).with_children(|builder| {
        builder.spawn((
            BuildingLocation(coords),
            BuildingType::Minotaur,
            Minotaur::default(),
            StateScoped(Screen::Gameplay),
            Transform::from_xyz(
                // coords.x as f32 * map.sprite_size,
                // coords.y as f32 * map.sprite_size,
                config.0.x - parent_tx.translation.x,
                config.0.y - parent_tx.translation.y,
                0.1,
            ),
            Visibility::Visible,
            Sprite {
                image: buildings.minotaur.clone(),
                custom_size: Some(Vec2::splat(16.0)),
                anchor: Anchor::Center,
                ..default()
            },
        ));
    });

    *building_mode = BuildingMode::None;
}

/// A mana producing building
#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub struct Minotaur {
    /// The time the minotaur was last updated
    time_since_last_tick: f32,
    /// The offset from the building location to describe where this minotaur
    /// currently is
    location: IVec2,
    /// The range of the minotaur (i.e. distance from the building location)
    range: i32,
}

impl Default for Minotaur {
    fn default() -> Self {
        Self {
            time_since_last_tick: 0.0,
            location: IVec2::ZERO,
            range: 5,
        }
    }
}

impl Minotaur {
    /// Move the minotaur to a random new position
    fn move_to_grass(&mut self, map: &mut GameMap, center: IVec2) {
        // first find all the available cells that are grass or trees
        let coords = ((center.y - self.range).max(0)..=(center.y + self.range).max(0))
            .flat_map(|y| {
                ((center.x - self.range).max(0)..=(center.x + self.range).max(0))
                    .map(move |x| IVec2::new(x, y))
            })
            .filter_map(
                // limit to trees and grass
                |coord| match map.data[coord.y as usize][coord.x as usize].terrain {
                    TerrainType::Grassland | TerrainType::Tree => Some(coord),
                    _ => None,
                },
            )
            .collect::<Vec<_>>();

        if coords.is_empty() {
            self.location = IVec2::ZERO;
            return;
        }

        // now pick one and move there
        let mut rng = rand::rng();
        let idx = rng.random_range(0..coords.len());
        self.location = coords[idx];
    }
}

fn while_placing_minotaur(
    mouse: Res<MousePosition>,
    mode: Res<BuildingMode>,
    map: Res<GameMap>,
    mut nearest_forge: ResMut<ParentManaForge>,
    forges: Query<(Entity, &BuildingLocation), With<ManaForge>>,
) {
    const MAX_DISTANCE: i32 = 30;

    // unlikely but exit early anyway
    if !matches!(*mode, BuildingMode::PlaceMinotaur) {
        return;
    }

    // clear previous closest
    let mouse_pos = map.tile_coords(mouse.world_pos);
    let mut distances = forges
        .iter()
        .filter_map(|(e, loc)| {
            let distance_to_forge = mouse_pos.distance_squared(loc.0);
            if distance_to_forge > MAX_DISTANCE * MAX_DISTANCE {
                return None;
            }

            Some((e, distance_to_forge))
        })
        .collect::<Vec<_>>();

    distances.sort_by(|(_, a), (_, b)| a.cmp(b));

    nearest_forge.0 = distances.first().map(|(e, _)| e).copied();
}

fn produce_from_minotaur(
    time: Res<Time>,
    mut map: ResMut<GameMap>,
    mut forges: Query<(&BuildingLocation, &mut Minotaur)>,
) {
    let delta = time.delta_secs();

    for (loc, mut minotaur) in &mut forges {
        if minotaur.time_since_last_tick + delta <= 0.5 {
            minotaur.time_since_last_tick += delta;
            continue;
        }

        minotaur.time_since_last_tick = 0.0;

        // reduce the current cell
        if let Some(current) = map.get_mut(minotaur.location) {
            match current.terrain {
                TerrainType::Grassland => {
                    current.terrain = TerrainType::Dirt;
                    current.mark_dirty();
                }
                TerrainType::Tree => {
                    current.terrain = TerrainType::Grassland;
                    current.mark_dirty();
                    // continue, don't move on until we have dirt
                    continue;
                }
                TerrainType::Dirt
                | TerrainType::Stone
                | TerrainType::Fire
                | TerrainType::Smoldering => {
                    // nop
                }
            }
        }

        // move the minotaur in a spiral
        minotaur.move_to_grass(&mut map, loc.0);
    }
}
