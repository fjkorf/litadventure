use std::collections::HashMap;

use bevy::prelude::*;
use bevy_common_assets::ron::RonAssetPlugin;
use serde::Deserialize;

// -- Objective data --

#[derive(Deserialize, Debug, Clone)]
pub struct ObjectiveDef {
    pub id: String,
    pub text: String,
    pub visible: bool,
    #[serde(default)]
    pub conflicts_with: Vec<String>,
    #[serde(default)]
    pub unlocks: Vec<String>,
    pub requires_item: Option<String>,
    pub requires_navigation: Option<String>,
}

#[derive(Deserialize, Asset, TypePath, Debug)]
pub struct ObjectiveSet {
    pub objectives: Vec<ObjectiveDef>,
}

// -- Hint data --

#[derive(Deserialize, Asset, TypePath, Debug)]
pub struct HintSet {
    pub hints: HashMap<String, [String; 3]>,
    #[serde(default = "default_tier_thresholds")]
    pub tier_thresholds: [f32; 3],
}

fn default_tier_thresholds() -> [f32; 3] {
    [30.0, 60.0, 90.0]
}

// -- Recipe data --

#[derive(Deserialize, Debug, Clone)]
pub struct RecipeDef {
    pub item_a: String,
    pub item_b: String,
    pub result_id: String,
    pub result_name: String,
    pub result_description: String,
}

#[derive(Deserialize, Asset, TypePath, Debug)]
pub struct RecipeBook {
    pub recipes: Vec<RecipeDef>,
}

// -- Room data --

#[derive(Deserialize, Debug, Clone)]
pub struct RoomDef {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub starting_spot: Option<String>,
}

#[derive(Deserialize, Asset, TypePath, Debug)]
pub struct RoomSet {
    pub rooms: Vec<RoomDef>,
    pub starting_room: String,
}

// -- Level manifest --

#[derive(Deserialize, Debug, Clone)]
pub struct RoomSceneRef {
    pub scene: String,
}

#[derive(Deserialize, Asset, TypePath, Debug)]
pub struct LevelManifest {
    pub name: String,
    pub starting_room: String,
    pub rooms: HashMap<String, RoomSceneRef>,
}

// -- Asset handles stored as a resource --

#[derive(Resource, Default)]
pub struct GameDataHandles {
    pub objectives: Option<Handle<ObjectiveSet>>,
    pub hints: Option<Handle<HintSet>>,
    pub recipes: Option<Handle<RecipeBook>>,
    pub rooms: Option<Handle<RoomSet>>,
    pub level: Option<Handle<LevelManifest>>,
}

/// Whether all game data assets have finished loading.
#[derive(Resource, Default)]
pub struct GameDataReady(pub bool);

/// System that kicks off loading all game data assets.
fn load_game_data(mut handles: ResMut<GameDataHandles>, asset_server: Res<AssetServer>) {
    handles.objectives = Some(asset_server.load("data/objectives.ron"));
    handles.hints = Some(asset_server.load("data/hints.ron"));
    handles.recipes = Some(asset_server.load("data/recipes.ron"));
    handles.rooms = Some(asset_server.load("data/rooms.ron"));
    handles.level = Some(asset_server.load("data/demo.level.ron"));
}

/// System that checks if all assets are loaded and sets the ready flag.
fn check_game_data_ready(
    handles: Res<GameDataHandles>,
    objectives: Res<Assets<ObjectiveSet>>,
    hints: Res<Assets<HintSet>>,
    recipes: Res<Assets<RecipeBook>>,
    rooms: Res<Assets<RoomSet>>,
    levels: Res<Assets<LevelManifest>>,
    mut ready: ResMut<GameDataReady>,
) {
    if ready.0 {
        return;
    }

    let all_loaded = handles
        .objectives
        .as_ref()
        .is_some_and(|h| objectives.get(h).is_some())
        && handles
            .hints
            .as_ref()
            .is_some_and(|h| hints.get(h).is_some())
        && handles
            .recipes
            .as_ref()
            .is_some_and(|h| recipes.get(h).is_some())
        && handles
            .rooms
            .as_ref()
            .is_some_and(|h| rooms.get(h).is_some())
        && handles
            .level
            .as_ref()
            .is_some_and(|h| levels.get(h).is_some());

    if all_loaded {
        ready.0 = true;
    }
}

pub struct GameDataPlugin;

impl Plugin for GameDataPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RonAssetPlugin::<ObjectiveSet>::new(&["objectives.ron"]))
            .add_plugins(RonAssetPlugin::<HintSet>::new(&["hints.ron"]))
            .add_plugins(RonAssetPlugin::<RecipeBook>::new(&["recipes.ron"]))
            .add_plugins(RonAssetPlugin::<RoomSet>::new(&["rooms.ron"]))
            .add_plugins(RonAssetPlugin::<LevelManifest>::new(&["level.ron"]))
            .init_resource::<GameDataHandles>()
            .init_resource::<GameDataReady>()
            .add_systems(Startup, load_game_data)
            .add_systems(Update, check_game_data_ready);
    }
}
