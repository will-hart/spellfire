//! The screen state for the main gameplay.

use bevy::{
    color::palettes::tailwind::SLATE_800, input::common_conditions::input_just_pressed,
    math::CompassOctant, prelude::*, ui::Val::*,
};
use bevy_simple_subsecond_system::prelude::*;

use crate::{
    Pause,
    demo::level::spawn_level,
    input::MousePosition,
    menus::Menu,
    screens::{Screen, gameplay::building::SpawnManaForge},
    theme::node_builder::NodeBuilder,
    wildfire::{GameMap, OnLightningStrike, WindDirection},
};

mod building;

pub(super) fn plugin(app: &mut App) {
    app.register_type::<EnergyTextMarker>();
    app.register_type::<BuildingMode>();
    app.register_type::<CursorModeItem>();
    app.register_type::<PlayerResources>();
    app.register_type::<BuildingBar>();

    app.init_resource::<BuildingMode>();

    app.add_plugins(building::plugin);

    app.add_systems(
        OnEnter(Screen::Gameplay),
        (spawn_level, spawn_toolbar, draw_building_bar),
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
        cancel_cursor_mode
            .run_if(in_state(Screen::Gameplay).and(
                input_just_pressed(KeyCode::Space).or(input_just_pressed(MouseButton::Right)),
            )),
    );

    app.add_systems(
        Update,
        (
            update_toolbar.run_if(resource_exists::<PlayerResources>),
            handle_mouse_click_input.run_if(input_just_pressed(MouseButton::Left)),
            handle_build_mode_change
                .run_if(resource_changed::<BuildingMode>)
                .after(cancel_cursor_mode),
        )
            .chain()
            .run_if(in_state(Screen::Gameplay).and(in_state(Pause(false)))),
    );
}

#[derive(Resource, Reflect, Debug, Clone)]
#[reflect(Resource)]
pub struct PlayerResources {
    pub mana: u32,
}

impl Default for PlayerResources {
    fn default() -> Self {
        Self { mana: 15 }
    }
}

#[derive(Resource, Reflect, Debug, Clone, Default)]
#[reflect(Resource)]
pub enum BuildingMode {
    #[default]
    None,
    PlaceManaForge,
}

fn unpause(mut next_pause: ResMut<NextState<Pause>>) {
    next_pause.set(Pause(false));
}

fn pause(mut next_pause: ResMut<NextState<Pause>>) {
    next_pause.set(Pause(true));
}

fn handle_mouse_click_input(
    mut commands: Commands,
    mut mode: ResMut<BuildingMode>,
    mouse: Res<MousePosition>,
    maybe_map: Option<Res<GameMap>>,
) {
    match *mode {
        BuildingMode::None => {
            if let Some(map) = maybe_map {
                let coords = map.tile_coords(mouse.world_pos);
                commands.trigger(OnLightningStrike(coords));
            } else {
                warn!("Skipping lightning strike input as there is no map yet");
            }
        }
        BuildingMode::PlaceManaForge => {
            commands.queue(SpawnManaForge(mouse.world_pos));
            *mode = BuildingMode::None;
        }
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
            // .width(Val::Px(200.0))
            .height(Val::Px(32.0))
            .center_content()
            .build(),
        Button,
        children![(Text::new(text), TextFont::from_font_size(12.0))],
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
    map: Res<GameMap>,
    mut energy_text: Single<&mut Text, (Without<WindTextMarker>, With<EnergyTextMarker>)>,
    mut wind_text: Single<&mut Text, (Without<EnergyTextMarker>, With<WindTextMarker>)>,
) {
    let cell_state = if let Some(cell) = map.tile_at_world_pos(mouse.world_pos) {
        format!("{}", *cell)
    } else {
        String::new()
    };

    energy_text.0 = format!("MANA: {}", player_resource.mana);
    wind_text.0 = format!(
        " | WIND: From {} / {} | {cell_state}",
        match CompassOctant::from(Dir2::new(wind.0).expect("to dir")) {
            CompassOctant::North => "N",
            CompassOctant::NorthEast => "NE",
            CompassOctant::East => "E",
            CompassOctant::SouthEast => "SE",
            CompassOctant::South => "S",
            CompassOctant::SouthWest => "SW",
            CompassOctant::West => "W",
            CompassOctant::NorthWest => "NW",
        },
        wind.0
    );
}

#[derive(Component, Reflect, Debug, Clone, Default)]
#[reflect(Component)]
pub struct BuildingBar;

#[hot]
fn draw_building_bar(
    mut commands: Commands,
    mode: Res<BuildingMode>,
    previous_items: Query<Entity, With<BuildingBar>>,
) {
    for entity in &previous_items {
        commands.entity(entity).despawn();
    }

    commands
        .spawn((
            toolbar_node().bottom(0.0).build(),
            StateScoped(Screen::Gameplay),
            BuildingBar,
        ))
        .with_children(|builder| match *mode {
            BuildingMode::None => {
                builder.spawn(toolbar_button("Mana Forge")).observe(
                    |_trigger: Trigger<Pointer<Click>>, mut mode: ResMut<BuildingMode>| {
                        info!("Placing Maa Forge");
                        *mode = BuildingMode::PlaceManaForge;
                    },
                );
            }
            BuildingMode::PlaceManaForge => {
                builder.spawn((
                    Text::new("Click the map to place a forge. Press <space> to cancel placement."),
                    TextFont::from_font_size(14.0),
                ));
            }
        });
}

fn cancel_cursor_mode(mut mode: ResMut<BuildingMode>) {
    info!("Resetting curs mode");
    *mode = BuildingMode::None;
}

#[derive(Component, Reflect, Debug, Clone, Default)]
#[reflect(Component)]
pub struct CursorModeItem;

#[hot]
fn handle_build_mode_change(
    mut commands: Commands,
    previous_items: Query<Entity, With<CursorModeItem>>,
) {
    // despawn previous entities
    for entity in &previous_items {
        commands.entity(entity).despawn();
    }

    commands.run_system_cached(draw_building_bar);

    // TODO spawn cursor/buldling placement items
}
