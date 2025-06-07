use avian3d::prelude::*;
use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::action::{PlayerAction, RequireAction, TargetAction};
use crate::camera_controller::split_screen::{
    CameraType, QueryCameras,
};
use crate::inventory::Inventory;
use crate::physics::GameLayer;
use crate::player::PlayerType;

mod animation;

/// Plugin that sets up kinematic character movement
pub(super) struct CharacterControllerPlugin;

impl Plugin for CharacterControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(animation::CharacterAnimationPlugin);

        app.add_systems(
            FixedUpdate,
            (
                check_grounded,
                apply_gravity,
                movement,
                jump,
                rotate_to_velocity,
                movement_damping,
            )
                .chain(),
        )
        .add_systems(
            PhysicsSchedule,
            kinematic_controller_collisions
                .in_set(NarrowPhaseSet::Last),
        );

        app.register_type::<CharacterController>();
    }
}

#[derive(Deref)]
struct GroundCastShape(Collider);

impl Default for GroundCastShape {
    fn default() -> Self {
        Self(Collider::sphere(0.1))
    }
}

/// Check grounded state by raycasting downwards.
fn check_grounded(
    mut q_characters: Query<(
        &GlobalTransform,
        &CharacterController,
        &mut IsGrounded,
    )>,
    spatial_query: SpatialQuery,
    cast_shape: Local<GroundCastShape>,
) {
    const MAX_DIST: f32 = 0.3;
    const SHAPE_CAST_CONFIG: ShapeCastConfig = ShapeCastConfig {
        max_distance: MAX_DIST,
        ..ShapeCastConfig::DEFAULT
    };
    const RAY_DIRECTION: Dir3 = Dir3::NEG_Y;

    for (global_transform, character, mut is_grounded) in
        q_characters.iter_mut()
    {
        let char_pos = global_transform.translation();

        let ray_origin = char_pos + Vec3::Y * 0.2;

        let mut mask = LayerMask::ALL;
        mask.remove(GameLayer::Player);

        // Exclude the character's own entity from the raycast
        let filter = SpatialQueryFilter::default().with_mask(mask);

        if let Some(hit) = spatial_query.cast_shape(
            &cast_shape,
            ray_origin,
            Quat::IDENTITY,
            RAY_DIRECTION,
            &SHAPE_CAST_CONFIG,
            &filter,
        ) {
            let slope_angle = hit.normal1.angle_between(Vec3::Y);

            // Check if the normal is valid and surface is walkable
            if slope_angle.is_finite()
                && slope_angle <= character.max_slope_angle
            {
                is_grounded.set_if_neq(IsGrounded(true));
            } else {
                is_grounded.set_if_neq(IsGrounded(false));
            }
        } else {
            is_grounded.set_if_neq(IsGrounded(false));
        }
    }
}

fn jump(
    mut q_characters: Query<(
        &mut LinearVelocity,
        &mut IsGrounded,
        &CharacterController,
        &TargetAction,
    )>,
    q_actions: Query<&ActionState<PlayerAction>>,
) {
    for (
        mut linear_velocity,
        mut is_grounded,
        character,
        target_action,
    ) in q_characters.iter_mut()
    {
        let Ok(action) = q_actions.get(target_action.get()) else {
            continue;
        };

        if is_grounded.0 && action.just_pressed(&PlayerAction::Jump) {
            linear_velocity.0.y = character.jump_impulse;
            is_grounded.set_if_neq(IsGrounded(false));
        }
    }
}

fn rotate_to_velocity(
    mut q_characters: Query<
        (&mut Rotation, &LinearVelocity, &IsMoving),
        With<CharacterController>,
    >,
    time: Res<Time>,
) {
    const ROTATION_RATE: f32 = 10.0;
    let dt = time.delta_secs();

    for (mut rotation, linear_velocity, is_moving) in
        q_characters.iter_mut()
    {
        // Rotate during movement only.
        if is_moving.0 == false {
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

/// Applies gravity to vertical velocity
fn apply_gravity(
    mut q_characters: Query<(
        &mut LinearVelocity,
        &CharacterController,
        &IsGrounded,
    )>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    for (mut linear_velocity, character, is_grounded) in
        q_characters.iter_mut()
    {
        if is_grounded.0 == false {
            linear_velocity.0 += character.gravity * dt;
        }
    }
}

/// Handles movement and jumping
fn movement(
    time: Res<Time>,
    q_cameras: QueryCameras<&GlobalTransform>,
    q_actions: Query<&ActionState<PlayerAction>>,
    mut q_characters: Query<(
        &CharacterController,
        &mut LinearVelocity,
        &mut IsMoving,
        &TargetAction,
        &PlayerType,
    )>,
) {
    let dt = time.delta_secs_f64() as f32;

    for (
        character,
        mut linear_velocity,
        mut is_moving,
        target_action,
        player_type,
    ) in q_characters.iter_mut()
    {
        // Get camera transform.
        let Ok(cam_global_transform) =
            q_cameras.get(match player_type {
                PlayerType::A => CameraType::A,
                PlayerType::B => CameraType::B,
            })
        else {
            return;
        };

        let cam_forward = cam_global_transform.forward();
        let cam_forward = Vec2::new(cam_forward.x, cam_forward.z)
            .normalize_or_zero();
        let cam_left = cam_global_transform.left();
        let cam_left =
            Vec2::new(cam_left.x, cam_left.z).normalize_or_zero();

        let Ok(action) = q_actions.get(target_action.get()) else {
            warn!("No `InputMap` found for player: {player_type:?}");
            continue;
        };

        let movement = action
            .clamped_axis_pair(&PlayerAction::Move)
            .clamp_length_max(1.0);
        if movement.length_squared() <= f32::EPSILON {
            // Ignore movement when it's negligible.
            is_moving.set_if_neq(IsMoving(false));
            continue;
        }

        is_moving.set_if_neq(IsMoving(true));

        let world_move =
            (cam_forward * movement.y) - (cam_left * movement.x);
        let world_move = Vec3::new(world_move.x, 0.0, world_move.y);

        // Only allow sprinting if grounded
        // let can_sprint = *sprint && is_grounded.0;
        let is_sprinting = false;

        // Apply acceleration * sprint factor
        let factor = if is_sprinting { 2.0 } else { 1.0 };
        let acceleration = character.acceleration;
        linear_velocity.0 +=
            world_move * (acceleration * dt * factor);

        // Clamp horizontal speed (only sprint speed if grounded)
        let max_speed = match is_sprinting {
            true => character.max_sprint,
            false => character.max_walk,
        };

        let horiz =
            Vec2::new(linear_velocity.0.x, linear_velocity.0.z);
        if horiz.length() > max_speed {
            let clamped = horiz.normalize() * max_speed;
            linear_velocity.0.x = clamped.x;
            linear_velocity.0.z = clamped.y;
        }
    }
}

/// Applies damping to horizontal movement
fn movement_damping(
    mut q_characters: Query<(
        &mut LinearVelocity,
        &CharacterController,
    )>,
) {
    for (mut linear_velocity, character) in q_characters.iter_mut() {
        // Damping cannot go above 1.0.
        let damping = character.damping.min(1.0);
        // Apply damping directly to physics velocity, except gravity.
        linear_velocity.x *= damping;
        linear_velocity.z *= damping;
    }
}

/// Handles collisions for kinematic character controllers
fn kinematic_controller_collisions(
    collisions: Collisions,
    bodies: Query<&RigidBody>,
    collider_rbs: Query<&ColliderOf, Without<Sensor>>,
    mut q_characters: Query<
        (
            &mut Position,
            &mut LinearVelocity,
            &CharacterController,
            &mut IsGrounded,
        ),
        (With<RigidBody>, With<CharacterController>),
    >,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for contacts in collisions.iter() {
        // Pull out the two bodies
        let Ok([&ColliderOf { body: a }, &ColliderOf { body: b }]) =
            collider_rbs
                .get_many([contacts.collider1, contacts.collider2])
        else {
            continue;
        };

        // Figure out which one is me
        let (entity, is_first, other) = if q_characters.get(a).is_ok()
        {
            (a, true, b)
        } else if q_characters.get(b).is_ok() {
            (b, false, a)
        } else {
            continue;
        };

        // Only do kinematic
        if !bodies.get(entity).unwrap().is_kinematic() {
            continue;
        }

        let (mut pos, mut linear_velocity, ctl, mut is_grounded) =
            q_characters.get_mut(entity).unwrap();

        // Detect if the other collider is dynamic
        let other_dynamic =
            bodies.get(other).is_ok_and(|rb| rb.is_dynamic());

        for manifold in &contacts.manifolds {
            let normal = if is_first {
                -manifold.normal
            } else {
                manifold.normal
            };

            // Push out of penetration and handle velocity
            let mut deepest = 0.0;
            for pt in &manifold.points {
                if pt.penetration > 0.0 {
                    let is_ground = normal.y > 0.7;
                    let is_jumping = linear_velocity.y > 0.0;

                    // Apply penetration correction unless jumping into ceiling
                    if !(is_ground && is_jumping) {
                        pos.0 += normal * pt.penetration;
                    }

                    // Cancel all vertical velocity when grounded
                    if is_ground {
                        linear_velocity.y = 0.0;
                        is_grounded.0 = true;
                    }
                }
                deepest = f32::max(deepest, pt.penetration);
            }

            // Skip dynamic collisions
            if other_dynamic {
                continue;
            }

            let slope_angle = normal.angle_between(Vec3::Y).abs();
            let can_climb = slope_angle <= ctl.max_slope_angle;

            if deepest > 0.0 {
                if can_climb {
                    // slope-snap logic
                    let dir_xz = normal
                        .reject_from_normalized(Vec3::Y)
                        .normalize_or_zero();
                    let vel_xz = linear_velocity.dot(dir_xz);
                    let max_y = -vel_xz * slope_angle.tan();
                    linear_velocity.y = linear_velocity.y.max(max_y);
                } else {
                    // Wall-slide: zero out velocity into the wall
                    let into = linear_velocity.dot(normal);
                    if into < 0.0 {
                        linear_velocity.0 -= normal * into;
                    }
                }
            } else {
                // Speculative contact
                let n_speed = linear_velocity.dot(normal);
                if n_speed < 0.0 {
                    let impulse = (n_speed - (deepest / dt)) * normal;
                    if can_climb {
                        linear_velocity.y -= impulse.y.min(0.0);
                    } else {
                        let mut i = impulse;
                        i.y = i.y.max(0.0);
                        linear_velocity.0 -= i;
                    }
                }
            }
        }
    }
}

#[derive(Component, Deref, DerefMut, Default, PartialEq, Eq)]
pub struct IsGrounded(pub bool);

#[derive(Component, Deref, DerefMut, Default, PartialEq, Eq)]
pub struct IsMoving(pub bool);

/// Marker for kinematic character bodies
#[derive(Component, Reflect)]
#[require(
    IsGrounded,
    IsMoving,
    RequireAction,
    Inventory,
    TransformInterpolation,
    CollisionEventsEnabled,
    CollisionLayers::new(GameLayer::Player, LayerMask::ALL,)
)]
#[reflect(Component, Default)]
pub struct CharacterController {
    /// Acceleration applied during moveme movement.
    pub acceleration: f32,
    /// Maximum velocity of walking.
    pub max_walk: f32,
    /// Maximum velocity of sprinting.
    pub max_sprint: f32,
    /// Damping value applied every frame (should be below 1.0).
    pub damping: f32,
    pub jump_impulse: f32,
    pub max_slope_angle: f32,
    pub gravity: Vec3,
}

impl Default for CharacterController {
    fn default() -> Self {
        Self {
            acceleration: 100.0,
            max_walk: 5.0,
            max_sprint: 10.0,
            damping: 0.8,
            jump_impulse: 4.0,
            max_slope_angle: 1.41,
            gravity: Vec3::new(0.0, -20.0, 0.0),
        }
    }
}
