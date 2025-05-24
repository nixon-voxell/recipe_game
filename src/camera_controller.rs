use bevy::core_pipeline::Skybox;
use bevy::core_pipeline::bloom::Bloom;
use bevy::core_pipeline::smaa::Smaa;
use bevy::core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy::pbr::ScreenSpaceAmbientOcclusion;
use bevy::prelude::*;

use crate::ui::world_space::WorldSpaceUiCamera;

pub(super) struct CameraControllerPlugin;

impl Plugin for CameraControllerPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<CameraSnap>();

        app.add_systems(Startup, setup_game_camera_and_environment)
            .add_systems(Update, snap_camera)
            .add_observer(setup_directional_light);
    }
}

fn snap_camera(
    mut q_camera: Query<&mut Transform, With<GameCamera>>,
    q_camera_snaps: Query<&GlobalTransform, Added<CameraSnap>>,
) -> Result {
    let mut camera_transform = q_camera.single_mut()?;

    for snap_global_transform in q_camera_snaps.iter() {
        *camera_transform = snap_global_transform.compute_transform();
    }

    Ok(())
}

fn setup_game_camera_and_environment(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    commands.spawn((
        Camera3d::default(),
        Camera {
            hdr: true,
            ..default()
        },
        Tonemapping::None,
        Bloom::NATURAL,
        DebandDither::Enabled,
        Msaa::Off,
        ScreenSpaceAmbientOcclusion::default(),
        Smaa::default(),
        Skybox {
            image: asset_server.load("pisa_diffuse_rgb9e5_zstd.ktx2"),
            brightness: 1000.0,
            ..default()
        },
        EnvironmentMapLight {
            diffuse_map: asset_server
                .load("pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server
                .load("pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 1000.0,
            ..default()
        },
        GameCamera,
        WorldSpaceUiCamera,
    ));
}

fn setup_directional_light(
    trigger: Trigger<OnAdd, DirectionalLight>,
    mut q_lights: Query<&mut DirectionalLight>,
) -> Result {
    let mut light = q_lights.get_mut(trigger.target())?;
    light.shadows_enabled = true;

    Ok(())
}

/// Snaps camera to the [`GlobalTransform`] of this entity on [add][Added].
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct CameraSnap;

#[derive(Component)]
pub struct GameCamera;
