//! Code for destroying buildings

use std::time::Duration;

use bevy::{ecs::world::OnDespawn, prelude::*, time::common_conditions::on_timer};
use rand::Rng;

use crate::{
    Pause,
    screens::{
        BuildingType, PlayerResources, Screen,
        gameplay::{
            BuildTextHint,
            building::{BuildingLocation, ManaEntityLink, ManaLine},
        },
    },
    wildfire::{GameMap, OnLightningStrike},
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
                    .and(resource_exists::<PlayerResources>),
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
    map: Res<GameMap>,
    buildings: Query<
        (Entity, &BuildingLocation, &BuildingType),
        Without<BuildingMarkedForDestruction>,
    >,
    mut links: Query<(&ManaEntityLink, Option<&mut ManaLine>)>,
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
            commands
                .entity(destroyed_entity)
                .insert(BuildingMarkedForDestruction {
                    time_until_boom: 1.0,
                });

            if *building_type == BuildingType::ManaForge {
                // spawn fires around
                let mut rng = rand::thread_rng();
                let num_fires = rng.gen_range(2..=5);
                info!("Spawning {num_fires} other fires");

                // TODO: JUICE! spawn fireballs to show the effects
                for _ in 0..num_fires {
                    let fire_tile_coords =
                        loc.0 + IVec2::new(rng.gen_range(-10..=10), rng.gen_range(-10..10));
                    commands.trigger(OnLightningStrike(fire_tile_coords));
                }
            }

            // check if there are any children that need to be destroyed
            // TODO: eventually we may need to traverse multiple levels
            for (link, maybe_line) in &mut links {
                if link.from_entity == destroyed_entity {
                    commands
                        .entity(link.to_entity)
                        .insert(BuildingMarkedForDestruction {
                            time_until_boom: 2.0,
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
    resources: Option<ResMut<PlayerResources>>,
    mut hint: ResMut<BuildTextHint>,
    buildings: Query<&BuildingType>,
) {
    let Some(mut resources) = resources else {
        // probably because we're exiting the game or to menu
        return;
    };

    let target = trigger.target();
    let Ok(building_type) = buildings.get(target) else {
        warn!("Unable to find building to handle despawn");
        return;
    };

    match building_type {
        BuildingType::CityHall => {
            hint.set("GAME OVER");
        }
        BuildingType::ManaForge => {
            resources.mana_drain -= 3;
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
