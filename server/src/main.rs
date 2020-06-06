use actix_files::{Files, NamedFile};
use actix_rt;
use actix_session::{CookieSession, Session};
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use bcrypt::{hash, verify, DEFAULT_COST};
use failure::{bail, Error};
use serde::{Deserialize, Serialize};
use shared;
use std::io::Write;
use std::net::SocketAddrV4;
use std::process::Command;
use std::str;
use tempfile::NamedTempFile;

const INTERFACE_ADDRESS: &str = "10.200.100.1";

fn default_route() -> Result<String, Error> {
    let output = Command::new("route").output()?;
    let output_string = str::from_utf8(&output.stdout)?.to_string();

    let default = output_string
        .split("\n")
        .filter(|line| line.len() > 7 && &line[..7] == "default")
        .collect::<Vec<_>>();

    let default = match default.first() {
        Some(line) => line,
        None => bail!("no default route found"),
    };

    let link = match default.split(" ").last() {
        Some(link) => link.to_string(),
        None => bail!("could not find default link"),
    };

    Ok(link)
}

fn get_iface_ip(name: String) -> Result<std::net::IpAddr, std::io::Error> {
    let ifaces = get_if_addrs::get_if_addrs()?;
    match ifaces
        .into_iter()
        .filter(|iface| iface.name == name)
        .collect::<Vec<_>>()
        .first()
    {
        Some(iface) => Ok(iface.ip()),
        None => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("iface not found with name: {}", name),
            ))
        }
    }
}

// ---- Apis ("/api/*") ----

#[post("login")]
async fn login_request(
    session: Session,
    data: web::Data<AppData>,
    login_data: web::Json<shared::Request>,
) -> impl Responder {
    match login_data.0 {
        shared::Request::Login { username, password } => {
            if let Ok(user) = data.db.get::<User>("user") {
                if let Ok(res) = verify(password, &user.hashed_pass) {
                    if res {
                        let _res = session.set("user", username);
                        match session.get::<String>("user") {
                            Ok(Some(user)) => {
                                web::Json(shared::Response::LoginSuccess { session: user })
                            }
                            _ => web::Json(shared::Response::LoginFailure),
                        }
                    } else {
                        web::Json(shared::Response::LoginFailure)
                    }
                } else {
                    web::Json(shared::Response::LoginFailure)
                }
            } else {
                web::Json(shared::Response::LoginFailure)
            }
        }
        _ => web::Json(shared::Response::LoginFailure),
    }
}

#[post("logout")]
async fn logout_request(session: Session) -> impl Responder {
    session.clear();
    web::Json(shared::Response::Logout)
}

#[get("session")]
async fn session_request(session: Session) -> impl Responder {
    match session.get::<String>("user") {
        Ok(Some(user)) => web::Json(shared::Response::LoginSuccess { session: user }),
        _ => web::Json(shared::Response::LoginFailure),
    }
}

fn current_wg_config(data: &web::Data<AppData>) -> shared::wg_conf::WireGuardConf {
    let output = Command::new("./wg_wrapper.bin")
        .arg("show")
        .output()
        .unwrap()
        .stdout
        .clone();

    let config = str::from_utf8(&output).unwrap().to_string();
    let mut wg_config = shared::wg_conf::WireGuardConf::from(config.clone());

    wg_config.interface.dns = INTERFACE_ADDRESS.parse().unwrap();
    if let Ok(ppkeys) = data.db.all::<PubPrivKey>() {
        for peer in &mut wg_config.peers {
            match ppkeys
                .values()
                .filter(|ppk| ppk.public_key == peer.public_key)
                .collect::<Vec<_>>()
                .first()
            {
                Some(ppk) => {
                    peer.private_key = ppk.private_key.clone();
                    peer.name = ppk.name.clone();
                }
                None => {}
            }
        }
    }

    match data.ip {
        std::net::IpAddr::V4(ip) => wg_config.interface.address.set_ip(ip),
        _ => {}
    }

    wg_config.interface.private_key = "(hidden)".to_string();

    wg_config
}

fn wg_add_peer(peer: &shared::wg_conf::Peer) -> Result<(), std::io::Error> {
    let mut file = NamedTempFile::new()?;
    writeln!(file, "{}", peer.to_string())?;
    let path = file.path();
    let _c = Command::new("./wg_wrapper.bin")
        .args(&["add", path.to_str().unwrap()])
        .spawn()
        .unwrap()
        .wait();
    Ok(())
}

fn wg_remove_peer(peer: &shared::wg_conf::Peer) -> Result<(), std::io::Error> {
    let _c = Command::new("./wg_wrapper.bin")
        .args(&["remove", &peer.public_key])
        .spawn()
        .unwrap()
        .wait();
    Ok(())
}

#[get("config")]
async fn show_config(session: Session, data: web::Data<AppData>) -> impl Responder {
    if session.get::<String>("user").unwrap() != None {
        let wg_config = current_wg_config(&data);
        return HttpResponse::Ok().json(shared::Response::WireGuardConf { config: wg_config });
    }
    HttpResponse::Forbidden().body("")
}

#[get("new_peer")]
async fn new_peer(session: Session, data: web::Data<AppData>) -> impl Responder {
    if session.get::<String>("user").unwrap() != None {
        // get current config
        let mut wg_config = current_wg_config(&data);
        // get last peers allowed ip
        let allowed_ips = if wg_config.peers.len() >= 1 {
            let addr = wg_config.peers.last().unwrap().allowed_ips.addr();
            let o = addr.octets();
            ipnet::Ipv4Net::new(std::net::Ipv4Addr::new(o[0], o[1], o[2], o[3] + 1), 32).unwrap()
        } else {
            ipnet::Ipv4Net::new(std::net::Ipv4Addr::new(10, 200, 100, 2), 32).unwrap()
        };
        // generate private key
        let output = Command::new("wg")
            .arg("genkey")
            .output()
            .unwrap()
            .stdout
            .clone();

        let mut private_key = str::from_utf8(&output).unwrap().to_string();
        private_key.pop(); // remove linefeed

        let mut new_peer = shared::wg_conf::Peer::new();
        new_peer.allowed_ips = allowed_ips;
        // generate public key
        new_peer.set_private_key(private_key);

        new_peer.name = format!("Peer {}", wg_config.peers.len() + 1);

        match data.db.save_with_id(
            &PubPrivKey {
                private_key: new_peer.private_key.clone(),
                public_key: new_peer.public_key.clone(),
                name: new_peer.name.clone(),
            },
            &new_peer.allowed_ips.to_string(),
        ) {
            Ok(_) => {}
            Err(e) => println!("Could not save PubPrivKey {}", e),
        }

        match data.ip {
            std::net::IpAddr::V4(ip) => {
                new_peer.endpoint = SocketAddrV4::new(ip, wg_config.interface.address.port())
            }
            _ => {}
        }

        match wg_add_peer(&new_peer) {
            Ok(_) => {}
            Err(e) => println!("Ehh: {}", e),
        }

        wg_config.peers.push(new_peer);

        return HttpResponse::Ok().json(shared::Response::WireGuardConf { config: wg_config });
    }
    HttpResponse::Forbidden().body("")
}

#[post("update_peer_name")]
async fn update_peer_name(
    session: Session,
    data: web::Data<AppData>,
    request_data: web::Json<shared::Request>,
) -> impl Responder {
    if session.get::<String>("user").unwrap() != None {
        match request_data.0 {
            shared::Request::UpdatePeerName { index, name } => {
                let mut wg_config = current_wg_config(&data);

                let peer = &mut wg_config.peers[index];
                peer.name = name;

                let _ = data.db.save_with_id(
                    &PubPrivKey {
                        private_key: peer.private_key.clone(),
                        public_key: peer.public_key.clone(),
                        name: peer.name.clone(),
                    },
                    &peer.allowed_ips.to_string(),
                );
                web::Json(shared::Response::Success)
            }
            _ => web::Json(shared::Response::Failure),
        }
    } else {
        web::Json(shared::Response::Failure)
    }
}

#[get("download_peer/{index}")]
async fn download_peer_file(
    session: Session,
    data: web::Data<AppData>,
    index: web::Path<usize>,
) -> Result<NamedFile, std::io::Error> {
    if session.get::<String>("user").unwrap() != None {
        let wg_config = current_wg_config(&data);
        let peer = &wg_config.peers[index.into_inner()];
        let mut tmp = tempfile::tempfile().unwrap();
        let _res = write!(tmp, "{}", wg_config.peer_config(peer));
        Ok(NamedFile::from_file(tmp, "wg.conf")?)
    } else {
        Err(std::io::Error::new(std::io::ErrorKind::Other, "No Session"))
    }
}

#[get("remove_peer/{index}")]
async fn remove_peer(
    session: Session,
    data: web::Data<AppData>,
    index: web::Path<usize>,
) -> impl Responder {
    if session.get::<String>("user").unwrap() != None {
        let mut wg_config = current_wg_config(&data);
        let peer = &wg_config.peers.remove(index.into_inner());
        let _res = wg_remove_peer(peer);

        match data.db.delete(&peer.allowed_ips.to_string()) {
            Ok(_) => {}
            Err(e) => println!("Could not delete peer: {}", e),
        }

        return HttpResponse::Ok().json(shared::Response::WireGuardConf { config: wg_config });
    } else {
        HttpResponse::Forbidden().body("")
    }
}

async fn index() -> impl Responder {
    NamedFile::open("./client/index.html")
}

#[derive(Serialize, Deserialize, Debug)]
struct PubPrivKey {
    private_key: String,
    public_key: String,
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct User {
    name: String,
    hashed_pass: String,
}

struct AppData {
    ip: std::net::IpAddr,
    db: jfs::Store,
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let link = match default_route() {
        Ok(link) => link,
        Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
    };

    let ip = get_iface_ip(link)?;
    let mut cfg = jfs::Config::default();
    cfg.single = true; // false is default
    let db = jfs::Store::new_with_cfg("data", cfg).unwrap();

    //if let Ok(user) = db.get::<User>("user") {
    //    println!("{:?}", user);
    //} else {
    //    let name = "admin".to_string();
    //    match hash("secure"
    //        DEFAULT_COST,
    //    ) {
    //        Ok(hashed_pass) => {
    //            let _res = db.save_with_id(&User { name, hashed_pass }, "user");
    //            ()
    //        }
    //        Err(e) => println!("Could not hash pass {}", e),
    //    }
    //}

    HttpServer::new(move || {
        App::new()
            .wrap(CookieSession::signed(&[0; 32]).secure(false))
            .data(AppData { ip, db: db.clone() })
            .service(
                web::scope("/api/")
                    .service(login_request)
                    .service(logout_request)
                    .service(new_peer)
                    .service(update_peer_name)
                    .service(download_peer_file)
                    .service(remove_peer)
                    .service(session_request)
                    .service(show_config)
                    .default_service(web::route().to(web::HttpResponse::NotFound)),
            )
            .service(Files::new("/public", "./client/public"))
            .service(Files::new("/pkg", "./client/pkg"))
            .default_service(web::route().to(index))
    })
    .bind("127.0.0.1:8000")?
    .run()
    .await
}
