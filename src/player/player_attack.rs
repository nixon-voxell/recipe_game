use crate::action::{PlayerAction, TargetAction};
use crate::camera_controller::split_screen::{
    CameraType, QueryCameras,
};
use crate::enemy::IsEnemy;
use crate::physics::GameLayer;
use crate::player::PlayerType;
use crate::tower::Projectile;
use avian3d::prelude::*;
use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;

pub(super) struct PlayerAttackPlugin;

impl Plugin for PlayerAttackPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (player_shooting, update_cooldowns));
        app.register_type::<PlayerWeapon>();
    }
}

fn update_cooldowns(
    mut q_cooldowns: Query<&mut AttackCooldown>,
    time: Res<Time>,
) {
    for mut cooldown in q_cooldowns.iter_mut() {
        cooldown.0 -= time.delta_secs();
    }
}

fn player_shooting(
    mut commands: Commands,
    mut q_player_weapons: Query<(
        &GlobalTransform,
        &PlayerType,
        &PlayerWeapon,
        &TargetAction,
        &mut AttackCooldown,
    )>,
    q_cameras: QueryCameras<&GlobalTransform>,
    q_actions: Query<&ActionState<PlayerAction>>,
    q_enemies: Query<&GlobalTransform, With<IsEnemy>>,
    spatial_query: SpatialQuery,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (
        player_transform,
        player_type,
        weapon,
        target_action,
        mut cooldown,
    ) in q_player_weapons.iter_mut()
    {
        // Check cooldown
        if cooldown.0 > 0.0 {
            continue;
        }

        let Ok(action) = q_actions.get(target_action.get()) else {
            continue;
        };
        if !action.pressed(&PlayerAction::Attack) {
            continue;
        }

        let camera_type = match player_type {
            PlayerType::A => CameraType::A,
            PlayerType::B => CameraType::B,
        };
        let Ok(camera_transform) = q_cameras.get(camera_type) else {
            continue;
        };

        let camera_position = camera_transform.translation();
        let camera_forward = camera_transform.forward();

        // Spawn projectile from player
        let projectile_start =
            player_transform.translation() + Vec3::Y * 0.65;

        // Get player's forward direction
        let player_forward = player_transform.forward();

        // Perform a shape cast to detect enemies in front of the player
        let detection_shape = Collider::sphere(1.7);
        let shape_cast_config = ShapeCastConfig {
            max_distance: 50.0,
            ..ShapeCastConfig::DEFAULT
        };

        let shape_hit = spatial_query.cast_shape(
            &detection_shape,
            camera_position,
            Quat::IDENTITY,
            Dir3::new(*camera_forward).unwrap(),
            &shape_cast_config,
            &SpatialQueryFilter::default()
                .with_mask(GameLayer::Enemy),
        );

        // Check if enemy was hit
        let target_direction = if let Some(hit) = shape_hit {
            if let Ok(enemy_transform) = q_enemies.get(hit.entity) {
                // Aim from projectile spawn point to the detected enemy
                (enemy_transform.translation() - projectile_start)
                    .normalize()
            } else {
                // No enemy found, shoot in player's facing direction
                *player_forward
            }
        } else {
            // No hit, shoot in player's facing direction
            *player_forward
        };

        // Spawn projectile using weapon stats
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.06))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.8, 0.2),
                emissive: LinearRgba::rgb(2.0, 1.5, 0.5),
                ..default()
            })),
            Transform::from_translation(
                projectile_start + player_transform.forward() * 0.5,
            ),
            Collider::sphere(0.08),
            Projectile {
                velocity: target_direction * weapon.projectile_speed,
                damage: weapon.damage,
                lifetime: weapon.projectile_lifetime,
            },
        ));

        // Reset cooldown
        cooldown.0 = weapon.attack_cooldown;
    }
}

/// Player weapon component with configurable stats.
#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
#[require(AttackCooldown)]
pub struct PlayerWeapon {
    pub damage: f32,
    pub attack_cooldown: f32,
    pub projectile_speed: f32,
    pub projectile_lifetime: f32,
}

/// Player attack cooldown.
#[derive(Component, Deref, DerefMut, Debug, Default)]
pub struct AttackCooldown(pub f32);
