use avian3d::prelude::*;
use bevy::ecs::component::{ComponentHooks, Immutable, StorageType};
use bevy::prelude::*;

use crate::enemy::{Enemy, IsEnemy, Path};
use crate::physics::GameLayer;
use crate::player::player_attack::AttackCooldown;

use super::Projectile;

pub(super) struct TowerAttackPlugin;

impl Plugin for TowerAttackPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                (
                    check_target_range,
                    find_target,
                    tower_rotation,
                    tower_shooting,
                )
                    .chain(),
                handle_projectile_collisions,
                projectile_movement,
                despawn_on_death,
            ),
        );

        app.register_type::<Tower>().register_type::<MaxHealth>();
    }
}

fn check_target_range(
    mut commands: Commands,
    q_towers: Query<(&Tower, &Target, Entity)>,
    q_global_transforms: Query<&GlobalTransform>,
) -> Result {
    for (tower, target, entity) in q_towers.iter() {
        let tower_position =
            q_global_transforms.get(entity)?.translation();
        let target_position =
            q_global_transforms.get(target.entity())?.translation();

        // Switch target if out of range.
        if target_position.distance(tower_position) > tower.range {
            commands.entity(entity).remove::<Target>();
        }
    }

    Ok(())
}

/// Find and target the best enemy based on [`Path`] length (lower is better).
fn find_target(
    mut commands: Commands,
    q_towers: Query<(&Tower, Entity), Without<Target>>,
    q_collider_ofs: Query<&ColliderOf>,
    q_enemies: Query<(&Path, Entity), With<Enemy>>,
    q_global_transforms: Query<&GlobalTransform>,
    spatial_query: SpatialQuery,
) -> Result {
    for (tower, tower_entity) in q_towers.iter() {
        let tower_position =
            q_global_transforms.get(tower_entity)?.translation();

        // Find enemies in range using shape intersection.
        let detection_sphere = Collider::sphere(tower.range);
        let intersections = spatial_query.shape_intersections(
            &detection_sphere,
            tower_position,
            Quat::IDENTITY,
            &SpatialQueryFilter::default()
                .with_mask(GameLayer::Enemy),
        );

        // Find best target from intersected entities.
        let mut best_target = None;
        let mut least_path = usize::MAX;

        for entity in intersections {
            let Ok((path, enemy_entity)) = q_enemies.get(
                q_collider_ofs
                    .get(entity)
                    .map(|c| c.body)
                    .unwrap_or(entity),
            ) else {
                continue;
            };

            // Check if this enemy has better priority
            if path.len() < least_path {
                least_path = path.len();
                best_target = Some(enemy_entity);
            }
        }

        if let Some(target) = best_target {
            commands.entity(tower_entity).insert(Target(target));
        }
    }

    Ok(())
}

/// Rotate towers to face their targets.
fn tower_rotation(
    mut q_towers: Query<
        (&mut Transform, &GlobalTransform, &Target),
        With<Tower>,
    >,
    q_global_transforms: Query<&GlobalTransform>,
    time: Res<Time>,
) -> Result {
    const ROTATION_SPEED: f32 = 8.0;
    const SNAP_THRESHOLD: f32 = 0.15;

    for (mut transform, global_transform, target) in
        q_towers.iter_mut()
    {
        let tower_position = global_transform.translation();
        let target_position =
            q_global_transforms.get(target.entity())?.translation();

        let Ok(direction) =
            Dir3::new(target_position - tower_position)
        else {
            continue;
        };

        let target_rotation =
            Quat::from_rotation_y(direction.x.atan2(direction.z));

        let angle_diff =
            transform.rotation.angle_between(target_rotation);

        if angle_diff < SNAP_THRESHOLD {
            transform.rotation = target_rotation;
        } else {
            transform.rotation.smooth_nudge(
                &target_rotation,
                ROTATION_SPEED,
                time.delta_secs(),
            );
        }
    }

    Ok(())
}

/// Shoot at current target
fn tower_shooting(
    mut commands: Commands,
    mut q_towers: Query<
        (
            &Transform,
            &GlobalTransform,
            &Tower,
            &mut AttackCooldown,
            &Target,
        ),
        Without<Enemy>,
    >,
    q_enemies: Query<&GlobalTransform, With<Enemy>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) -> Result {
    // Minimum facing accuracy to fire.
    const MIN_FACING_ACCURACY: f32 = 0.9;

    for (transform, global_transform, tower, mut cooldown, target) in
        q_towers.iter_mut()
    {
        if cooldown.0 > 0.0 {
            continue;
        }

        let tower_position = global_transform.translation();
        let target_position =
            q_enemies.get(target.entity())?.translation()
                + Vec3::Y * 0.5;

        // Check if tower is facing the target
        let tower_forward = -transform.forward();
        let target_direction =
            (target_position - tower_position).normalize();
        let facing_dot = tower_forward.dot(target_direction);

        if facing_dot < MIN_FACING_ACCURACY {
            continue;
        }

        let projectile_start = tower_position + Vec3::Y * 0.5;
        let direction =
            (target_position - projectile_start).normalize();

        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.1))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.2, 0.8, 1.0),
                emissive: LinearRgba::rgb(0.5, 2.0, 3.0),
                ..default()
            })),
            Transform::from_translation(projectile_start),
            Collider::sphere(0.1),
            Projectile {
                velocity: direction * tower.projectile_speed,
                damage: tower.damage,
                lifetime: 3.0,
            },
        ));

        cooldown.0 = tower.attack_cooldown;
    }

    Ok(())
}

/// Handle projectile collisions using physics system.
fn handle_projectile_collisions(
    mut commands: Commands,
    mut collision_events: EventReader<CollisionStarted>,
    q_projectiles: Query<&Projectile>,
    q_collider_ofs: Query<&ColliderOf>,
    q_is_enemy: Query<(), With<IsEnemy>>,
    mut q_healths: Query<&mut Health>,
) {
    for CollisionStarted(entity1, entity2) in collision_events.read()
    {
        // Check if one is projectile, other is enemy
        let (projectile_entity, enemy_entity) = if q_projectiles
            .contains(*entity1)
            && q_is_enemy.contains(*entity2)
        {
            (*entity1, *entity2)
        } else if q_projectiles.contains(*entity2)
            && q_is_enemy.contains(*entity1)
        {
            (*entity2, *entity1)
        } else {
            continue;
        };

        // Get projectile data and apply damage
        if let Ok(projectile) = q_projectiles.get(projectile_entity) {
            let enemy_entity = q_collider_ofs
                .get(enemy_entity)
                .map(|c| c.body)
                .unwrap_or(enemy_entity);

            if let Ok(mut health) = q_healths.get_mut(enemy_entity) {
                health.0 -= projectile.damage;
            }

            // Despawn projectile after hit
            commands.entity(projectile_entity).despawn();
        }
    }
}

fn despawn_on_death(
    mut commands: Commands,
    q_healths: Query<(&Health, Entity), Changed<Health>>,
) {
    for (health, entity) in q_healths.iter() {
        if health.0 <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

/// Move projectiles.
fn projectile_movement(
    mut commands: Commands,
    mut q_projectiles: Query<(
        &mut Transform,
        &mut Projectile,
        Entity,
    )>,
    time: Res<Time>,
) {
    let delta_time = time.delta_secs();

    for (mut transform, mut projectile, projectile_entity) in
        q_projectiles.iter_mut()
    {
        // Update lifetime
        projectile.lifetime -= delta_time;
        if projectile.lifetime <= 0.0 {
            commands.entity(projectile_entity).despawn();
            continue;
        }

        // Move projectile
        transform.translation += projectile.velocity * delta_time;
    }
}

/// Tower component with stats only.
#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
#[require(
    AttackCooldown,
    CollisionLayers::new(GameLayer::Tower, {
        let mut layer = LayerMask::ALL;
        layer.remove(GameLayer::Enemy);
        layer
    })
)]
pub struct Tower {
    pub range: f32,
    pub damage: f32,
    pub attack_cooldown: f32,
    pub projectile_speed: f32,
}

/// Health component for entities that can take damage
#[derive(Reflect, Debug)]
#[reflect(Component)]
pub struct MaxHealth(pub f32);

impl Component for MaxHealth {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    type Mutability = Immutable;

    /// Setup camera tag: [`Health`] based on [`MaxHealth`].
    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_add(|mut world, hook| {
            let entity = hook.entity;
            let max_health =
                world.get::<Self>(hook.entity).unwrap().0;

            world
                .commands()
                .entity(entity)
                .insert(Health(max_health));
        });
    }
}

#[derive(Component, Deref, DerefMut, Debug)]
pub struct Health(pub f32);

/// Relationship components for tower targeting
#[derive(Component, Deref, Debug)]
#[relationship(relationship_target = TargetsOf)]
pub struct Target(Entity);

#[derive(Component, Deref, Default, Debug)]
#[relationship_target(relationship = Target)]
pub struct TargetsOf(Vec<Entity>);
