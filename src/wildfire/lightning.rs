//! A plugin that adds lightning in a given grid location. Can be triggered
//! on click other other user input or randomly by triggering [OnLightningStrike]

use bevy::prelude::*;

use crate::wildfire::{TerrainType, map::GameMap};

pub(super) fn plugin(app: &mut App) {
    app.register_type::<OnLightningStrike>();

    app.add_observer(handle_lightning_strike);
}

#[derive(Debug, Clone, Event, Reflect)]
pub struct OnLightningStrike(pub IVec2);

fn handle_lightning_strike(trigger: Trigger<OnLightningStrike>, mut map: ResMut<GameMap>) {
    let loc = trigger.event().0;
    let Some(cell) = map.get_mut(loc) else {
        info!("Unable to find cell for lightning strike at {loc:?}");
        return;
    };

    match cell.terrain {
        TerrainType::Grassland | TerrainType::Tree => {
            info!("Spawning lightning strike at {loc}");
            cell.terrain = TerrainType::Fire;
            cell.dirty = true;
        }
        TerrainType::Fire | TerrainType::Dirt | TerrainType::Stone | TerrainType::Smoldering => {}
    }
}
