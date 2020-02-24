use ipnet::Ipv4Net;
use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, SocketAddrV4};

#[cfg(target_arch = "x86_64")]
use std::io::Write;
#[cfg(target_arch = "x86_64")]
use std::net::IpAddr;
#[cfg(target_arch = "x86_64")]
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interface {
    pub address: SocketAddrV4,
    pub private_key: String,
    pub public_key: String,
    pub dns: Ipv4Addr,
}

#[cfg(target_arch = "x86_64")]
impl Interface {
    fn new() -> Self {
        Interface {
            address: SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 51820),
            private_key: "".to_string(),
            public_key: "".to_string(),
            dns: Ipv4Addr::new(0, 0, 0, 0),
        }
    }

    fn set_private_key(self: &mut Self, private_key: String) {
        self.private_key = private_key.clone();
        self.public_key = WireGuardConf::gen_public_key(private_key);
    }
}

#[cfg(not(target_arch = "x86_64"))]
impl Interface {
    fn new() -> Self {
        Interface {
            address: SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 51820),
            private_key: "".to_string(),
            public_key: "".to_string(),
            dns: Ipv4Addr::new(0, 0, 0, 0),
        }
    }

    fn set_private_key(self: &mut Self, private_key: String) {
        self.private_key = private_key;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peer {
    pub public_key: String,
    pub private_key: String,
    pub endpoint: SocketAddrV4,
    pub allowed_ips: Ipv4Net,
}

#[cfg(target_arch = "x86_64")]
impl Peer {
    pub fn new() -> Self {
        let ethernet = &get_if_addrs::get_if_addrs()
            .unwrap()
            .into_iter()
            .filter(|i| &i.name[0..2] == "wl")
            .collect::<Vec<_>>()[0];

        let mut address = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 51820);
        match ethernet.ip() {
            IpAddr::V4(addr) => {
                address.set_ip(addr);
            }
            IpAddr::V6(_addr) => {}
        };
        Peer {
            public_key: "".to_string(),
            private_key: "".to_string(),
            endpoint: SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 8080),
            allowed_ips: Ipv4Net::new(Ipv4Addr::new(0, 0, 0, 0), 16).unwrap(),
        }
    }

    pub fn set_private_key(self: &mut Self, private_key: String) {
        self.private_key = private_key.clone();
        self.public_key = WireGuardConf::gen_public_key(private_key);
    }
}

#[cfg(not(target_arch = "x86_64"))]
impl Peer {
    fn new() -> Self {
        Peer {
            public_key: "".to_string(),
            private_key: "".to_string(),
            endpoint: SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 8080),
            allowed_ips: Ipv4Net::new(Ipv4Addr::new(0, 0, 0, 0), 16).unwrap(),
        }
    }
}

impl ToString for Peer {
    fn to_string(&self) -> String {
        let mut peer = "[Peer]\n".to_string();
        peer.push_str(&format!("PublicKey = {}\n", self.public_key));
        peer.push_str(&format!(
            "AllowedIPs = {}\n\n",
            self.allowed_ips.to_string()
        ));
        peer
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireGuardConf {
    pub interface: Interface,
    pub peers: Vec<Peer>,
}

impl WireGuardConf {
    pub fn new() -> Self {
        WireGuardConf {
            interface: Interface::new(),
            peers: vec![],
        }
    }

    pub fn peer_config(&self, peer: &Peer) -> String {
        let mut peer_conf = "[Interface]\n".to_string();
        // ListenPort
        peer_conf.push_str(&format!("DNS = {}\n", self.interface.dns));
        // PrivateKey
        peer_conf.push_str(&format!("Address = {}\n", peer.allowed_ips.to_string()));
        // Address
        peer_conf.push_str(&format!("PrivateKey = {}\n\n", peer.private_key));
        // [Peer]
        peer_conf.push_str("[Peer]\n");
        // PublicKey
        peer_conf.push_str(&format!("PublicKey = {}\n", self.interface.public_key));
        // AllowedIPs
        peer_conf.push_str(&format!("AllowedIPs = {}/32\n", self.interface.dns));
        // Endpoint
        peer_conf.push_str(&format!(
            "Endpoint = {}\n",
            self.interface.address.to_string()
        ));
        peer_conf
    }
}

impl ToString for WireGuardConf {
    fn to_string(&self) -> String {
        let mut wg_conf = "[Interface]\n".to_string();
        wg_conf.push_str(&format!("PrivateKey = {}\n", self.interface.private_key));
        wg_conf.push_str(&format!(
            "ListenPort = {}\n\n",
            self.interface.address.port()
        ));
        for peer in &self.peers {
            wg_conf.push_str("[Peer]\n");
            wg_conf.push_str(&format!("PublicKey = {}\n", peer.public_key));
            wg_conf.push_str(&format!(
                "AllowedIPs = {}\n\n",
                peer.allowed_ips.to_string()
            ));
        }
        wg_conf
    }
}

#[cfg(target_arch = "x86_64")]
impl WireGuardConf {
    pub fn gen_public_key(private_key: String) -> String {
        // calculate public key
        let mut wg = Command::new("wg")
            .arg("pubkey")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn wg");
        {
            let stdin = wg.stdin.as_mut().expect("Failed to open stdin");
            stdin
                .write_all(private_key.as_bytes())
                .expect("Failed to write to stdin");
        }

        let output = wg.wait_with_output().expect("Failed to read stdout");
        let mut public_key = String::from_utf8_lossy(&output.stdout).to_string();
        public_key.pop(); // remove newline at the end
        public_key
    }
}

impl Default for WireGuardConf {
    fn default() -> WireGuardConf {
        WireGuardConf::new()
    }
}

impl From<String> for WireGuardConf {
    fn from(config: String) -> Self {
        let lines = config.split("\n");
        let mut curr_block = Block::None;
        let mut peers: Vec<Peer> = vec![];
        let mut interface = Interface::new();

        for line in lines {
            match line {
                "[Interface]" => curr_block = Block::Interface,
                "[Peer]" => {
                    curr_block = Block::Peer;
                    peers.push(Peer::new())
                }
                "" => curr_block = Block::None,
                _ if curr_block == Block::Interface => {
                    parse_interface_attribute(line, &mut interface)
                }
                _ if curr_block == Block::Peer => {
                    // last peer is always at the end
                    let last_index = peers.len() - 1;
                    // unwrap is safe here, because [Peer] match
                    // was called at least once
                    parse_peer_attribute(line, peers.get_mut(last_index).unwrap())
                }
                _ => {}
            }
        }

        WireGuardConf {
            interface: interface,
            peers: peers,
        }
    }
}

#[derive(PartialEq)]
enum Block {
    Interface,
    Peer,
    None,
}

fn parse_interface_attribute(attr: &str, interface: &mut Interface) {
    let split = attr.split(" = ").collect::<Vec<_>>();
    let (name, value) = (split[0], split[1]);
    match name {
        "ListenPort" => interface
            .address
            .set_port(value.to_string().parse::<u16>().unwrap()),
        "PrivateKey" => interface.set_private_key(value.to_string()),
        "Address" => interface.address = value.to_string().parse().unwrap(),
        _ => {}
    }
}
fn parse_peer_attribute(attr: &str, peer: &mut Peer) {
    let split = attr.split(" = ").collect::<Vec<_>>();
    let (name, value) = (split[0], split[1]);

    match name {
        "PublicKey" => peer.public_key = value.to_string(),
        "AllowedIPs" => peer.allowed_ips = value.to_string().parse().unwrap(),
        "Endpoint" => peer.endpoint = value.to_string().parse().unwrap(),
        _ => {}
    }
}
