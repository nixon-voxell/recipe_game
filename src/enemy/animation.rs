use core::time::Duration;

use bevy::animation::AnimationTarget;
use bevy::prelude::*;

use crate::asset_pipeline::animation_pipeline::{
    AnimationGraphMap, NodeMap,
};
use crate::asset_pipeline::{AssetState, PrefabAssets, PrefabName};

use super::{Enemy, TargetReached};

pub(super) struct EnemyAnimationPlugin;

impl Plugin for EnemyAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (setup_animation_graph, movement_animation)
                .run_if(in_state(AssetState::Loaded)),
        );
    }
}

fn movement_animation(
    q_enemies: Query<
        (&NodeMap, &AnimationTarget, Has<TargetReached>),
        With<Enemy>,
    >,
    mut q_animation_players: Query<(
        &mut AnimationPlayer,
        &mut AnimationTransitions,
    )>,
) -> Result {
    for (node_map, animation_target, reached_target) in
        q_enemies.iter()
    {
        let (mut anim_player, mut anim_transitions) =
            q_animation_players.get_mut(animation_target.player)?;

        if reached_target {
            info!("Eating...");
            let eat_node = *node_map
                .get("Eat")
                .ok_or("No idle animation found for enemy!")?;

            if anim_player.is_playing_animation(eat_node) == false {
                anim_transitions
                    .play(
                        &mut anim_player,
                        eat_node,
                        Duration::from_millis(200),
                    )
                    .repeat();
            }
        } else {
            info!("Walking...");
            let walk_node = *node_map
                .get("Walk")
                .ok_or("No walking animation found for enemy!")?;

            if anim_player.is_playing_animation(walk_node) == false {
                anim_transitions
                    .play(
                        &mut anim_player,
                        walk_node,
                        Duration::from_millis(200),
                    )
                    .set_speed(1.5)
                    .repeat();
            }
        }
    }

    Ok(())
}

fn setup_animation_graph(
    mut commands: Commands,
    q_enemies: Query<
        (&AnimationTarget, Entity),
        (With<Enemy>, Without<NodeMap>),
    >,
    prefabs: Res<PrefabAssets>,
) -> Result {
    for (animation_target, entity) in q_enemies.iter() {
        let AnimationGraphMap { graph, node_map } = prefabs
            .get_animation(PrefabName::FileName("mouse_a"))
            .ok_or("Unable to get animation for enemy!")?;

        commands.entity(entity).insert(node_map.clone());
        commands.entity(animation_target.player).insert((
            AnimationGraphHandle(graph.clone()),
            AnimationTransitions::new(),
        ));

        info!("Setup animation graph for enemy.");
    }

    Ok(())
}
