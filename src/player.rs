use bevy::color::palettes::tailwind::*;
use bevy::ecs::component::{ComponentHooks, Immutable, StorageType};
use bevy::ecs::query::{
    QueryData, QueryEntityError, QueryFilter, QuerySingleError,
    ROQueryItem,
};
use bevy::ecs::spawn::SpawnWith;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::action::{GamepadIndex, PlayerAction};
use crate::asset_pipeline::PrefabName;
use crate::camera_controller::split_screen::{
    CameraType, QueryCameras,
};
use crate::character_controller::CharacterController;
use crate::ui::world_space::WorldUi;
use crate::util::PropagateComponentAppExt;

pub mod player_attack;
pub mod player_mark;

pub(super) struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            player_attack::PlayerAttackPlugin,
            player_mark::PlayerMarkPlugin,
        ));

        app.init_state::<PlayerState>()
            .add_observer(setup_name_ui_for_player)
            .add_systems(
                OnEnter(PlayerState::Possessing),
                setup_possession_ui,
            )
            .add_systems(
                Update,
                (
                    process_posessing_inputs,
                    ready_inputs
                        .run_if(resource_exists::<PlayerPossessor>),
                )
                    .run_if(in_state(PlayerState::Possessing)),
            )
            .add_observer(handle_possession_triggers)
            .propagate_component::<PlayerType, Children>();

        app.register_type::<PlayerType>();
    }
}

fn ready_inputs(
    mut commands: Commands,
    player_possessor: Res<PlayerPossessor>,
    q_gamepads: Query<&Gamepad>,
    kbd_inputs: Res<ButtonInput<KeyCode>>,
    mut player_state: ResMut<NextState<PlayerState>>,
) {
    let Some((player_a, player_b)) =
        player_possessor.get_possessors()
    else {
        return;
    };

    let mut ready = kbd_inputs.just_pressed(KeyCode::Enter);
    for gamepad in q_gamepads.iter() {
        ready = ready || gamepad.just_pressed(GamepadButton::South);
    }

    if !ready {
        return;
    }

    match player_a {
        PossessorType::Keyboard => {
            commands.spawn(PlayerAction::new_kbm())
        }
        PossessorType::Gamepad(entity) => commands
            .spawn(PlayerAction::new_gamepad().with_gamepad(*entity)),
    }
    .insert(PlayerType::A);

    match player_b {
        PossessorType::Keyboard => {
            commands.spawn(PlayerAction::new_kbm())
        }
        PossessorType::Gamepad(entity) => commands
            .spawn(PlayerAction::new_gamepad().with_gamepad(*entity)),
    }
    .insert(PlayerType::B);

    player_state.set(PlayerState::Possessed);
}

fn process_posessing_inputs(
    mut commands: Commands,
    q_gamepads: Query<(&Gamepad, Entity)>,
    kbd_inputs: Res<ButtonInput<KeyCode>>,
) {
    if kbd_inputs.just_pressed(KeyCode::KeyA) {
        commands.trigger(Possession {
            player_type: Some(PlayerType::A),
            possessor: PossessorType::Keyboard,
        });
    }

    if kbd_inputs.just_pressed(KeyCode::KeyD) {
        commands.trigger(Possession {
            player_type: Some(PlayerType::B),
            possessor: PossessorType::Keyboard,
        });
    }

    // Handle cancelation.
    if kbd_inputs.just_pressed(KeyCode::Escape) {
        commands.trigger(Possession {
            player_type: None,
            possessor: PossessorType::Keyboard,
        });
    }

    for (gamepad, entity) in q_gamepads.iter() {
        if gamepad.just_pressed(GamepadButton::DPadLeft) {
            commands.trigger(Possession {
                player_type: Some(PlayerType::A),
                possessor: PossessorType::Gamepad(entity),
            });
        }

        if gamepad.just_pressed(GamepadButton::DPadRight) {
            commands.trigger(Possession {
                player_type: Some(PlayerType::B),
                possessor: PossessorType::Gamepad(entity),
            });
        }

        // Handle cancelation.
        if gamepad.just_pressed(GamepadButton::East) {
            commands.trigger(Possession {
                player_type: None,
                possessor: PossessorType::Gamepad(entity),
            });
        }
    }
}

fn handle_possession_triggers(
    trigger: Trigger<Possession>,
    mut commands: Commands,
    q_gamepad_indices: Query<&GamepadIndex>,
    mut player_possessor: ResMut<PlayerPossessor>,
) -> Result {
    let possession = trigger.event();

    if let Some(player_type) = possession.player_type {
        // Set color and possessors accordingly.
        match player_type {
            PlayerType::A => {
                player_possessor.player_a =
                    Some(possession.possessor);

                // Remove previous possession if any.
                if player_possessor.player_b
                    == Some(possession.possessor)
                {
                    player_possessor.player_b = None;
                }
            }
            PlayerType::B => {
                player_possessor.player_b =
                    Some(possession.possessor);

                // Remove previous possession if any.
                if player_possessor.player_a
                    == Some(possession.possessor)
                {
                    player_possessor.player_a = None;
                }
            }
        }
    } else {
        // Handle possession cancelation.
        if player_possessor.player_a == Some(possession.possessor) {
            player_possessor.player_a = None;
        }
        if player_possessor.player_b == Some(possession.possessor) {
            player_possessor.player_b = None;
        }
    }

    let get_text = |possessor: &PossessorType| {
        let text = match possessor {
            PossessorType::Keyboard => "Keyboard".to_string(),
            PossessorType::Gamepad(entity) => {
                let s = "Gamepad #".to_string();
                s + &format!(
                    "{}",
                    q_gamepad_indices.get(*entity)?.get()
                )
            }
        };

        Ok::<_, QueryEntityError>(centered_text(text))
    };

    if let Some(possessor) = player_possessor.player_a {
        commands
            .entity(player_possessor.ui_slot_a)
            .insert(BackgroundColor(EMERALD_600.into()))
            .despawn_related::<Children>()
            .with_child(get_text(&possessor)?);
    } else {
        commands
            .entity(player_possessor.ui_slot_a)
            .insert(BackgroundColor(RED_900.into()))
            .despawn_related::<Children>()
            .with_child(centered_text("N/A"));
    }

    if let Some(possessor) = player_possessor.player_b {
        commands
            .entity(player_possessor.ui_slot_b)
            .insert(BackgroundColor(EMERALD_600.into()))
            .despawn_related::<Children>()
            .with_child(get_text(&possessor)?);
    } else {
        commands
            .entity(player_possessor.ui_slot_b)
            .insert(BackgroundColor(RED_900.into()))
            .despawn_related::<Children>()
            .with_child(centered_text("N/A"));
    }

    if player_possessor.is_ready() {
        // Allow pressing A / Enter to ready the players!
        commands
            .entity(player_possessor.ui_ready)
            .insert(Visibility::Inherited);
    } else {
        commands
            .entity(player_possessor.ui_ready)
            .insert(Visibility::Hidden);
    }

    Ok(())
}

fn setup_possession_ui(mut commands: Commands) {
    const INSTRUCTION_CANCEL: &str =
        "Press Esc (keyboard) | B (controller) to cancel.";
    const INSTRUCTION_A: &str = "Press:\n\
    A (keyboard) / DPadLeft (controller)";
    const INSTRUCTION_B: &str = "Press:\n\
    D (keyboard) / DPadRight (controller)";
    const INSTRUCTION_READY: &str =
        "Press Enter (keyboard) / A (controller) to confirm!";

    let instruction_ui_node = Node {
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        padding: UiRect::all(Val::VMin(6.0)),
        flex_grow: 1.0,
        flex_direction: FlexDirection::Column,
        ..default()
    };

    // The rectangle ui slot for possession indication.
    let possession_slot = (
        Node {
            width: Val::VMin(20.0),
            height: Val::VMin(10.0),
            margin: UiRect::all(Val::VMin(2.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(RED_900.with_alpha(0.8).into()),
        BorderRadius::all(Val::VMin(2.0)),
    );

    let ui_slot_a = commands
        .spawn(possession_slot.clone())
        .with_child(centered_text("N/A"))
        .id();
    let ui_slot_b = commands
        .spawn(possession_slot)
        .with_child(centered_text("N/A"))
        .id();

    let ui_ready = commands
        .spawn((
            Text::new(INSTRUCTION_READY),
            TextLayout::new_with_justify(JustifyText::Center),
            Visibility::Hidden,
        ))
        .id();

    commands.insert_resource(PlayerPossessor {
        player_a: None,
        player_b: None,
        ui_slot_a,
        ui_slot_b,
        ui_ready,
    });

    let instruction_content_ui = Children::spawn((
        SpawnWith({
            let instruction_ui_node = instruction_ui_node.clone();
            move |parent: &mut ChildSpawner| {
                parent
                    .spawn(instruction_ui_node)
                    .with_child((
                        Text::new("Player A"),
                        Node {
                            margin: UiRect::all(Val::VMin(3.0)),
                            ..default()
                        },
                    ))
                    .with_child(Text::new(INSTRUCTION_A))
                    .add_child(ui_slot_a);
            }
        }),
        // Separation line.
        Spawn((
            Node {
                width: Val::Px(10.0),
                height: Val::Percent(80.0),
                ..default()
            },
            BackgroundColor(GRAY_200.into()),
        )),
        SpawnWith(move |parent: &mut ChildSpawner| {
            parent
                .spawn(instruction_ui_node)
                .with_child((
                    Text::new("Player B"),
                    Node {
                        margin: UiRect::all(Val::VMin(3.0)),
                        ..default()
                    },
                ))
                .with_child(Text::new(INSTRUCTION_B))
                .add_child(ui_slot_b);
        }),
    ));

    let instruction_ui = [
        commands
            .spawn((
                Text::new(INSTRUCTION_CANCEL),
                TextLayout::new_with_justify(JustifyText::Center),
            ))
            .id(),
        commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    flex_grow: 1.0,
                    ..default()
                },
                instruction_content_ui,
            ))
            .id(),
        ui_ready,
    ];

    commands.spawn((
        StateScoped(PlayerState::Possessing),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            padding: UiRect::all(Val::VMin(10.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        // Should be on top of all other uis.
        GlobalZIndex(10),
        Children::spawn(SpawnWith(
            move |parent: &mut ChildSpawner| {
                parent
                    .spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::VMin(4.0)),
                            flex_grow: 1.0,
                            ..default()
                        },
                        BackgroundColor(
                            ZINC_950.with_alpha(0.8).into(),
                        ),
                        BorderRadius::all(Val::VMin(4.0)),
                        BoxShadow::new(
                            ZINC_950.with_alpha(0.9).into(),
                            Val::Auto,
                            Val::Auto,
                            Val::Px(20.0),
                            Val::Px(16.0),
                        ),
                    ))
                    .add_children(&instruction_ui);
            },
        )),
    ));
}

fn centered_text(text: impl Into<String>) -> impl Bundle {
    (
        Text::new(text),
        TextLayout::new_with_justify(JustifyText::Center),
    )
}

/// Setup world space name ui for players.
fn setup_name_ui_for_player(
    trigger: Trigger<OnAdd, PlayerType>,
    mut commands: Commands,
    q_players: Query<&PlayerType, With<CharacterController>>,
    q_cameras: QueryCameras<Entity>,
) -> Result {
    let entity = trigger.target();

    let Ok(player_type) = q_players.get(entity) else {
        // Spawned entity might not be a character.
        return Ok(());
    };

    let ui_bundle = move |name: &str, height: f32| {
        (
            WorldUi::new(entity).with_world_offset(Vec3::Y * height),
            Node {
                padding: UiRect::all(Val::Px(8.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                position_type: PositionType::Absolute,
                ..default()
            },
            BorderRadius::all(Val::Px(8.0)),
            BackgroundColor(ZINC_900.with_alpha(0.5).into()),
            BoxShadow::new(
                ZINC_900.into(),
                Val::Px(4.0),
                Val::Px(4.0),
                Val::Px(14.0),
                Val::Px(12.0),
            ),
            Children::spawn(Spawn((
                Text::new(name),
                TextLayout::new_with_justify(JustifyText::Center),
            ))),
        )
    };

    // Spawn ui only for the other player to view their floating tag.
    match player_type {
        PlayerType::A => {
            commands.spawn((
                ui_bundle("Polo Bun", 1.0),
                UiTargetCamera(q_cameras.get(CameraType::B)?),
            ));
        }
        PlayerType::B => {
            commands.spawn((
                ui_bundle("Baguette", 1.5),
                UiTargetCamera(q_cameras.get(CameraType::A)?),
            ));
        }
    }

    Ok(())
}

#[derive(Reflect, Debug, Clone, Copy, PartialEq, Eq)]
#[reflect(Component)]
pub enum PlayerType {
    /// Polo Bun.
    A,
    /// Baguette.
    B,
}

impl PlayerType {
    pub fn prefab_name(&self) -> PrefabName {
        match self {
            PlayerType::A => PrefabName::FileName("polo_bun"),
            PlayerType::B => PrefabName::FileName("baguette"),
        }
    }
}

impl Component for PlayerType {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    type Mutability = Immutable;

    /// Setup player tag: [`PlayerA`] or [`PlayerB`]
    /// based on [`PlayerType`].
    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_add(|mut world, hook| {
            let entity = hook.entity;
            let player_type = world.get::<Self>(hook.entity).unwrap();

            match player_type {
                PlayerType::A => {
                    world.commands().entity(entity).insert(PlayerA);
                }
                PlayerType::B => {
                    world.commands().entity(entity).insert(PlayerB);
                }
            }
        });
    }
}

/// A shorthand [`SystemParam`] for getting all types of players
/// using exclusive queries.
#[derive(SystemParam)]
pub struct QueryPlayers<'w, 's, D, F = ()>
where
    D: QueryData + 'static,
    F: QueryFilter + 'static,
{
    pub q_camera_a: QueryPlayerA<'w, 's, D, F>,
    pub q_camera_b: QueryPlayerB<'w, 's, D, F>,
}

impl<D, F> QueryPlayers<'_, '_, D, F>
where
    D: QueryData + 'static,
    F: QueryFilter + 'static,
{
    #[allow(dead_code)]
    pub fn get(
        &self,
        player_type: PlayerType,
    ) -> Result<ROQueryItem<'_, D>, QuerySingleError> {
        match player_type {
            PlayerType::A => self.q_camera_a.single(),
            PlayerType::B => self.q_camera_b.single(),
        }
    }

    #[allow(dead_code)]
    pub fn get_mut(
        &mut self,
        player_type: PlayerType,
    ) -> Result<D::Item<'_>, QuerySingleError> {
        match player_type {
            PlayerType::A => self.q_camera_a.single_mut(),
            PlayerType::B => self.q_camera_b.single_mut(),
        }
    }
}

/// A unique query to the [`PlayerA`] entity.
pub type QueryPlayerA<'w, 's, D, F = ()> =
    Query<'w, 's, D, (F, With<PlayerA>, Without<PlayerB>)>;

/// A unique query to the [`PlayerB`] entity.
pub type QueryPlayerB<'w, 's, D, F = ()> =
    Query<'w, 's, D, (F, With<PlayerB>, Without<PlayerA>)>;

/// A unique component tag for player A.
#[derive(Component, Debug)]
pub struct PlayerA;

/// A unique component tag for player B.
#[derive(Component, Debug)]
pub struct PlayerB;

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
#[states(scoped_entities)]
pub enum PlayerState {
    #[default]
    Possessing,
    Possessed,
}

/// The currently possession state of the players.
#[derive(Resource, Debug)]
pub struct PlayerPossessor {
    pub player_a: Option<PossessorType>,
    pub player_b: Option<PossessorType>,
    pub ui_slot_a: Entity,
    pub ui_slot_b: Entity,
    pub ui_ready: Entity,
}

impl PlayerPossessor {
    pub fn is_ready(&self) -> bool {
        self.player_a.is_some() && self.player_b.is_some()
    }

    pub fn get_possessors(
        &self,
    ) -> Option<(&PossessorType, &PossessorType)> {
        Some((self.player_a.as_ref()?, self.player_b.as_ref()?))
    }
}

/// Possesion type, can be keyboard or a specific gamepad.
#[derive(Component, Debug, PartialEq, Eq, Clone, Copy)]
pub enum PossessorType {
    Keyboard,
    Gamepad(Entity),
}

#[derive(Event, Debug, Clone, Copy)]
pub struct Possession {
    /// [Some] for a positive possession, [None] for cancelation.
    pub player_type: Option<PlayerType>,
    pub possessor: PossessorType,
}
