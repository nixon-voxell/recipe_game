use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy_asset_loader::prelude::*;

pub mod animation_pipeline;

pub(super) struct AssetPipelinePlugin;

impl Plugin for AssetPipelinePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(animation_pipeline::AnimationPipelinePlugin);

        app.init_state::<PrefabState>()
            .init_state::<SceneState>()
            .add_loading_state(
                LoadingState::new(PrefabState::LoadingGltf)
                    .continue_to_state(PrefabState::LoadingAnimation)
                    .with_dynamic_assets_file::<StandardDynamicAssetCollection>(
                        "dynamic_asset.assets.ron",
                    )
                    .load_collection::<PrefabAssets>(),
            )
            .add_loading_state(
                LoadingState::new(SceneState::LoadingGltf)
                    .continue_to_state(SceneState::Loaded)
                    .with_dynamic_assets_file::<StandardDynamicAssetCollection>(
                        "dynamic_asset.assets.ron",
                    )
                    .load_collection::<SceneAssets>(),
            )
            .add_systems(
                OnEnter(SceneState::Loaded),
                load_default_scene,
            );

        #[cfg(feature = "dev")]
        app.register_type::<PrefabAssets>();
    }
}

fn load_default_scene(
    mut commands: Commands,
    scenes: Res<SceneAssets>,
    gltfs: Res<Assets<Gltf>>,
) -> Result {
    let gltf = gltfs
        .get(&scenes.default_scene)
        .ok_or("Scene should have been loaded")?;

    commands.spawn(SceneRoot(
        gltf.default_scene
            .clone()
            .expect("Should have a default scene."),
    ));

    Ok(())
}

#[derive(AssetCollection, Resource, Debug)]
#[cfg_attr(feature = "dev", derive(Reflect))]
#[cfg_attr(feature = "dev", reflect(Resource))]
pub struct SceneAssets {
    #[asset(key = "scenes.default")]
    pub default_scene: Handle<Gltf>,
}

#[derive(AssetCollection, Resource, Debug)]
#[cfg_attr(feature = "dev", derive(Reflect))]
#[cfg_attr(feature = "dev", reflect(Resource))]
pub struct PrefabAssets {
    #[asset(key = "prefabs", collection(typed, mapped))]
    pub named_prefabs: HashMap<String, Handle<Gltf>>,
    pub named_graphs: HashMap<String, Handle<AnimationGraph>>,
}

impl PrefabAssets {
    pub fn get_gltf<'a>(
        &self,
        name: PrefabName,
        gltfs: &'a Assets<Gltf>,
    ) -> Option<&'a Gltf> {
        self.named_prefabs
            .get(&name.cast())
            .and_then(|handle| gltfs.get(handle))
    }
}

#[derive(Debug)]
pub enum PrefabName<'a> {
    Absolute(&'a str),
    _FileName(&'a str),
}

impl PrefabName<'_> {
    pub fn cast(self) -> String {
        match self {
            PrefabName::Absolute(name) => name.to_string(),
            PrefabName::_FileName(filename) => {
                let prefix = "prefabs/".to_string();
                prefix + filename + ".glb"
            }
        }
    }
}

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
pub enum PrefabState {
    #[default]
    LoadingGltf,
    LoadingAnimation,
    Loaded,
}

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
pub enum SceneState {
    #[default]
    LoadingGltf,
    Loaded,
}
