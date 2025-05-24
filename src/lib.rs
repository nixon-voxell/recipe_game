use avian3d::prelude::*;
use bevy::prelude::*;

mod action;
mod asset_pipeline;
mod camera_controller;
mod character_controller;
mod interaction;
mod physics;
mod player;
mod ui;

pub struct AppPlugin;

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            bevy_framepace::FramepacePlugin,
            bevy_skein::SkeinPlugin::default(),
            PhysicsPlugins::default(),
            // PhysicsPickingPlugin,
            PhysicsDebugPlugin::default(),
        ))
        .add_plugins((
            action::ActionPlugin,
            ui::UiPlugin,
            physics::PhysicsPlugin,
            asset_pipeline::AssetPipelinePlugin,
            camera_controller::CameraControllerPlugin,
            character_controller::MovementPlugin,
            interaction::InteractionPlugin,
            player::PlayerPlugin,
        ));

        #[cfg(feature = "dev")]
        app.add_plugins((
            bevy_inspector_egui::bevy_egui::EguiPlugin {
                enable_multipass_for_primary_context: true,
            },
            bevy_inspector_egui::quick::WorldInspectorPlugin::new(),
        ));
    }
}
