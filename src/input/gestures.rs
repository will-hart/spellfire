use bevy::{input::mouse::MouseWheel, prelude::*};

use crate::{MainCamera, Pause, input::MousePosition};

pub(super) fn plugin(app: &mut App) {
    app.register_type::<MouseWheelGestures>();
    app.register_type::<MousePanGestures>();

    app.insert_resource(MousePanGestures {
        sensitivity: 1.0,
        current: GestureType::None,
    });

    app.insert_resource(MouseWheelGestures {
        sensitivity: 0.1,
        delta: 0.0,
        min_scale: 0.4,
        max_scale: 1.3,
    });

    app.add_systems(
        Update,
        (update_pan_gestures, update_zoom_gestures).distributive_run_if(in_state(Pause(false))),
    );

    app.add_systems(
        Update,
        (handle_camera_pan_gestures, handle_camera_zoom_gestures)
            .distributive_run_if(in_state(Pause(false))),
    );
}

fn update_pan_gestures(
    time: Res<Time>,
    position: Res<MousePosition>,
    mut gestures: ResMut<MousePanGestures>,
) {
    gestures.update(&position, time.elapsed_secs());
}

fn update_zoom_gestures(
    time: Res<Time>,
    mut wheel_events: EventReader<MouseWheel>,
    mut gestures: ResMut<MouseWheelGestures>,
) {
    gestures.update(&mut wheel_events, time.elapsed_secs());
}

fn handle_camera_zoom_gestures(
    wheel_gestures: Res<MouseWheelGestures>,
    mut camera: Single<&mut Projection, With<MainCamera>>,
) {
    #[expect(clippy::collapsible_if, reason = "don't always want to use nightly")]
    if let GestureType::Pinch { unscaled_delta } = wheel_gestures.current() {
        if let Projection::Orthographic(ref mut proj) = **camera {
            proj.scale = (proj.scale + unscaled_delta * proj.scale)
                .clamp(wheel_gestures.min_scale, wheel_gestures.max_scale);
        }
    }
}

fn handle_camera_pan_gestures(
    pan_gestures: Res<MousePanGestures>,
    mut camera: Single<(&mut Transform, &Projection), With<MainCamera>>,
) {
    if let GestureType::Pan { unscaled_delta } = pan_gestures.current() {
        let (ref mut tx, Projection::Orthographic(proj)) = *camera else {
            warn!("Unable to find orthographic projection for camera in pan_gestures. Aborting");
            return;
        };

        tx.translation += (unscaled_delta * proj.scale).extend(0.0);
    }
}

/// The different gestures that are available for camera controls
#[derive(PartialEq, Default, Reflect, Debug, Clone, Copy)]
pub enum GestureType {
    #[default]
    None,
    Pan {
        /// the raw distance the camera was dragged, unscaled by camera projection
        unscaled_delta: Vec2,
    },
    Pinch {
        // the raw pinch delta, unscaled by camera projection
        unscaled_delta: f32,
    },
    /// Pinch gesture has ended
    PinchCancelled,
    /// Pan gesture started but not enough time has elapsed yet
    PanInsufficientTime,
    /// Pan gesture has started but has not dragged a sufficient distance yet
    PanInsufficientDistance,
}

/// Different gesture types
pub trait GestureTracker<T> {
    fn current(&self) -> GestureType;
    fn update(&mut self, tracker: T, elapsed_game_seconds: f32);
}

#[derive(Debug, Resource, Reflect)]
#[reflect(Resource)]
pub struct MouseWheelGestures {
    pub sensitivity: f32,
    pub min_scale: f32,
    pub max_scale: f32,
    pub delta: f32,
}

impl GestureTracker<&'_ mut EventReader<'_, '_, MouseWheel>> for MouseWheelGestures {
    fn current(&self) -> GestureType {
        if self.delta.abs() > 0.5 * self.sensitivity {
            GestureType::Pinch {
                unscaled_delta: self.delta,
            }
        } else {
            GestureType::None
        }
    }

    fn update(
        &mut self,
        mouse_wheel_events: &mut EventReader<MouseWheel>,
        _elapsed_game_seconds: f32,
    ) {
        self.delta = 0.0;

        for event in mouse_wheel_events.read() {
            self.delta = -event.y.signum() * self.sensitivity;
        }
    }
}

#[derive(Debug, Resource, Reflect, Default)]
#[reflect(Resource)]
pub struct MousePanGestures {
    pub sensitivity: f32,
    pub current: GestureType,
}

impl GestureTracker<&'_ Res<'_, MousePosition>> for MousePanGestures {
    fn current(&self) -> GestureType {
        self.current
    }

    fn update(&mut self, history: &Res<MousePosition>, _elapsed_game_seconds: f32) {
        if history.primary_pressed && history.viewport_delta.length() > self.sensitivity {
            self.current = GestureType::Pan {
                unscaled_delta: Vec2::new(history.viewport_delta.x, -history.viewport_delta.y),
            };
        } else {
            self.current = GestureType::None;
        }
    }
}
