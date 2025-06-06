//! Logic + code for placing mana forge buildings

use bevy::{prelude::*, sprite::Anchor};
#[cfg(target_os = "macos")]
use bevy_simple_subsecond_system::hot;
use rand::Rng;

use crate::{
    Pause,
    input::MousePosition,
    screens::{
        PlayerResources, Screen,
        gameplay::{
            BuildingMode,
            building::{
                BuildingAssets, BuildingLocation, BuildingType, ManaLine, ManaLineBalls,
                ParentManaForge, mana_forge::ManaForge,
            },
        },
    },
    wildfire::{GameMap, TerrainType},
};

pub(super) fn plugin(app: &mut App) {
    app.register_type::<Minotaur>();

    app.add_systems(
        Update,
        (produce_from_minotaur, while_placing_minotaur).distributive_run_if(
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

fn spawn_minotaur(
    In(config): In<SpawnMinotaur>,
    mut commands: Commands,
    mut resources: ResMut<PlayerResources>,
    mut building_mode: ResMut<BuildingMode>,
    parent_forge: Single<(Entity, &ParentManaForge)>,
    buildings: Res<BuildingAssets>,
    map: Res<GameMap>,
    forges: Query<&Transform, With<ManaForge>>,
) {
    if resources.mana < 30 {
        warn!("Not enough resources to spawn minotaur");
        return;
    }

    let (parent_forge_entity, parent_forge) = *parent_forge;
    let Some(parent_forge) = parent_forge.0 else {
        warn!("No parent mana forge inside tracking, skipping minotaur placement");
        return;
    };

    commands.entity(parent_forge_entity).despawn();
    resources.mana -= 30;

    let coords = map.tile_coords(config.0);
    info!("Spawning minotaur at {coords}");

    let Ok(parent_tx) = forges.get(parent_forge) else {
        warn!("Unable to find parent mana forge");
        return;
    };

    commands.entity(parent_forge).with_children(|builder| {
        builder.spawn((
            BuildingLocation(coords),
            BuildingType::Minotaur,
            Minotaur::default(),
            ManaLine {
                from: parent_tx.translation.truncate().extend(0.05),
                to: config.0.extend(0.05),
                disabled: false,
            },
            ManaLineBalls::default(),
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
        let range = self.range;

        let coords = ((center.y - range).max(0)..=(center.y + range).max(0))
            .flat_map(|y| {
                ((center.x - range).max(0)..=(center.x + range).max(0)).filter_map(move |x| {
                    let v = IVec2::new(x, y);

                    if v.distance_squared(center) > range * range {
                        None
                    } else {
                        Some(v)
                    }
                })
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

#[cfg_attr(target_os = "macos", hot)]
fn while_placing_minotaur(
    mouse: Res<MousePosition>,
    mode: Res<BuildingMode>,
    map: Res<GameMap>,
    mut parent_forge: Single<(&mut ParentManaForge, &mut ManaLine)>,
    forges: Query<(Entity, &Transform), With<ManaForge>>,
) {
    const MAX_DISTANCE_SQR: f32 = 50.0 * 50.0;

    // unlikely but exit early anyway
    if !matches!(*mode, BuildingMode::PlaceMinotaur) {
        return;
    }

    let (forge, parent_mana_line) = &mut *parent_forge;

    // clear previous closest
    let mouse_pos = mouse.world_pos;
    let mut distances = forges
        .iter()
        .filter_map(|(e, tx)| {
            let distance_to_forge = mouse_pos.distance_squared(tx.translation.truncate());
            if distance_to_forge > MAX_DISTANCE_SQR * map.sprite_size {
                return None;
            }

            Some((e, distance_to_forge, tx.translation.truncate()))
        })
        .collect::<Vec<_>>();

    distances.sort_by(|(_, a, _), (_, b, _)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let Some((closest_forge, tx)) = distances.first().map(|(e, _, tx)| (e, tx)) else {
        forge.0 = None;
        parent_mana_line.disabled = true;

        return;
    };

    forge.0 = Some(*closest_forge);
    parent_mana_line.from = tx.extend(0.05);
    parent_mana_line.to = mouse_pos.extend(0.05);
    parent_mana_line.disabled = false;
}

#[cfg_attr(target_os = "macos", hot)]
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
