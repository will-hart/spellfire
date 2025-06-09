//! The game over screen

use bevy::prelude::*;

use crate::asset_tracking::LoadResource;
use crate::audio::sound_effect;
use crate::screens::Screen;
use crate::theme::widget;

pub(super) fn plugin(app: &mut App) {
    app.register_type::<GameOverAssets>();
    app.load_resource::<GameOverAssets>();

    app.add_systems(OnEnter(Screen::GameOver), spawn_game_over_screen);
}

fn spawn_game_over_screen(mut commands: Commands, game_over_assets: Res<GameOverAssets>) {
    commands.spawn(sound_effect(game_over_assets.defeated.clone()));

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
            parent.spawn((widget::button(
                "Try Again?",
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

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct GameOverAssets {
    #[dependency]
    defeated: Handle<AudioSource>,
}

impl FromWorld for GameOverAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        Self {
            defeated: assets.load("audio/sound_effects/you_are_defeated.ogg"),
        }
    }
}
