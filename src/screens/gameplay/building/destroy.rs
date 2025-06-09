//! Code for destroying buildings

use std::time::Duration;

use bevy::{
    ecs::world::OnDespawn,
    platform::collections::{HashMap, HashSet},
    prelude::*,
    time::common_conditions::on_timer,
};
use rand::Rng;

use crate::{
    Pause,
    audio::sound_effect,
    screens::{
        BuildingType, PlayerResources, Screen,
        gameplay::{
            BuildTextHint,
            building::{BuildingAssets, BuildingLocation, ManaEntityLink, ManaLine},
        },
    },
    wildfire::{Fireball, GameMap, MeteorAssets, TerrainType},
};

pub(super) fn plugin(app: &mut App) {
    app.register_type::<BuildingMarkedForDestruction>();

    app.add_systems(
        Update,
        (
            burn_buildings.run_if(on_timer(Duration::from_millis(100))),
            destroy_marked_buildings,
        )
            .run_if(
                in_state(Pause(false))
                    .and(in_state(Screen::Gameplay))
                    .and(resource_exists::<PlayerResources>)
                    .and(resource_exists::<GameMap>),
            ),
    );

    app.add_observer(handle_despawned_buildings);
}

#[derive(Component, Reflect, Debug, Copy, Clone, Default)]
#[reflect(Component)]
pub struct BuildingMarkedForDestruction {
    pub time_until_boom: f32,
}

/// Burns buildings that are consumed by fire
fn burn_buildings(
    mut commands: Commands,
    building_assets: Res<BuildingAssets>,
    map: ResMut<GameMap>,
    buildings: Query<
        (Entity, &BuildingLocation, &BuildingType),
        Without<BuildingMarkedForDestruction>,
    >,
    mut links: Query<(Entity, &ManaEntityLink, Option<&mut ManaLine>)>,
) {
    for (destroyed_entity, loc, building_type) in &buildings {
        // check if there is fire near the building
        if map.check_on_fire(&[
            loc.0,
            loc.0 + IVec2::new(1, 0),
            loc.0 + IVec2::new(1, 1),
            loc.0 + IVec2::new(0, 1),
        ]) {
            // currently despawns child buildings of a mana forge too due to
            // hierarchy.
            info!("{building_type:?} at {loc:?} destroyed by fire");
            if *building_type == BuildingType::CityHall {
                commands.entity(destroyed_entity).despawn();
                return;
            }

            commands.spawn(sound_effect(building_assets.building_lost.clone()));

            // check if there are any children that need to be destroyed
            // lots of looping iteration here but I guess it happens infrequently
            // and its too late to think of a better way.
            // NOTE: lumber mills dont have a parent so have a ManaEntityLink pointing
            //       to themselves so they're captured by this logic
            let mut entities_to_boom = HashSet::<Entity>::from_iter([destroyed_entity]);
            let mut boom_times = HashMap::<Entity, f32>::from_iter([(destroyed_entity, 1.0)]);
            let mut made_changes = true;
            let mut boom_time = 2.0;

            while made_changes {
                made_changes = false;

                for (target_entity, link, _) in &links {
                    if entities_to_boom.contains(&link.from_entity)
                        && !entities_to_boom.contains(&target_entity)
                    {
                        info!("Queuing {} for destruction", target_entity);
                        entities_to_boom.insert(target_entity);
                        boom_times.insert(target_entity, boom_time);
                        made_changes = true;
                    }
                }

                boom_time += 1.0;
            }

            // loop through yet again and do the actual state changes aka the booming
            for entity in &entities_to_boom {
                if let Ok((target_entity, _, maybe_line)) = links.get_mut(*entity) {
                    commands
                        .entity(target_entity)
                        .insert(BuildingMarkedForDestruction {
                            time_until_boom: *boom_times.get(&target_entity).unwrap_or(&2.0),
                        });

                    if let Some(mut line) = maybe_line {
                        line.destroying = true;
                    }
                }
            }
        }
    }
}

fn destroy_marked_buildings(
    mut commands: Commands,
    time: Res<Time>,
    mut marked: Query<(Entity, &mut BuildingMarkedForDestruction)>,
) {
    for (marked_entity, mut destruction) in &mut marked {
        destruction.time_until_boom -= time.delta_secs();
        if destruction.time_until_boom > 0.0 {
            continue;
        }

        commands.entity(marked_entity).despawn();
    }
}

fn handle_despawned_buildings(
    trigger: Trigger<OnDespawn, BuildingType>,
    mut commands: Commands,
    meteor_assets: Res<MeteorAssets>,
    resources: Option<ResMut<PlayerResources>>,
    map: Option<ResMut<GameMap>>,
    mut hint: ResMut<BuildTextHint>,
    buildings: Query<(&BuildingType, &BuildingLocation)>,
) {
    let Some(mut resources) = resources else {
        // probably because we're exiting the game or to menu
        return;
    };

    let Some(mut map) = map else {
        return;
    };

    let target = trigger.target();
    let Ok((building_type, loc)) = buildings.get(target) else {
        warn!("Unable to find building to handle despawn");
        return;
    };

    match building_type {
        BuildingType::CityHall => {
            hint.set("GAME OVER");
        }
        BuildingType::ManaForge => {
            resources.mana_drain -= 3;

            // spawn some chain reaction fire balls
            let mut rng = rand::thread_rng();
            let num_fires = rng.gen_range(2..=6);
            info!("Spawning {num_fires} other fires");

            for _ in 0..num_fires {
                let fire_tile_coords =
                    loc.0 + IVec2::new(rng.gen_range(-14..=14), rng.gen_range(-14..14));

                commands.spawn((
                    StateScoped(Screen::Gameplay),
                    Fireball {
                        target_world_pos: map.world_coords(fire_tile_coords),
                    },
                    Transform::from_translation(map.world_coords(loc.0).extend(0.5)),
                    Sprite {
                        image: meteor_assets.fireball.clone(),
                        ..Default::default()
                    },
                ));

                if let Some(cell) = map.get_mut(fire_tile_coords) {
                    cell.terrain = TerrainType::Fire;
                    cell.mark_dirty();
                }
            }
        }
        BuildingType::StormMage => {
            resources.mana_drain -= 2;
        }
        BuildingType::Minotaur | BuildingType::WaterGolem => {
            resources.mana_drain += 1;
        }
        BuildingType::LumberMill => {}
    }
}
