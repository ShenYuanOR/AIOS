//! Frozen syscall table. Plugins must not extend this enum.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Syscall {
    Spawn,
    Attach,
    Reap,
    CapCall,
    Audit,
    SecretGet,
    Quota,
    Bootstrap,
    Rescue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaseState {
    Running,
    Idle,
    Stopped,
    Archived,
    Tombstoned,
    Purged,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lease {
    pub project_id: String,
    pub room_id: String,
    pub state: LeaseState,
    pub disk: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub op: Syscall,
    #[serde(default)]
    pub project_id: String,
    #[serde(default)]
    pub room_id: String,
    #[serde(default)]
    pub cap: String,
    #[serde(default)]
    pub args: serde_json::Value,
    #[serde(default)]
    pub secret_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub ok: bool,
    pub error: Option<String>,
    pub body: serde_json::Value,
}
