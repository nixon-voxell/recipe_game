use avian3d::prelude::*;
use bevy::prelude::*;

use crate::physics::GameLayer;
use crate::player::player_attack::AttackCooldown;
use crate::player::player_mark::PlayerMark;
use crate::tile::{PlacedBy, TileMap};
use crate::tower::tower_attack::{Health, Tower};
use crate::util::PropagateComponentAppExt;

mod animation;
mod spawner;

pub(super) struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            animation::EnemyAnimationPlugin,
            spawner::EnemySpawnerPlugin,
        ));

        app.propagate_component::<IsEnemy, Children>()
            .add_systems(
                PostUpdate,
                pathfind.after(TransformSystem::TransformPropagate),
            )
            .add_systems(FixedUpdate, enemy_movement)
            .add_systems(
                Update,
                (
                    rotate_to_velocity,
                    (target_reach_respond, attack_tower).chain(),
                ),
            )
            .add_observer(on_path_changed);

        app.register_type::<FinalTarget>().register_type::<Enemy>();
    }
}

fn pathfind(
    mut commands: Commands,
    q_enemies: Query<(&Path, &GlobalTransform, Entity)>,
    q_final_target: Query<&GlobalTransform, With<FinalTarget>>,
    tile_map: Res<TileMap>,
) {
    let Ok(final_target) = q_final_target.single() else {
        return;
    };

    for (enemy_path, transform, entity) in q_enemies.iter() {
        // Pathfind if it's just newly added or the tile map has been updated.
        if enemy_path.is_empty() || tile_map.is_changed() {
            let start_translation = transform.translation();
            let end_translation = final_target.translation();

            debug!(
                "pathfind: {start_translation}, {end_translation}"
            );
            if let Some(path_to_final) = tile_map.pathfind_to(
                &start_translation,
                &end_translation,
                false,
            ) {
                debug!("To target: {:?}", path_to_final);
                commands
                    .entity(entity)
                    .insert((Path(path_to_final), TargetType::Final));
            } else if let Some(path_to_tower) = tile_map.pathfind_to(
                &start_translation,
                &end_translation,
                true,
            ) {
                debug!("To tower: {:?}", path_to_tower);
                commands
                    .entity(entity)
                    .insert((Path(path_to_tower), TargetType::Tower));
            } else {
                warn!("Can't find path for enemy {entity}!");
            }
        }
    }
}

fn on_path_changed(
    trigger: Trigger<OnInsert, Path>,
    mut commands: Commands,
) {
    commands
        .entity(trigger.target())
        .insert(PathIndex(0))
        .remove::<(TargetReached, TargetTower)>();
}

fn enemy_movement(
    mut commands: Commands,
    mut q_enemies: Query<
        (
            &Enemy,
            &Path,
            &mut PathIndex,
            &mut LinearVelocity,
            &Position,
            Entity,
        ),
        Without<TargetReached>,
    >,
) {
    for (
        enemy,
        path,
        mut path_index,
        mut linear_velocity,
        position,
        entity,
    ) in q_enemies.iter_mut()
    {
        let Some(target_position) = path.get_target(&path_index)
        else {
            linear_velocity.0 = Vec3::ZERO;
            commands.entity(entity).insert(TargetReached);
            continue;
        };

        let current_position = position.xz();

        if current_position.distance(target_position) < 0.1 {
            path_index.increment();
        }

        let target_velocity = (target_position - current_position)
            .normalize()
            * enemy.movement_speed;

        linear_velocity.0 =
            Vec3::new(target_velocity.x, 0.0, target_velocity.y);
    }
}

fn target_reach_respond(
    mut commands: Commands,
    q_enemies: Query<
        (&TargetType, &Path, Entity),
        (With<TargetReached>, Without<TargetTower>),
    >,
    q_is_tower: Query<(), With<Tower>>,
    q_children: Query<&Children>,
    q_placed_by: Query<&PlacedBy>,
    tile_map: Res<TileMap>,
    mut player_mark: ResMut<PlayerMark>,
) {
    for (target_type, path, entity) in q_enemies.iter() {
        if *target_type != TargetType::Tower {
            player_mark.0 = player_mark.saturating_sub(0);
            info!("Enemy reached destination, mark decreased!");
            commands.entity(entity).despawn();
            continue;
        }

        let Some(tile_coord) = path.last() else {
            warn!(
                "Cannot get tile coord for enemy {entity}, despawning due to out of bounds?"
            );
            commands.entity(entity).despawn();
            continue;
        };

        // Get the first tower beside it.
        let tower_parent = TileMap::KNIGHT
            .iter()
            .map(move |m| tile_coord + m)
            .filter_map(|coord| {
                // Must be a valid coordinate.
                if TileMap::within_map_range(&coord) == false {
                    return None;
                }

                let index = TileMap::tile_coord_to_tile_idx(
                    &coord.as_uvec2(),
                );
                let tile_meta = tile_map[index]?;

                // Must be occupied for a tower to exist.
                if tile_meta.occupied() {
                    Some(tile_meta.target())
                } else {
                    None
                }
            })
            .next()
            .and_then(|e| {
                q_placed_by
                    .get(e)
                    .ok()
                    .and_then(|p| p.first().copied())
            });

        if let Some(tower_parent) = tower_parent {
            // Find the tower in the child hierarchy
            // (as we are only getting the SceneRoot).
            for child in q_children.iter_descendants(tower_parent) {
                if q_is_tower.contains(child) {
                    info!("Set target tower {tower_parent}");
                    commands.entity(entity).try_insert(TargetTower {
                        root: tower_parent,
                        target: child,
                    });
                    break;
                }
            }
        }
    }
}

fn attack_tower(
    mut commands: Commands,
    mut q_enemies: Query<
        (&TargetTower, &Enemy, &mut AttackCooldown, Entity),
        With<TargetReached>,
    >,
    mut q_healths: Query<&mut Health>,
) {
    for (target_tower, enemy, mut cooldown, entity) in
        q_enemies.iter_mut()
    {
        if let Ok(mut health) = q_healths.get_mut(target_tower.target)
        {
            if cooldown.0 > 0.0 {
                continue;
            }

            health.0 -= enemy.damage;
            cooldown.0 = enemy.attack_cooldown;

            if health.0 <= 0.0 {
                commands.entity(target_tower.root).despawn();
            }
            info!("attacking {}", health.0);
        } else {
            // No more target, find another one.
            commands.entity(entity).remove::<TargetTower>();
        }
    }
}

fn rotate_to_velocity(
    mut q_enemies: Query<
        (&mut Rotation, &LinearVelocity),
        With<Enemy>,
    >,
    time: Res<Time>,
) {
    const ROTATION_RATE: f32 = 10.0;
    let dt = time.delta_secs();

    for (mut rotation, linear_velocity) in q_enemies.iter_mut() {
        // Rotate during movement only.
        if linear_velocity.length() < 0.1 {
            continue;
        }

        let Some(direction) =
            Vec2::new(linear_velocity.x, linear_velocity.z)
                .try_normalize()
        else {
            continue;
        };

        let target_rotation = Quat::from_rotation_y(f32::atan2(
            -direction.x,
            -direction.y,
        ));

        rotation.0 =
            rotation.0.slerp(target_rotation, dt * ROTATION_RATE);
    }
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct FinalTarget;

/// Configuration for the enemy unit.
#[derive(Component, Reflect)]
#[component(immutable)]
#[require(
    IsEnemy,
    Path,
    CollisionEventsEnabled,
    CollisionLayers::new(GameLayer::Enemy, LayerMask::ALL),
    AttackCooldown
)]
#[reflect(Component)]
pub struct Enemy {
    pub movement_speed: f32,
    pub damage: f32,
    pub attack_cooldown: f32,
}

/// Tag component for enemy units.
/// Will be propagated down the hierarchy.
#[derive(Component, Default, Clone, Copy)]
pub struct IsEnemy;

/// The current path of the enemy.
#[derive(Component, Deref, Default)]
#[require(PathIndex)]
#[component(immutable)]
pub struct Path(Vec<IVec2>);

impl Path {
    pub fn get_target(&self, index: &PathIndex) -> Option<Vec2> {
        self.0.get(index.0).map(TileMap::tile_coord_to_world_space)
    }
}

#[derive(Component, Deref, Default)]
pub struct PathIndex(usize);

impl PathIndex {
    pub fn increment(&mut self) {
        self.0 += 1;
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetType {
    Tower,
    Final,
}

#[derive(Component)]
pub struct TargetReached;

#[derive(Component)]
#[component(immutable)]
pub struct TargetTower {
    pub root: Entity,
    pub target: Entity,
}
