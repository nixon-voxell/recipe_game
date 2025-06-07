use avian3d::prelude::*;
use bevy::prelude::*;

use crate::util::PropagateComponentAppExt;

pub(super) struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            PhysicsPlugins::default(),
            // PhysicsPickingPlugin,
            PhysicsDebugPlugin::default(),
        ));

        app.add_observer(setup_collision_layer)
            .propagate_component::<CollisionLayers, RigidBodyColliders>();

        app.register_type::<CollisionLayerConstructor>()
            .register_type::<GameLayer>();
    }
}

fn setup_collision_layer(
    trigger: Trigger<OnAdd, CollisionLayerConstructor>,
    mut commands: Commands,
    q_constructors: Query<&CollisionLayerConstructor>,
) -> Result {
    let entity = trigger.target();

    let constructor = q_constructors.get(entity)?;
    let mut memberships = LayerMask::NONE;
    let mut filters = LayerMask::NONE;

    for &membership in constructor.memberships.iter() {
        memberships.add(membership);
    }

    for &filter in constructor.filters.iter() {
        filters.add(filter);
    }

    commands
        .entity(trigger.target())
        .insert(CollisionLayers::new(memberships, filters))
        .remove::<CollisionLayerConstructor>();

    Ok(())
}

/// This component serves only the purpose of creating [`CollisionLayers`].
#[derive(Component, Reflect, Default, Debug)]
#[reflect(Component, Default)]
pub struct CollisionLayerConstructor {
    pub memberships: Vec<GameLayer>,
    pub filters: Vec<GameLayer>,
}

#[derive(
    PhysicsLayer, Component, Reflect, Default, Debug, Clone, Copy,
)]
#[reflect(Component, Default)]
pub enum GameLayer {
    #[default]
    Default,
    Player,
    Enemy,
    Interactable,
    InventoryItem,
    Projectile,
    Tower,
}
