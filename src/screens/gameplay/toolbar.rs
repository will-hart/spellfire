//! Gameplay toolbar code, separated to make the file manageable and maybe
//! so that `cargo fmt` doesn't break constantly

use std::time::Duration;

use bevy::{
    color::palettes::tailwind::{SLATE_400, SLATE_700, SLATE_800, SLATE_950},
    ecs::relationship::RelatedSpawnerCommands,
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
            BuildTextHint, BuildTextMarker, HintMessage,
            building::{BuildingAssets, ResourceAssets},
        },
    },
    theme::node_builder::NodeBuilder,
    wildfire::{GameMap, WindDirection},
};

pub(super) fn plugin(app: &mut App) {
    app.register_type::<ToolbarUi>();
    app.register_type::<ToolbarButtons>();
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
        .add_observer(handle_redraw_toolbar_buttons);
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
pub struct OnRedrawToolbarButtonsOnly;

fn handle_on_redraw_toolbar(_trigger: Trigger<OnRedrawToolbar>, mut commands: Commands) {
    commands.run_system_cached(spawn_toolbar);
}

fn handle_redraw_toolbar_buttons(
    _trigger: Trigger<OnRedrawToolbarButtonsOnly>,
    mut commands: Commands,
    maybe_endless_mode: Option<Res<EndlessMode>>,
    building_assets: Res<BuildingAssets>,
    player_resources: Res<PlayerResources>,
    button_toolbar: Single<Entity, With<ToolbarButtons>>,
) {
    // despawn existing buttons
    commands
        .entity(*button_toolbar)
        .despawn_related::<Children>();

    commands
        .entity(*button_toolbar)
        .with_related_entities(|toolbar| {
            _toolbar_buttons(
                toolbar,
                maybe_endless_mode.is_some(),
                &building_assets,
                &player_resources,
            );
        });
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
    is_disabled: bool,
    button_label: impl Into<String>,
    mode: BuildingMode,
    image: Handle<Image>,
    hover_text: HintMessage,
    selected_text: HintMessage,
) {
    let mode = mode.clone();
    let label = button_label.into();
    let selected = selected_text.clone();
    let hover = hover_text.clone();

    let mut entity_cmds = toolbar.spawn((
        NodeBuilder::new()
            // .width(Val::Px(200.0))
            .height(Val::Px(32.0))
            .center_content()
            .background(if is_disabled { SLATE_400 } else { SLATE_800 })
            .margin(UiRect::right(Val::Px(10.0)))
            .padding(UiRect::all(Val::Px(5.0)))
            .build(),
        Button,
        children![
            (
                NodeBuilder::new()
                    .margin(UiRect::horizontal(Val::Px(5.0)))
                    .build(),
                ImageNode { image, ..default() }
            ),
            (Text::new(label), TextFont::from_font_size(12.0),)
        ],
    ));

    if !is_disabled {
        entity_cmds
            .observe(
                move |_trigger: Trigger<Pointer<Over>>,
                      mode: Res<BuildingMode>,
                      mut hint: ResMut<BuildTextHint>,
                      mut buttons: Query<&mut BackgroundColor, With<Button>>| {
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
                move |_trigger: Trigger<Pointer<Click>>,
                      mut new_mode: ResMut<BuildingMode>,
                      mut hint: ResMut<BuildTextHint>,
                      mut buttons: Query<&mut BackgroundColor, With<Button>>| {
                    info!("Setting building mode to {mode:?}");
                    *new_mode = mode.clone();
                    hint.0 = selected.clone();

                    if let Ok(mut bg) = buttons.get_mut(_trigger.target()) {
                        bg.0 = SLATE_700.into();
                    }
                },
            )
            .observe(
                |_trigger: Trigger<Pointer<Out>>,
                 mode: Res<BuildingMode>,
                 mut hint: ResMut<BuildTextHint>,
                 mut buttons: Query<&mut BackgroundColor, With<Button>>| {
                    if matches!(*mode, BuildingMode::None) {
                        hint.clear();
                    }

                    if let Ok(mut bg) = buttons.get_mut(_trigger.target()) {
                        bg.0 = SLATE_700.into();
                    }
                },
            );
    }
}

#[derive(Component, Reflect)]
#[reflect(Component)]
struct ToolbarUi;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct ToolbarButtons;

fn _toolbar_buttons(
    toolbar: &mut RelatedSpawnerCommands<ChildOf>,
    in_endless_mode: bool,
    building_assets: &Res<BuildingAssets>,
    player_resources: &Res<PlayerResources>,
) {
    #[cfg(debug_assertions)]
    let show_bolt_in_story = true;
    #[cfg(not(debug_assertions))]
    let show_bolt_in_story = false;

    if in_endless_mode || show_bolt_in_story {
        toolbar_button(
            toolbar,
            false,
            "Lightning",
            BuildingMode::Lightning,
            building_assets.lightning.clone(),
            HintMessage::BuildingData {
                name: "Lightning Bolt".into(),
                cost: "Free!".into(),
                details: "Be a pyro and start some fires :D".into(),
            },
            "Click to trigger a lightning bolt, press <space> to stop.".into(),
        );
    }

    toolbar_button(toolbar,
                        player_resources.lumber < 30,
                        "Mill",
                        BuildingMode::PlaceLumberMill,
                        building_assets.lumber_mill.clone(),
                        HintMessage::BuildingData {
                            name: "Lumber Mill".into(),
                            cost: "30 Lumber".into(),
                            details: "Produces Lumber from nearby trees every (0.5 sec), with a 25% chance to plant a tree instead. Can be placed anywhere, but best in a forest!".into(),
                        },
                        "Click the map to place a lumber mill. Press <space> to cancel placement.".into()
                    );

    toolbar_button(toolbar,
                        player_resources.lumber < 50,
                        "Mana Forge",
                         BuildingMode::PlaceManaForge,
                         building_assets.mana_forge.clone(),
                         HintMessage::BuildingData {
                             name: "Mana Forge".into(),
                             cost: "50 Lumber".into(),
                             details: "MANA FORGE. Cost: 50 Lumber. Produces Mana (3/sec), required for most other buildings.".into()
                         },
                        "Click the map to place a forge. Press <space> to cancel placement.".into()
                    );

    toolbar_button(toolbar,
                        player_resources.mana < 40,
                        "Minotaur",
                        BuildingMode::PlaceMinotaur,
                        building_assets.minotaur.clone(),
                        HintMessage::BuildingData {
                            name: "Minotaur Hutch".into(),
                            cost: "40 Mana".into(),
                            details: "The minotaur inside consumes 1 mana / sec and turns trees into grass into dirt. Requires Mana Forge nearby.".into()
                        },
                        "Click the map to place a minotaur camp (close to a mana forge). Press <space> to cancel placement.".into()
                    );
}

fn spawn_toolbar(
    mut commands: Commands,
    requires_city_hall: Option<Res<RequiresCityHall>>,
    maybe_endless_mode: Option<Res<EndlessMode>>,
    player_resources: Res<PlayerResources>,
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
                    ToolbarButtons,
                    NodeBuilder::new().center_content().build(),
                ))
                .with_children(|toolbar| {
                    _toolbar_buttons(
                        toolbar,
                        maybe_endless_mode.is_some(),
                        &building_assets,
                        &player_resources,
                    );
                });
        });

    commands.spawn((
        Name::new("Hint Popup UI"),
        ToolbarUi,
        BuildingHintToolbar,
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
        format!("{}", *cell)
    } else {
        String::new()
    };

    energy_text.0 = format!(
        "{} ({:+})",
        player_resource.mana, player_resource.mana_drain
    );
    lumber_text.0 = format!("{}", player_resource.lumber);
    wind_text.0 = format!(" | WIND: {} | {cell_state}", *wind);

    commands.trigger(OnRedrawToolbarButtonsOnly);
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
