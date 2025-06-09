//! Gameplay toolbar code, separated to make the file manageable and maybe
//! so that `cargo fmt` doesn't break constantly

use std::time::Duration;

use bevy::{
    color::palettes::tailwind::{SLATE_400, SLATE_700, SLATE_800, SLATE_950},
    ecs::relationship::RelatedSpawnerCommands,
    input::common_conditions::input_just_pressed,
    prelude::*,
    time::common_conditions::on_timer,
};

use crate::{
    Pause,
    demo::level::spawn_level,
    input::MousePosition,
    screens::{
        BuildingMode, EndlessMode, PlayerResources, RequiresCityHall, Screen,
        gameplay::{
            BuildTextHint, BuildTextMarker, HintMessage, LUMBER_MILL_COST_LUMBER,
            MANA_FORGE_COST_LUMBER, MINOTAUR_COST_MANA, STORM_MAGE_COST_MANA,
            WATER_GOLEM_COST_MANA,
            building::{BuildingAssets, ResourceAssets},
        },
    },
    theme::node_builder::NodeBuilder,
    wildfire::{GameMap, WindDirection},
};

pub(super) fn plugin(app: &mut App) {
    app.register_type::<ToolbarUi>();
    app.register_type::<ToolbarButtonType>();
    app.register_type::<EnergyTextMarker>();
    app.register_type::<LumberTextMarker>();
    app.register_type::<BuildingHintToolbar>();

    app.add_systems(
        OnEnter(Screen::Gameplay),
        (spawn_level, spawn_toolbar.after(spawn_level)),
    );
    app.add_systems(
        Update,
        (
            update_toolbar.run_if(
                resource_exists::<PlayerResources>.and(on_timer(Duration::from_millis(300))),
            ),
            update_build_hint_ui,
        )
            .chain()
            .run_if(in_state(Screen::Gameplay).and(in_state(Pause(false)))),
    );

    app.add_observer(handle_on_redraw_toolbar)
        .add_observer(handle_disabling_toolbar_buttons);

    app.add_systems(
        Update,
        (
            meteor_hotkey.run_if(input_just_pressed(KeyCode::Digit0)),
            mana_forge_hotkey.run_if(input_just_pressed(KeyCode::Digit1)),
            lumber_mill_hotkey.run_if(input_just_pressed(KeyCode::Digit2)),
            minotaur_hotkey.run_if(input_just_pressed(KeyCode::Digit3)),
            water_golem_hotkey.run_if(input_just_pressed(KeyCode::Digit4)),
            storm_mage_hotkey.run_if(input_just_pressed(KeyCode::Digit5)),
        )
            .run_if(in_state(Screen::Gameplay).and(in_state(Pause(false)))),
    );
}

fn meteor_hotkey(mut mode: ResMut<BuildingMode>, mut hint: ResMut<BuildTextHint>) {
    if *mode == BuildingMode::None {
        *mode = BuildingMode::Meteor;
        hint.0 = toolbar_data(ToolbarButtonType::Meteor).1;
    }
}

fn mana_forge_hotkey(mut mode: ResMut<BuildingMode>, mut hint: ResMut<BuildTextHint>) {
    if *mode == BuildingMode::None {
        *mode = BuildingMode::PlaceManaForge;
        hint.0 = toolbar_data(ToolbarButtonType::ManaForge).1;
    }
}

fn lumber_mill_hotkey(mut mode: ResMut<BuildingMode>, mut hint: ResMut<BuildTextHint>) {
    if *mode == BuildingMode::None {
        *mode = BuildingMode::PlaceLumberMill;
        hint.0 = toolbar_data(ToolbarButtonType::LumberMill).1;
    }
}

fn minotaur_hotkey(mut mode: ResMut<BuildingMode>, mut hint: ResMut<BuildTextHint>) {
    if *mode == BuildingMode::None {
        *mode = BuildingMode::PlaceMinotaur;
        hint.0 = toolbar_data(ToolbarButtonType::MinotaurHutch).1;
    }
}

fn water_golem_hotkey(mut mode: ResMut<BuildingMode>, mut hint: ResMut<BuildTextHint>) {
    if *mode == BuildingMode::None {
        *mode = BuildingMode::PlaceWaterGolem;
        hint.0 = toolbar_data(ToolbarButtonType::WaterGolem).1;
    }
}

fn storm_mage_hotkey(mut mode: ResMut<BuildingMode>, mut hint: ResMut<BuildTextHint>) {
    if *mode == BuildingMode::None {
        *mode = BuildingMode::PlaceStormMage;
        hint.0 = toolbar_data(ToolbarButtonType::StormMage).1;
    }
}

#[derive(Component, Reflect, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct EnergyTextMarker;

#[derive(Component, Reflect, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct LumberTextMarker;

#[derive(Component, Reflect, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct WindTextMarker;

#[derive(Component, Reflect, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct BuildingHintToolbar;

#[derive(Event, Debug, Clone, Default)]
pub struct OnRedrawToolbar;

#[derive(Event, Debug, Clone, Default)]
pub struct OnUpdateToolbarButtonDisabledState;

#[derive(Component, Reflect, Debug, Clone, Copy, Eq, PartialEq)]
#[reflect(Component)]
enum ToolbarButtonType {
    Meteor,
    LumberMill,
    ManaForge,
    MinotaurHutch,
    StormMage,
    WaterGolem,
}

#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
struct ToolbarButtonDisabled;

fn handle_on_redraw_toolbar(_trigger: Trigger<OnRedrawToolbar>, mut commands: Commands) {
    commands.run_system_cached(spawn_toolbar);
}

fn handle_disabling_toolbar_buttons(
    _trigger: Trigger<OnUpdateToolbarButtonDisabledState>,
    mut commands: Commands,
    player_resources: Res<PlayerResources>,
    mut buttons: Query<(Entity, &ToolbarButtonType, &mut BackgroundColor)>,
) {
    for (entity, button, mut bg) in &mut buttons {
        if toolbar_button_disabled(*button, &player_resources) {
            commands.entity(entity).insert(ToolbarButtonDisabled);
            bg.0 = SLATE_400.into();
        } else {
            commands.entity(entity).remove::<ToolbarButtonDisabled>();
            bg.0 = SLATE_800.into();
        }
    }
}

fn toolbar_node() -> NodeBuilder {
    NodeBuilder::new()
        .position(PositionType::Absolute)
        .width(Val::Percent(100.0))
        .height(Val::Px(35.0))
        .padding(UiRect::horizontal(Val::Px(10.0)))
        .left(0.0)
        .background(SLATE_800)
        .flex_direction(FlexDirection::Row)
}

fn toolbar_button(
    toolbar: &mut RelatedSpawnerCommands<ChildOf>,
    button_label: impl Into<String>,
    mode: BuildingMode,
    image: Handle<Image>,
    toolbar_type: ToolbarButtonType,
) {
    let label = button_label.into();
    let (hover, selected) = toolbar_data(toolbar_type);

    toolbar
        .spawn((
            NodeBuilder::new()
                // .width(Val::Px(200.0))
                .height(Val::Px(32.0))
                .center_content()
                .background(SLATE_800)
                .margin(UiRect::right(Val::Px(10.0)))
                .padding(UiRect::all(Val::Px(5.0)))
                .build(),
            Button,
            toolbar_type,
            children![
                (
                    NodeBuilder::new()
                        .margin(UiRect::horizontal(Val::Px(5.0)))
                        .build(),
                    ImageNode { image, ..default() }
                ),
                (Text::new(label), TextFont::from_font_size(12.0),)
            ],
        ))
        .observe(
            move |_trigger: Trigger<Pointer<Click>>,
                  mut new_mode: ResMut<BuildingMode>,
                  mut hint: ResMut<BuildTextHint>,
                  mut buttons: Query<
                &mut BackgroundColor,
                (Without<ToolbarButtonDisabled>, With<Button>),
            >| {
                if let Ok(mut bg) = buttons.get_mut(_trigger.target()) {
                    info!("Setting building mode to {mode:?}");
                    *new_mode = mode;
                    hint.0 = selected.clone();
                    bg.0 = SLATE_700.into();
                }
            },
        )
        .observe(
            move |_trigger: Trigger<Pointer<Over>>,
                  mode: Res<BuildingMode>,
                  mut hint: ResMut<BuildTextHint>,
                  mut buttons: Query<
                &mut BackgroundColor,
                (Without<ToolbarButtonDisabled>, With<Button>),
            >| {
                if !matches!(*mode, BuildingMode::None) {
                    return;
                }

                if let Ok(mut bg) = buttons.get_mut(_trigger.target()) {
                    bg.0 = SLATE_950.into();
                }

                hint.0 = hover.clone();
            },
        )
        .observe(
            |_trigger: Trigger<Pointer<Out>>,
             mode: Res<BuildingMode>,
             mut hint: ResMut<BuildTextHint>,
             mut buttons: Query<
                &mut BackgroundColor,
                (Without<ToolbarButtonDisabled>, With<Button>),
            >| {
                if matches!(*mode, BuildingMode::None) {
                    hint.clear();
                }

                if let Ok(mut bg) = buttons.get_mut(_trigger.target()) {
                    bg.0 = SLATE_700.into();
                }
            },
        );
}

#[derive(Component, Reflect)]
#[reflect(Component)]
struct ToolbarUi;

fn _toolbar_buttons(
    toolbar: &mut RelatedSpawnerCommands<ChildOf>,
    in_endless_mode: bool,
    building_assets: &Res<BuildingAssets>,
) {
    #[cfg(debug_assertions)]
    let show_bolt_in_story = true;
    #[cfg(not(debug_assertions))]
    let show_bolt_in_story = false;

    toolbar_button(
        toolbar,
        "Forge",
        BuildingMode::PlaceManaForge,
        building_assets.mana_forge.clone(),
        ToolbarButtonType::ManaForge,
    );

    toolbar_button(
        toolbar,
        "Mill",
        BuildingMode::PlaceLumberMill,
        building_assets.lumber_mill.clone(),
        ToolbarButtonType::LumberMill,
    );

    toolbar_button(
        toolbar,
        "Minotaur",
        BuildingMode::PlaceMinotaur,
        building_assets.minotaur.clone(),
        ToolbarButtonType::MinotaurHutch,
    );

    toolbar_button(
        toolbar,
        "Water Golem",
        BuildingMode::PlaceWaterGolem,
        building_assets.water_golem.clone(),
        ToolbarButtonType::WaterGolem,
    );

    toolbar_button(
        toolbar,
        "Storm Mage",
        BuildingMode::PlaceStormMage,
        building_assets.storm_mage.clone(),
        ToolbarButtonType::StormMage,
    );

    if in_endless_mode || show_bolt_in_story {
        toolbar_button(
            toolbar,
            "Meteor",
            BuildingMode::Meteor,
            building_assets.meteor.clone(),
            ToolbarButtonType::Meteor,
        );
    }
}

fn spawn_toolbar(
    mut commands: Commands,
    requires_city_hall: Option<Res<RequiresCityHall>>,
    maybe_endless_mode: Option<Res<EndlessMode>>,
    resource_assets: Res<ResourceAssets>,
    building_assets: Res<BuildingAssets>,
    previous_toolbars: Query<Entity, With<ToolbarUi>>,
) {
    for previous in &previous_toolbars {
        commands.entity(previous).despawn();
    }

    let requires_city_hall = requires_city_hall.is_some();

    commands
        .spawn((
            ToolbarUi,
            toolbar_node()
                .top(0.0)
                .justify(JustifyContent::SpaceBetween)
                .align_content(AlignContent::SpaceBetween)
                .build(),
            StateScoped(Screen::Gameplay),
        ))
        .with_children(|parent| {
            if requires_city_hall {
                return;
            }

            parent.spawn((
                Name::new("Resource Toolbar"),
                NodeBuilder::new()
                    .height(Val::Px(35.0))
                    .center_content()
                    .build(),
                children![
                    (
                        Node {
                            margin: UiRect::right(Val::Px(5.0)),
                            ..default()
                        },
                        ImageNode {
                            image: resource_assets.resource_icons.clone(),
                            rect: Some(Rect::from_corners(Vec2::ZERO, Vec2::splat(16.0))),
                            ..default()
                        }
                    ),
                    (
                        EnergyTextMarker,
                        NodeBuilder::new()
                            .background(SLATE_800)
                            .center_content()
                            .margin(UiRect::horizontal(Val::Px(5.0)))
                            .build(),
                        Text::new("0"),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        }
                    ),
                    (
                        Node {
                            margin: UiRect::right(Val::Px(5.0)),
                            ..default()
                        },
                        ImageNode {
                            image: resource_assets.resource_icons.clone(),
                            rect: Some(Rect::from_corners(
                                Vec2::new(16.0, 0.0),
                                Vec2::new(32.0, 16.0)
                            )),
                            ..default()
                        },
                    ),
                    (
                        LumberTextMarker,
                        NodeBuilder::new()
                            .background(SLATE_800)
                            .center_content()
                            .margin(UiRect::horizontal(Val::Px(5.0)))
                            .build(),
                        Text::new("0"),
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
                    ),
                ],
            ));

            parent
                .spawn((
                    Name::new("Building button toolbar"),
                    NodeBuilder::new().center_content().build(),
                ))
                .with_children(|toolbar| {
                    _toolbar_buttons(toolbar, maybe_endless_mode.is_some(), &building_assets);
                });
        });

    commands.spawn((
        Name::new("Hint Popup UI"),
        ToolbarUi,
        BuildingHintToolbar,
        GlobalZIndex(3),
        StateScoped(Screen::Gameplay),
        NodeBuilder::new()
            .position(PositionType::Absolute)
            .padding(UiRect::all(Val::Px(10.0)))
            .top(if requires_city_hall { 0.0 } else { 35.0 })
            .right(0.0)
            .width(Val::Px(250.0))
            .background(SLATE_700)
            .build(),
        children![(
            BuildTextMarker,
            Text::new(""),
            TextFont::from_font_size(12.0),
        )],
    ));
}

fn update_toolbar(
    mut commands: Commands,
    player_resource: Res<PlayerResources>,
    wind: Res<WindDirection>,
    mouse: Res<MousePosition>,
    map: Res<GameMap>,
    mut energy_text: Single<
        &mut Text,
        (
            Without<LumberTextMarker>,
            Without<WindTextMarker>,
            With<EnergyTextMarker>,
        ),
    >,
    mut wind_text: Single<
        &mut Text,
        (
            Without<LumberTextMarker>,
            Without<EnergyTextMarker>,
            With<WindTextMarker>,
        ),
    >,
    mut lumber_text: Single<
        &mut Text,
        (
            With<LumberTextMarker>,
            Without<EnergyTextMarker>,
            Without<WindTextMarker>,
        ),
    >,
) {
    let cell_state = if let Some(cell) = map.tile_at_world_pos(mouse.world_pos) {
        if cfg!(debug_assertions) {
            format!(
                "{}, local wind: {} | mouse {:.0},{:.0}",
                *cell, cell.wind, mouse.world_pos.x, mouse.world_pos.y
            )
        } else {
            format!("{}", *cell)
        }
    } else {
        String::new()
    };

    energy_text.0 = format!(
        "{} ({:+})",
        player_resource.mana, player_resource.mana_drain
    );
    lumber_text.0 = format!("{}", player_resource.lumber);
    wind_text.0 = format!(" | WIND: {} | {cell_state}", *wind);

    commands.trigger(OnUpdateToolbarButtonDisabledState);
}

fn update_build_hint_ui(
    maybe_requires_city_hall: Option<Res<RequiresCityHall>>,
    build_text: Res<BuildTextHint>,
    mut toolbar: Single<&mut Visibility, With<BuildingHintToolbar>>,
    mut hint_text: Single<&mut Text, With<BuildTextMarker>>,
) {
    if maybe_requires_city_hall.is_some() {
        **toolbar = Visibility::Visible;
        hint_text.0 = "Click to place your city hall on grass or trees. Take care of this building, if you lose it everything is lost!".into();
        return;
    }

    match &build_text.0 {
        HintMessage::None => {
            **toolbar = Visibility::Hidden;
        }
        HintMessage::Text(text) => {
            **toolbar = Visibility::Visible;
            if hint_text.0 != *text {
                hint_text.0 = text.clone();
            }
        }
        HintMessage::BuildingData {
            name,
            cost,
            details,
        } => {
            let text = format!("{name}\n------\nCosts: {cost}\n\n{details}");
            **toolbar = Visibility::Visible;
            hint_text.0 = text;
        }
    }
}

/// Returns a tuple with (hover message, selected message)
fn toolbar_data(toolbar_type: ToolbarButtonType) -> (HintMessage, HintMessage) {
    match toolbar_type {
        ToolbarButtonType::Meteor => (
            HintMessage::BuildingData {
                name: "Meteor".into(),
                cost: "Free!".into(),
                details: "Hurl some giant rocks and start some fires :D".into(),
            },
            "Click to trigger a meteor bolt, press <space> to stop.".into(),
        ),
        ToolbarButtonType::LumberMill => (
            HintMessage::BuildingData {
                name: "Lumber Mill".into(),
                cost: format!("{LUMBER_MILL_COST_LUMBER} Lumber"),
                details: "Produces 2 Lumber from nearby trees every (1 sec), with a 25% chance to plant a tree instead. Can be placed anywhere, but best in a forest!".into(),
            },
            "Produces Lumber from nearby trees every (0.5 sec), with a 25% chance to plant a tree instead. Can be placed anywhere, but best in a forest!".into()
        ),
        ToolbarButtonType::ManaForge => (
            HintMessage::BuildingData {
                 name: "Mana Forge".into(),
                 cost: format!("{MANA_FORGE_COST_LUMBER} Lumber"),
                 details: "MANA FORGE. Cost: 50 Lumber. Produces Mana (5/sec), required for most other buildings.".into()
            },
             "Click the map to place a forge. Press <space> to cancel placement.".into()
         ),
        ToolbarButtonType::MinotaurHutch => (
            HintMessage::BuildingData {
                name: "Minotaur Hutch".into(),
                cost: format!("{MINOTAUR_COST_MANA} Mana"),
                details: "The minotaur inside consumes 1 mana / sec and turns trees into grass into dirt. Requires Mana Forge nearby.".into()
            },
            "Click the map to place a minotaur camp (close to a mana forge). Press <space> to cancel placement.".into()
         ),
         ToolbarButtonType::StormMage => (
             HintMessage::BuildingData {
                 name: "Storm Mage".into(),
                 cost: format!("{STORM_MAGE_COST_MANA} Mana"),
                 details: "The Storm Mage calls down strong winds consuming 2 mana / sec and push the fire away in one direction. Requires Mana Forge nearby".into(),
             },
             "Click the map to place a storm mage (close to a mana forge). Press <space> to cancel placement or <r> to rotate.".into()
         ),
         ToolbarButtonType::WaterGolem => (
             HintMessage::BuildingData {
                 name: "Water Golem".into(),
                 cost: format!("{WATER_GOLEM_COST_MANA} Mana"),
                 details: "The Water Golem inhabits the area, consuming 4 mana every 2 seconds. When it consumes mana it makes the whole area wetter (less likely to catch fire) and has a 20% chance to quench nearby flames. Requires Mana Forge nearby".into(),
             },
             "Click the map to place a water golem (close to a mana forge). Press <space> to cancel placement".into()
         ),
    }
}

fn toolbar_button_disabled(
    toolbar_type: ToolbarButtonType,
    resources: &Res<PlayerResources>,
) -> bool {
    match toolbar_type {
        ToolbarButtonType::Meteor => false,
        ToolbarButtonType::LumberMill => resources.lumber < LUMBER_MILL_COST_LUMBER,
        ToolbarButtonType::ManaForge => resources.lumber < MANA_FORGE_COST_LUMBER,
        ToolbarButtonType::MinotaurHutch => resources.mana < MINOTAUR_COST_MANA,
        ToolbarButtonType::StormMage => resources.mana < STORM_MAGE_COST_MANA,
        ToolbarButtonType::WaterGolem => resources.mana < WATER_GOLEM_COST_MANA,
    }
}
