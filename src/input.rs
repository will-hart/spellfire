//! Handles mouse input etc

use bevy::{prelude::*, window::PrimaryWindow};

use crate::MainCamera;

pub(super) fn plugin(app: &mut App) {
    app.register_type::<MousePosition>();

    app.add_systems(Startup, setup_mouse_tracking);
    app.add_systems(PreUpdate, track_mouse);
}

/// Tracks the current position of the mouse
#[derive(Debug, Resource, Reflect, Default)]
#[reflect(Resource)]
pub struct MousePosition {
    pub viewport_pos: Vec2,
    pub world_pos: Vec2,
}

fn setup_mouse_tracking(mut commands: Commands) {
    commands.init_resource::<MousePosition>();
}

fn track_mouse(
    mut history: ResMut<MousePosition>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
) {
    let (camera, camera_transform) = *camera;

    if let Some(cursor) = window.cursor_position() {
        history.viewport_pos = cursor;

        if let Ok(ray) = camera.viewport_to_world(camera_transform, cursor) {
            history.world_pos = ray.origin.truncate();
        }
    }
}
