use avian3d::prelude::*;
use bevy::color::palettes::tailwind::*;
use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::action::{PlayerAction, TargetAction};
use crate::asset_pipeline::{AssetState, CurrentScene, PrefabAssets};
use crate::camera_controller::{A_RENDER_LAYER, B_RENDER_LAYER};
use crate::character_controller::CharacterController;
use crate::inventory::Inventory;
use crate::inventory::item::{ItemRegistry, ItemType};
use crate::physics::GameLayer;
use crate::player::{PlayerType, QueryPlayers};
use crate::tile::{PlacedBy, PlacedOn, Tile};

pub mod tower_attack;

pub struct TowerPlugin;

impl Plugin for TowerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(tower_attack::TowerAttackPlugin);

        app.add_systems(Startup, setup_preview_cube).add_systems(
            Update,
            (
                tower_placement_and_preview
                    .run_if(in_state(AssetState::Loaded)),
                (enter_placement_mode, exit_placement_mode),
            )
                .chain(),
        );
    }
}

fn setup_preview_cube(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let preview_cube = (
        Mesh3d(meshes.add(Cuboid::new(0.5, 0.5, 0.5))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: GREEN_600.with_alpha(0.4).into(),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })),
        Preview,
        Visibility::Hidden,
    );

    commands.spawn((
        preview_cube.clone(),
        A_RENDER_LAYER,
        PlayerType::A,
    ));
    commands.spawn((preview_cube, B_RENDER_LAYER, PlayerType::B));
}

fn enter_placement_mode(
    mut commands: Commands,
    mut q_players: Query<
        (&Inventory, &TargetAction, Entity),
        (With<CharacterController>, Without<InPlacementMode>),
    >,
    q_actions: Query<&ActionState<PlayerAction>>,
) -> Result {
    for (inventory, target_action, entity) in q_players.iter_mut() {
        let action = q_actions.get(target_action.get())?;

        if action.just_pressed(&PlayerAction::Placement) {
            if let Some(selected_tower) =
                inventory.selected_tower.as_ref()
            {
                // Enter placement mode only if there's a tower to place.
                if inventory
                    .towers()
                    .get(selected_tower)
                    .copied()
                    .unwrap_or(0)
                    > 0
                {
                    commands.entity(entity).insert(InPlacementMode);
                }
            }
        }
    }

    Ok(())
}

fn exit_placement_mode(
    mut commands: Commands,
    mut q_players: Query<
        (&PlayerType, &TargetAction, Entity),
        (With<CharacterController>, With<InPlacementMode>),
    >,
    q_actions: Query<&ActionState<PlayerAction>>,
    mut q_previews: QueryPlayers<&mut Visibility, With<Preview>>,
) -> Result {
    for (player_type, target_action, entity) in q_players.iter_mut() {
        let action = q_actions.get(target_action.get())?;

        if action.just_pressed(&PlayerAction::Cancel) {
            // Exit placement mode.
            commands.entity(entity).remove::<InPlacementMode>();
            *q_previews.get_mut(*player_type)? = Visibility::Hidden;
        }
    }

    Ok(())
}

fn tower_placement_and_preview(
    mut commands: Commands,
    // Find players in placement mode.
    mut q_players: Query<
        (
            &GlobalTransform,
            &PlayerType,
            &mut Inventory,
            &TargetAction,
            Entity,
        ),
        (With<CharacterController>, With<InPlacementMode>),
    >,
    q_tiles: Query<&GlobalTransform, (With<Tile>, Without<PlacedBy>)>,
    mut q_previews: QueryPlayers<
        (&mut Transform, &mut Visibility),
        With<Preview>,
    >,
    q_actions: Query<&ActionState<PlayerAction>>,
    item_registry: ItemRegistry,
    spatial_query: SpatialQuery,
    prefabs: Res<PrefabAssets>,
    gltfs: Res<Assets<Gltf>>,
    current_scene: Res<CurrentScene>,
) -> Result {
    let Some(current_scene) = current_scene.get() else {
        return Ok(());
    };

    for (
        global_transform,
        player_type,
        mut inventory,
        target_action,
        player_entity,
    ) in q_players.iter_mut()
    {
        // In front of the player.
        let target_position = global_transform.translation()
            + global_transform.forward() * 2.0;

        // Create a sphere collider for intersection testing around the camera target
        let interaction_sphere = Collider::sphere(4.0);

        // Check if the tile intersects with the interaction sphere
        let intersections = spatial_query.shape_intersections(
            &interaction_sphere,
            target_position,
            Quat::IDENTITY,
            &SpatialQueryFilter::default(),
        );

        // Find the closest valid tile.
        let mut closest_distance = f32::MAX;
        let mut closest_tile_data = None;

        for tile_entity in intersections {
            let Ok(tile_position) =
                q_tiles.get(tile_entity).map(|t| t.translation())
            else {
                continue;
            };

            let distance_sq =
                target_position.distance_squared(tile_position);

            if distance_sq < closest_distance {
                closest_distance = distance_sq;
                closest_tile_data =
                    Some((tile_position, tile_entity));
            }
        }

        let (mut preview_transform, mut preview_viz) =
            q_previews.get_mut(*player_type)?;

        let Some((tile_position, tile_entity)) = closest_tile_data
        else {
            *preview_viz = Visibility::Hidden;
            continue;
        };

        if q_actions
            .get(target_action.get())?
            .just_pressed(&PlayerAction::Placement)
        {
            // Exit placement mode regardless if placing is a success or not.
            commands
                .entity(player_entity)
                .remove::<InPlacementMode>();

            let Some(selected_tower) =
                inventory.selected_tower.clone()
            else {
                continue;
            };

            let Some(item) = item_registry
                .get_item(&selected_tower)
                .filter(|i| i.item_type == ItemType::Tower)
            else {
                continue;
            };

            if inventory.remove_tower(&selected_tower, 1) == false {
                continue;
            }

            // Spawn the tower.
            commands.spawn((
                SceneRoot(
                    prefabs
                        .get_gltf(item.prefab_name(), &gltfs)
                        .ok_or(format!(
                            "Can't find {selected_tower} prefab!"
                        ))?
                        .default_scene
                        .clone()
                        .ok_or(
                            "Tower prefab have a default scene.",
                        )?,
                ),
                Transform::from_translation(tile_position),
                PlacedOn(tile_entity),
                ChildOf(current_scene),
            ));

            *preview_viz = Visibility::Hidden;
        } else {
            *preview_viz = Visibility::Inherited;
            // Move the preview cube to the tile position.
            preview_transform.translation =
                tile_position + Vec3::Y * 0.25;
        }
    }

    Ok(())
}

/// Tag component for players who are in placement mode.
#[derive(Component)]
pub struct InPlacementMode;

/// Tag component for preview mesh.
#[derive(Component, Clone, Copy)]
pub struct Preview;

/// Projectile component representing a fired projectile
#[derive(Component, Debug)]
#[require(
    RigidBody::Kinematic,
    CollisionEventsEnabled,
    CollisionLayers::new(GameLayer::Projectile, GameLayer::Enemy,)
)]
pub struct Projectile {
    pub velocity: Vec3,
    pub damage: f32,
    pub lifetime: f32,
}
