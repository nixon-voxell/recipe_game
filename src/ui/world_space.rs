use bevy::prelude::*;
use bevy::ui::UiSystem;

pub(super) struct WorldSpaceUiPlugin;

impl Plugin for WorldSpaceUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            update_world_ui
                .after(UiSystem::Layout)
                .after(TransformSystem::TransformPropagate),
        )
        .add_observer(cleanup_world_ui);
    }
}

fn update_world_ui(
    q_camera_transform: Query<
        (&GlobalTransform, &Camera),
        With<WorldSpaceUiCamera>,
    >,
    q_global_transforms: Query<
        &GlobalTransform,
        Without<WorldSpaceUiCamera>,
    >,
    mut q_world_space_uis: Query<(
        &WorldUi,
        &mut Node,
        &ComputedNode,
    )>,
) {
    let Ok((camera_transform, camera)) = q_camera_transform.single()
    else {
        if q_camera_transform.iter().len() != 0 {
            warn!(
                "There is more than 1 camera with `WorldSpaceUiCamera` component attached to them!"
            );
        }

        // It's fine if there's no world space ui camera.
        return;
    };

    for (world_space_ui, mut node, computed_node) in
        q_world_space_uis.iter_mut()
    {
        let Ok(target_transform) =
            q_global_transforms.get(world_space_ui.target)
        else {
            warn!(
                "Unable to find WorldSpaceUi target: {}",
                world_space_ui.target
            );
            continue;
        };

        match camera.world_to_viewport(
            camera_transform,
            target_transform.translation()
                + world_space_ui.world_offset,
        ) {
            Ok(viewport) => {
                let viewport = viewport + world_space_ui.ui_offset;
                let half_size = computed_node.size * 0.5;

                node.left = Val::Px(viewport.x - half_size.x);
                node.top = Val::Px(viewport.y - half_size.y);
            }
            Err(err) => {
                warn!(
                    "Unable to get viewport location for target: {} ({err})",
                    world_space_ui.target
                );
            }
        }
    }
}

fn cleanup_world_ui(
    trigger: Trigger<OnRemove, RelatedWorldUis>,
    mut commands: Commands,
    q_related_uis: Query<&RelatedWorldUis>,
) -> Result {
    let entity = trigger.target();

    let related_uis = q_related_uis.get(entity)?;

    for ui_entity in related_uis.iter() {
        commands.entity(ui_entity).despawn();
    }

    Ok(())
}

/// Attached to the target entity of [`WorldUi`]s.
#[derive(Component, Deref, Default, Debug)]
#[relationship_target(relationship = WorldUi)]
pub struct RelatedWorldUis(Vec<Entity>);

/// Component for ui nodes to be transformed into world space
/// based on the target entity's [`GlobalTransform`].
#[derive(Component)]
#[relationship(relationship_target = RelatedWorldUis)]
pub struct WorldUi {
    #[relationship]
    pub target: Entity,
    pub ui_offset: Vec2,
    pub world_offset: Vec3,
}

impl WorldUi {
    pub fn new(target: Entity) -> Self {
        Self {
            target,
            ui_offset: Vec2::ZERO,
            world_offset: Vec3::ZERO,
        }
    }

    #[allow(dead_code)]
    pub fn with_world_offset(mut self, offset: Vec3) -> Self {
        self.world_offset = offset;
        self
    }

    #[allow(dead_code)]
    pub fn with_ui_offset(mut self, offset: Vec2) -> Self {
        self.ui_offset = offset;
        self
    }
}

/// A tag component for camera that will be used to render world space ui.
///
/// Should only be added to one camera!
#[derive(Component)]
pub struct WorldSpaceUiCamera;
