//! The screen state for the main gameplay.

use bevy::{
    color::palettes::tailwind::SLATE_800, input::common_conditions::input_just_pressed,
    math::CompassOctant, prelude::*, ui::Val::*,
};

use crate::{
    Pause,
    demo::level::spawn_level,
    input::MousePosition,
    menus::Menu,
    screens::Screen,
    theme::node_builder::NodeBuilder,
    wildfire::{OnLightningStrike, OnSpawnMap, TerrainCell, TerrainCellState, WindDirection},
};

pub(super) fn plugin(app: &mut App) {
    app.register_type::<EnergyTextMarker>();
    app.register_type::<PlayerResources>();

    app.add_systems(
        OnEnter(Screen::Gameplay),
        (spawn_level, spawn_toolbar, spawn_building_bar),
    );

    // Toggle pause on key press.
    app.add_systems(
        Update,
        (
            (pause, spawn_pause_overlay, open_pause_menu).run_if(
                in_state(Screen::Gameplay)
                    .and(in_state(Menu::None))
                    .and(input_just_pressed(KeyCode::KeyP).or(input_just_pressed(KeyCode::Escape))),
            ),
            close_menu.run_if(
                in_state(Screen::Gameplay)
                    .and(not(in_state(Menu::None)))
                    .and(input_just_pressed(KeyCode::KeyP)),
            ),
        ),
    );
    app.add_systems(OnExit(Screen::Gameplay), (close_menu, unpause));
    app.add_systems(
        OnEnter(Menu::None),
        unpause.run_if(in_state(Screen::Gameplay)),
    );

    app.add_systems(
        Update,
        handle_lightning_strike_input
            .run_if(in_state(Screen::Gameplay).and(input_just_pressed(MouseButton::Left))),
    );

    app.add_systems(
        Update,
        update_toolbar.run_if(in_state(Pause(false)).and(resource_exists::<PlayerResources>)),
    );
}

#[derive(Resource, Reflect, Debug, Clone)]
#[reflect(Resource)]
pub struct PlayerResources {
    pub energy: u32,
}

impl Default for PlayerResources {
    fn default() -> Self {
        Self { energy: 15 }
    }
}

fn unpause(mut next_pause: ResMut<NextState<Pause>>) {
    next_pause.set(Pause(false));
}

fn pause(mut next_pause: ResMut<NextState<Pause>>) {
    next_pause.set(Pause(true));
}

fn handle_lightning_strike_input(
    mut commands: Commands,
    mouse_pos: Res<MousePosition>,
    maybe_map: Option<Res<OnSpawnMap>>,
) {
    if let Some(map) = maybe_map {
        let coords = map.tile_coords(&mouse_pos);
        commands.trigger(OnLightningStrike(coords));
    } else {
        warn!("Skipping lightning strike input as there is no map yet");
    }
}

fn spawn_pause_overlay(mut commands: Commands) {
    commands.spawn((
        Name::new("Pause Overlay"),
        Node {
            width: Percent(100.0),
            height: Percent(100.0),
            ..default()
        },
        GlobalZIndex(1),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
        StateScoped(Pause(true)),
    ));
}

fn open_pause_menu(mut next_menu: ResMut<NextState<Menu>>) {
    next_menu.set(Menu::Pause);
}

fn close_menu(mut next_menu: ResMut<NextState<Menu>>) {
    next_menu.set(Menu::None);
}

#[derive(Component, Reflect, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct EnergyTextMarker;

#[derive(Component, Reflect, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct WindTextMarker;

fn toolbar_node() -> NodeBuilder {
    NodeBuilder::new()
        .position(PositionType::Absolute)
        .width(Val::Percent(100.0))
        .height(Val::Px(35.0))
        .left(0.0)
        .background(SLATE_800)
        .flex_direction(FlexDirection::Row)
        .align(AlignItems::Center)
}

fn toolbar_button(text: impl Into<String>) -> impl Bundle {
    (
        NodeBuilder::new()
            .width(Val::Px(100.0))
            .height(Val::Px(32.0))
            .center_content()
            .build(),
        Button,
        children![
            Text::new(text),
            TextFont {
                font_size: 12.0,
                ..default()
            }
        ],
    )
}

fn spawn_toolbar(mut commands: Commands) {
    commands.spawn((
        toolbar_node().top(0.0).build(),
        StateScoped(Screen::Gameplay),
        children![
            (
                EnergyTextMarker,
                Text::new("ENERGY: 10"),
                TextFont {
                    font_size: 12.0,
                    ..default()
                }
            ),
            (
                WindTextMarker,
                Text::new(""),
                TextFont {
                    font_size: 12.0,
                    ..default()
                }
            )
        ],
    ));
}

fn update_toolbar(
    player_resource: Res<PlayerResources>,
    wind: Res<WindDirection>,
    mouse: Res<MousePosition>,
    map: Res<OnSpawnMap>,
    mut energy_text: Single<&mut Text, (Without<WindTextMarker>, With<EnergyTextMarker>)>,
    mut wind_text: Single<&mut Text, (Without<EnergyTextMarker>, With<WindTextMarker>)>,
    tiles: Query<(&TerrainCellState, &TerrainCell)>,
) {
    let loc = map.tile_coords(&mouse);
    let mouse_over_terrain = if let Some(state) = tiles
        .iter()
        .find_map(|(s, t)| if t.coords == loc { Some(s) } else { None })
    {
        format!("{}", *state)
    } else {
        String::new()
    };

    energy_text.0 = format!("ENERGY: {}", player_resource.energy);
    wind_text.0 = format!(
        " | WIND: From {} | {mouse_over_terrain}",
        match CompassOctant::from(Dir2::new(wind.0).expect("to dir")) {
            CompassOctant::North => "N",
            CompassOctant::NorthEast => "NE",
            CompassOctant::East => "E",
            CompassOctant::SouthEast => "SE",
            CompassOctant::South => "S",
            CompassOctant::SouthWest => "SW",
            CompassOctant::West => "W",
            CompassOctant::NorthWest => "NW",
        }
    );
}

fn spawn_building_bar(mut commands: Commands) {
    commands
        .spawn((
            toolbar_node().bottom(0.0).build(),
            StateScoped(Screen::Gameplay),
        ))
        .with_children(|builder| {
            builder
                .spawn(toolbar_button("LEY"))
                .observe(|_trigger: Trigger<Pointer<Click>>| {
                    info!("CLICKED");
                });
        });
}
