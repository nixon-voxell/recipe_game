use avian3d::{math::*, parry::na::RealField, prelude::*};
use bevy::prelude::*;

/// Plugin that sets up kinematic character movement
pub(super) struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app
            // .add_systems(Startup, spawn_test_scene)
            .add_event::<MovementAction>()
            .add_systems(
                Update,
                (
                    keyboard_input,
                    apply_gravity,
                    update_grounded,
                    movement,
                    apply_movement_damping,
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

/// Reads keyboard input and emits movement events
fn keyboard_input(
    mut writer: EventWriter<MovementAction>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let up = keys.pressed(KeyCode::KeyW);
    let down = keys.pressed(KeyCode::KeyS);
    let left = keys.pressed(KeyCode::KeyA);
    let right = keys.pressed(KeyCode::KeyD);

    let dir = Vector2::new(
        (right as i32 - left as i32) as Scalar,
        (up as i32 - down as i32) as Scalar,
    )
    .clamp_length_max(1.0);

    let sprint = keys.pressed(KeyCode::ShiftLeft)
        || keys.pressed(KeyCode::ShiftRight);

    if dir != Vector2::ZERO {
        writer.write(MovementAction::Move { dir, sprint });
    }
    if keys.just_pressed(KeyCode::Space) {
        writer.write(MovementAction::Jump);
    }
}

/// Updates grounded state by raycasting downwards
fn update_grounded(
    mut spatial_query: SpatialQuery,
    mut query: Query<(
        Entity,
        &GlobalTransform,
        &mut CharacterController,
    )>,
) {
    spatial_query.update_pipeline();

    for (entity, tf, mut ctl) in query.iter_mut() {
        let char_pos = tf.translation();

        let ray_origin = char_pos;
        let ray_direction = Dir3::NEG_Y;
        let max_distance = 1.5;

        // Exclude the character's own entity from the raycast
        let filter = SpatialQueryFilter::default()
            .with_excluded_entities([entity]);

        if let Some(hit) = spatial_query.cast_ray(
            ray_origin,
            ray_direction,
            max_distance,
            true,
            &filter,
        ) {
            let slope_angle = hit.normal.angle_between(Vec3::Y);

            // Check if the normal is valid and surface is walkable
            if slope_angle.is_finite() {
                ctl.grounded = slope_angle <= ctl.max_slope_angle;
            } else {
                ctl.grounded = false;
            }
        } else {
            ctl.grounded = false;
        }
    }
}

/// Applies gravity to vertical velocity
fn apply_gravity(
    time: Res<Time>,
    mut query: Query<(&mut LinearVelocity, &CharacterController)>,
) {
    let dt = time.delta_secs_f64().adjust_precision();
    for (mut linvel, ctl) in query.iter_mut() {
        if !ctl.grounded {
            linvel.0 += ctl.gravity * dt;
        }
    }
}

/// Handles movement and jumping
fn movement(
    time: Res<Time>,
    mut reader: EventReader<MovementAction>,
    cam_tf_q: Query<&GlobalTransform, With<Camera3d>>,
    mut query: Query<(
        &mut CharacterController,
        &mut Transform,
        &mut LinearVelocity,
    )>,
) {
    let dt = time.delta_secs_f64() as f32;

    // Speed caps
    let max_walk = 5.0;
    let max_sprint = 10.0;

    // Get camera transform
    let cam_tf = match cam_tf_q.single() {
        Ok(tf) => tf,
        Err(_) => return,
    };
    let cam_forward = cam_tf.forward();
    let cam_forward = Vec3::new(cam_forward.x, 0.0, cam_forward.z)
        .normalize_or_zero();
    let cam_right = cam_forward.cross(Vec3::Y).normalize_or_zero();

    for event in reader.read() {
        match event {
            MovementAction::Move { dir, sprint }
                if *dir != Vector2::ZERO =>
            {
                // Compute yaw directly from that vector: atan2(x, z)
                let world_move =
                    (cam_forward * dir.y) + (cam_right * dir.x);
                let world_move = world_move.normalize_or_zero();

                // Compute yaw and apply offset based on model orientation
                let yaw = f32::atan2(-world_move.x, -world_move.z);

                for (mut ctl, mut tx, mut linvel) in query.iter_mut()
                {
                    // Rotate to face movement direction
                    tx.rotation = Quat::from_rotation_y(yaw);

                    // Only allow sprinting if grounded
                    let can_sprint = *sprint && ctl.grounded;

                    // Apply acceleration * sprint factor
                    let factor = if can_sprint { 2.0 } else { 1.0 };
                    let acceleration = ctl.acceleration;
                    linvel.0 +=
                        world_move * (acceleration * dt * factor);

                    // Clamp horizontal speed (only sprint speed if grounded)
                    let max_speed = if can_sprint {
                        max_sprint
                    } else {
                        max_walk
                    };
                    let horiz = Vec2::new(linvel.0.x, linvel.0.z);
                    if horiz.length() > max_speed {
                        let clamped = horiz.normalize() * max_speed;
                        linvel.0.x = clamped.x;
                        linvel.0.z = clamped.y;
                    }

                    // Synchronize controller velocity
                    ctl.velocity = linvel.0;
                }
            }
            MovementAction::Jump => {
                for (mut ctl, _, mut linvel) in query.iter_mut() {
                    if ctl.grounded {
                        linvel.0.y = ctl.jump_impulse;
                        ctl.grounded = false;
                    }
                }
            }
            _ => {}
        }
    }

    // Clamp horizontal speed for airborne characters every frame
    for (ctl, _, mut linvel) in query.iter_mut() {
        if !ctl.grounded {
            let horiz = Vec2::new(linvel.0.x, linvel.0.z);
            if horiz.length() > max_walk {
                let clamped = horiz.normalize() * max_walk;
                linvel.0.x = clamped.x;
                linvel.0.z = clamped.y;
            }
        }
    }
}

/// Applies damping to horizontal movement
fn apply_movement_damping(
    mut query: Query<(&mut LinearVelocity, &CharacterController)>,
) {
    for (mut linvel, ctl) in query.iter_mut() {
        // Apply damping directly to physics velocity
        linvel.x *= ctl.damping;
        linvel.z *= ctl.damping;
    }
}

/// Handles collisions for kinematic character controllers
fn kinematic_controller_collisions(
    collisions: Collisions,
    bodies: Query<&RigidBody>,
    collider_rbs: Query<&ColliderOf, Without<Sensor>>,
    mut controllers: Query<
        (
            &mut Position,
            &mut LinearVelocity,
            &mut CharacterController,
        ),
        (With<RigidBody>, With<CharacterController>),
    >,
    time: Res<Time>,
) {
    let dt = time.delta_secs_f64().adjust_precision();

    for contacts in collisions.iter() {
        // Pull out the two bodies
        let Ok([&ColliderOf { body: a }, &ColliderOf { body: b }]) =
            collider_rbs
                .get_many([contacts.collider1, contacts.collider2])
        else {
            continue;
        };

        // Figure out which one is me
        let (entity, is_first, other) = if controllers.get(a).is_ok()
        {
            (a, true, b)
        } else if controllers.get(b).is_ok() {
            (b, false, a)
        } else {
            continue;
        };

        // Only do kinematic
        if !bodies.get(entity).unwrap().is_kinematic() {
            continue;
        }

        let (mut pos, mut linvel, mut ctl) =
            controllers.get_mut(entity).unwrap();

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
                    let is_jumping = linvel.y > 0.0;

                    // Apply penetration correction unless jumping into ceiling
                    if !(is_ground && is_jumping) {
                        pos.0 += normal * pt.penetration;
                    }

                    // Cancel all vertical velocity when grounded
                    if is_ground {
                        linvel.y = 0.0;
                        ctl.grounded = true;
                    }
                }
                deepest = deepest.max(pt.penetration);
            }

            // Skip dynamic collisions
            if other_dynamic {
                continue;
            }

            let slope_angle = normal.angle_between(Vector::Y).abs();
            let can_climb = slope_angle <= ctl.max_slope_angle;

            if deepest > 0.0 {
                if can_climb {
                    // slope-snap logic
                    let dir_xz = normal
                        .reject_from_normalized(Vector::Y)
                        .normalize_or_zero();
                    let vel_xz = linvel.dot(dir_xz);
                    let max_y = -vel_xz * slope_angle.tan();
                    linvel.y = linvel.y.max(max_y);
                } else {
                    // Wall-slide: zero out velocity into the wall
                    let into = linvel.dot(normal);
                    if into < 0.0 {
                        linvel.0 -= normal * into;
                    }
                }
            } else {
                // Speculative contact
                let n_speed = linvel.dot(normal);
                if n_speed < 0.0 {
                    let impulse = (n_speed - (deepest / dt)) * normal;
                    if can_climb {
                        linvel.y -= impulse.y.min(0.0);
                    } else {
                        let mut i = impulse;
                        i.y = i.y.max(0.0);
                        linvel.0 -= i;
                    }
                }
            }
        }
    }
}

/// Movement actions triggered by input
#[derive(Event)]
pub enum MovementAction {
    Move { dir: Vector2, sprint: bool },
    Jump,
}

/// Marker for kinematic character bodies
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct CharacterController {
    pub acceleration: Scalar,
    pub damping: Scalar,
    pub jump_impulse: Scalar,
    pub max_slope_angle: Scalar,
    // Gravity
    pub gravity: Vector,
    // State - Compute only
    pub grounded: bool,
    pub velocity: Vector,
}
