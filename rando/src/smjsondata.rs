use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub rooms: Vec<Room>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum Note {
    Line(String),
    Lines(Vec<String>),
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Room {
    pub id: i64,
    pub name: String,
    pub area: String,
    pub subarea: String,
    pub room_address: String,
    pub nodes: Vec<Node>,
    pub enemies: Vec<Enemy>,
    pub links: Vec<Link>,
    #[serde(default)]
    pub obstacles: Vec<Obstacle3>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub id: i64,
    pub name: String,
    pub node_type: String,
    pub node_sub_type: String,
    pub node_address: Option<String>,
    #[serde(default)]
    pub runways: Vec<Runway>,
    #[serde(default)]
    pub can_leave_charged: Vec<CanLeaveCharged>,
    #[serde(default)]
    pub locks: Vec<Lock>,
    pub note: Option<Note>,
    #[serde(default)]
    pub utility: Vec<String>,
    #[serde(default)]
    pub yields: Vec<String>,
    pub node_item: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Runway {
    pub length: i64,
    pub strats: Vec<Strat>,
    pub open_end: i64,
    pub usable_coming_in: Option<bool>,
    pub steep_up_tiles: Option<i64>,
    pub steep_down_tiles: Option<i64>,
    pub ending_up_tiles: Option<i64>,
    pub note: Option<Note>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Strat {
    pub name: String,
    pub notable: bool,
    #[serde(default)]
    pub requires: Vec<::serde_json::Value>,
    #[serde(default)]
    pub obstacles: Vec<Obstacle>,
    pub note: Option<Note>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Obstacle {
    pub id: String,
    pub name: Option<String>,
    #[serde(default)]
    pub requires: Vec<::serde_json::Value>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanLeaveCharged {
    pub used_tiles: i64,
    pub frames_remaining: i64,
    pub open_end: i64,
    pub strats: Vec<Strat2>,
    pub steep_up_tiles: Option<i64>,
    pub steep_down_tiles: Option<i64>,
    pub shinespark_frames: Option<i64>,
    pub initiate_at: Option<i64>,
    pub note: Option<Note>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Strat2 {
    pub name: String,
    pub notable: bool,
    #[serde(default)]
    pub requires: Vec<::serde_json::Value>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lock {
    pub lock_type: String,
    pub unlock_strats: Vec<UnlockStrat>,
    #[serde(default)]
    pub lock: Vec<String>,
    pub note: Option<Note>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnlockStrat {
    pub name: String,
    pub notable: bool,
    #[serde(default)]
    pub requires: Vec<::serde_json::Value>,
    pub note: Option<Note>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Enemy {
    pub name: String,
    pub quantity: i64,
    #[serde(default)]
    pub home_nodes: Vec<i64>,
    #[serde(default)]
    pub spawn: Vec<String>,
    #[serde(default)]
    pub stop_spawn: Vec<::serde_json::Value>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    pub from: i64,
    pub to: Vec<To>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct To {
    pub id: i64,
    pub strats: Vec<Strat3>,
    pub note: Option<Note>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Strat3 {
    pub name: String,
    pub notable: bool,
    #[serde(default)]
    pub requires: Vec<::serde_json::Value>,
    pub note: Option<Note>,
    #[serde(default)]
    pub obstacles: Vec<Obstacle2>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Obstacle2 {
    pub id: String,
    pub requires: Vec<::serde_json::Value>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Obstacle3 {
    pub id: String,
    pub name: String,
    pub obstacle_type: String,
    #[serde(default)]
    pub requires: Vec<serde_json::Value>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Require {
    pub or: (String, String, String, Or, Or2, String),
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Or {
    pub can_come_in_charged: CanComeInCharged,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanComeInCharged {
    pub from_node: i64,
    pub frames_remaining: i64,
    pub shinespark_frames: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Or2 {
    pub can_come_in_charged: CanComeInCharged2,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanComeInCharged2 {
    pub from_node: i64,
    pub frames_remaining: i64,
    pub shinespark_frames: i64,
}
