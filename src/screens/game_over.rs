//! The game over screen

use bevy::prelude::*;

use crate::screens::{EndlessMode, Screen};
use crate::theme::widget;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::GameOver), spawn_game_over_screen);
}

fn spawn_game_over_screen(mut commands: Commands, maybe_endless_mode: Option<Res<EndlessMode>>) {
    commands
        .spawn((
            widget::ui_root("Game Over Menu"),
            StateScoped(Screen::GameOver),
            children![
                widget::header("GAME OVER"),
                (
                    Text::new("Your City Hall has succumbed to the flames."),
                    TextFont::from_font_size(24.0),
                ),
            ],
        ))
        .with_children(|parent| {
            if maybe_endless_mode.is_some() {
                parent.spawn((widget::button(
                    "Try Again?",
                    |_trigger: Trigger<Pointer<Click>>, mut next: ResMut<NextState<Screen>>| {
                        next.set(Screen::Gameplay);
                    },
                ),));
            }
            parent.spawn(widget::button(
                "Flee to the menu",
                |_trigger: Trigger<Pointer<Click>>, mut next: ResMut<NextState<Screen>>| {
                    next.set(Screen::Title);
                },
            ));
        });
}
