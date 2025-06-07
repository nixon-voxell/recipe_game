use bevy::prelude::*;

mod action;
mod asset_pipeline;
mod camera_controller;
mod character_controller;
mod enemy;
mod interaction;
mod inventory;
mod machine;
mod physics;
mod player;
mod tile;
mod tower;
mod ui;
mod util;

pub struct AppPlugin;

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            bevy_framepace::FramepacePlugin,
            bevy_skein::SkeinPlugin::default(),
        ))
        .add_plugins((
            action::ActionPlugin,
            ui::UiPlugin,
            physics::PhysicsPlugin,
            asset_pipeline::AssetPipelinePlugin,
            camera_controller::CameraControllerPlugin,
            character_controller::CharacterControllerPlugin,
            interaction::InteractionPlugin,
            inventory::InventoryPlugin,
            player::PlayerPlugin,
            machine::MachinePlugin,
            tower::TowerPlugin,
            tile::TilePlugin,
            enemy::EnemyPlugin,
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
