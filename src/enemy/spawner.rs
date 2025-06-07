use bevy::prelude::*;

use crate::ui::Screen;

pub(super) struct EnemySpawnerPlugin;

impl Plugin for EnemySpawnerPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<EnemySpawner>();

        app.init_state::<SpawnWave>()
            .init_resource::<WaveCountdown>()
            .init_resource::<SpawnCount>()
            .init_resource::<SpawnTimer>()
            .add_systems(
                Update,
                (
                    (set_wave_countdown, set_spawn_timer)
                        .run_if(state_changed::<SpawnWave>),
                    ((wave_countdown, spawn_timer), spawn_enemy)
                        .chain(),
                )
                    .run_if(in_state(Screen::EnterLevel)),
            );
    }
}

fn spawn_enemy(
    countdown: Res<WaveCountdown>,
    timer: Res<SpawnTimer>,
) {
    if countdown.finished() == false {
        return;
    }

    if timer.just_finished() == false {
        return;
    }

    // TODO: Spawn enemy.
}

fn set_wave_countdown(
    current_wave: Res<State<SpawnWave>>,
    mut countdown: ResMut<WaveCountdown>,
    q_spawner: Query<&EnemySpawner>,
) {
    let Ok(spawner) = q_spawner.single() else {
        return;
    };

    let countdown_time = match current_wave.get() {
        SpawnWave::One => spawner.wave_1.countdown,
        SpawnWave::Two => spawner.wave_2.countdown,
        SpawnWave::Three => spawner.wave_3.countdown,
    };

    countdown.0 =
        Timer::from_seconds(countdown_time, TimerMode::Once);
}

fn set_spawn_timer(
    current_wave: Res<State<SpawnWave>>,
    mut timer: ResMut<SpawnTimer>,
    q_spawner: Query<&EnemySpawner>,
) {
    let Ok(spawner) = q_spawner.single() else {
        return;
    };

    let interval = match current_wave.get() {
        SpawnWave::One => spawner.wave_1.spawn_interval,
        SpawnWave::Two => spawner.wave_2.spawn_interval,
        SpawnWave::Three => spawner.wave_3.spawn_interval,
    };

    timer.0 = Timer::from_seconds(interval, TimerMode::Repeating);
}

/// Tick every frame.
fn wave_countdown(
    mut countdown: ResMut<WaveCountdown>,
    time: Res<Time>,
) {
    if countdown.finished() == false {
        countdown.tick(time.delta());
    }
}

fn spawn_timer(
    countdown: Res<WaveCountdown>,
    mut timer: ResMut<SpawnTimer>,
    time: Res<Time>,
) {
    // Only tick after countdown is reached.
    if countdown.finished() {
        timer.tick(time.delta());
    }
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct EnemySpawner {
    pub wave_1: WaveConfig,
    pub wave_2: WaveConfig,
    pub wave_3: WaveConfig,
}

#[derive(Reflect)]
pub struct WaveConfig {
    /// How long before the wave starts.
    pub countdown: f32,
    pub enemy_count: usize,
    pub spawn_interval: f32,
}

#[derive(
    States, Default, Debug, Hash, Clone, Copy, Eq, PartialEq,
)]
pub enum SpawnWave {
    #[default]
    One,
    Two,
    Three,
}

/// Countdown timer until enemies start to spawn.
#[derive(Resource, Deref, DerefMut, Default)]
pub struct WaveCountdown(Timer);

/// Number of enemies to spawn left.
#[derive(Resource, Deref, DerefMut, Default)]
pub struct SpawnCount(usize);

/// Time left before the next spawn.
#[derive(Resource, Deref, DerefMut, Default)]
pub struct SpawnTimer(Timer);
