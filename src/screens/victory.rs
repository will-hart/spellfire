//! The victory screen

use bevy::prelude::*;

use crate::screens::{NextStoryLevel, Screen, StoryModeLevel};
use crate::theme::widget;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::LevelWon), spawn_level_victory_screen);
}

fn spawn_level_victory_screen(
    mut commands: Commands,
    story_level: Res<StoryModeLevel>,
    mut next_level: ResMut<NextStoryLevel>,
) {
    next_level.0 = story_level.level_number + 1;

    commands
        .spawn((
            widget::ui_root("Level Victory Menu"),
            StateScoped(Screen::LevelWon),
            children![
                widget::header("VICTORY!"),
                (
                    Text::new("Your have successfully defended your City Hall!"),
                    TextFont::from_font_size(24.0),
                ),
            ],
        ))
        .with_children(|parent| {
            parent.spawn((widget::button(
                "Next Level",
                |_trigger: Trigger<Pointer<Click>>, mut next: ResMut<NextState<Screen>>| {
                    next.set(Screen::Gameplay);
                },
            ),));
            parent.spawn(widget::button(
                "Flee to the menu",
                |_trigger: Trigger<Pointer<Click>>, mut next: ResMut<NextState<Screen>>| {
                    next.set(Screen::Title);
                },
            ));
        });
}
