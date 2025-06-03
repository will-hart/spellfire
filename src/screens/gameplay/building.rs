//! Logic + code for placing buildings

use bevy::prelude::*;

use crate::{Pause, screens::PlayerResources, wildfire::GameMap};

pub(super) fn plugin(app: &mut App) {
    app.register_type::<BuildingLocation>();
    app.register_type::<ManaForge>();

    app.add_systems(
        Update,
        mana_forge.run_if(in_state(Pause(false)).and(resource_exists::<PlayerResources>)),
    );
}

#[derive(Component, Reflect, Debug, Copy, Clone)]
#[reflect(Component)]
pub struct BuildingLocation(pub IVec2);

/// A mana producing building
#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub struct ManaForge {
    pub mana_per_second: u32,
    pub health: u32,
    time_since_last_tick: f32,
}

fn mana_forge(
    time: Res<Time>,
    mut player: ResMut<PlayerResources>,
    mut forges: Query<&mut ManaForge>,
) {
    let delta = time.delta_secs();

    for mut forge in &mut forges {
        if forge.time_since_last_tick + delta <= 1.0 {
            forge.time_since_last_tick += delta;
            continue;
        }

        forge.time_since_last_tick = 0.0;
        player.mana += forge.mana_per_second;
    }
}

fn blow_up_mana_forge(
    map: Res<GameMap>,
    mut forges: Query<(Entity, &BuildingLocation, &mut ManaForge)>,
) {
    // check if there is fire near the mana forge

    // if there is then reduce health
    // if health is 0, blow the mana forge up and spawn random fires nearby
}
