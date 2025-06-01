//! The screen state for the main gameplay.

use bevy::{input::common_conditions::input_just_pressed, prelude::*, ui::Val::*};

use crate::{
    Pause,
    demo::level::spawn_level,
    input::MousePosition,
    menus::Menu,
    screens::Screen,
    wildfire::{OnLightningStrike, OnSpawnMap},
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Gameplay), spawn_level);

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
    map: Option<Res<OnSpawnMap>>,
) {
    if let Some(map) = map {
        // convert to tile coords
        let offset_x = map.size.x as f32 * map.sprite_size * 0.5;
        let offset_y = map.size.y as f32 * map.sprite_size * 0.5;

        let x = ((mouse_pos.world_pos.x + offset_x) / map.sprite_size).floor() as i32;
        let y = ((mouse_pos.world_pos.y + offset_y) / map.sprite_size).floor() as i32;

        commands.trigger(OnLightningStrike(IVec2::new(x, y)));
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
