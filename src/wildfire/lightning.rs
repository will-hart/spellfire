//! A plugin that adds lightning in a given grid location. Can be triggered
//! on click other other user input or randomly by triggering [OnLightningStrike]

use bevy::prelude::*;

use crate::wildfire::{TerrainCell, TerrainCellState, TerrainType};

pub(super) fn plugin(app: &mut App) {
    app.register_type::<OnLightningStrike>();

    app.add_observer(handle_lightning_strike);
}

#[derive(Debug, Clone, Event, Reflect)]
pub struct OnLightningStrike(pub IVec2);

fn handle_lightning_strike(
    trigger: Trigger<OnLightningStrike>,
    mut tiles: Query<(&mut TerrainCellState, &TerrainCell)>,
) {
    let loc = trigger.event().0;

    if let Some(mut state) = tiles
        .iter_mut()
        .find_map(|(s, t)| if t.coords == loc { Some(s) } else { None })
    {
        state.terrain = match state.terrain {
            TerrainType::Grassland(size) | TerrainType::Tree(size) => TerrainType::Fire(size),
            TerrainType::Fire(size) => TerrainType::Fire(size + 2),
            TerrainType::Dirt | TerrainType::Smoldering => state.terrain,
        };
    } else {
        warn!("Can't find tile for lightning strike at {loc:?}");
    }
}
