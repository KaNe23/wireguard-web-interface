use serde::{Deserialize, Serialize};
pub mod wg_conf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    Login { username: String, password: String },
    PeerDownload { index: usize },
    UpdatePeerName { index: usize, name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    LoginSuccess { session: String },
    LoginFailure,
    Logout,
    WireGuardConf { config: wg_conf::WireGuardConf },
    Success,
    Failure,
}
