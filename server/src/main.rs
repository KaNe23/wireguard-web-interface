use actix_files::{Files, NamedFile};
use actix_http::cookie::SameSite;
use actix_identity::{CookieIdentityPolicy, Identity, IdentityService};
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use bcrypt::{hash, verify, DEFAULT_COST};
use failure::{bail, Error};
use serde::{Deserialize, Serialize};
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
        .split('\n')
        .filter(|line| line.len() > 7 && &line[..7] == "default")
        .collect::<Vec<_>>();

    let default = match default.first() {
        Some(line) => line,
        None => bail!("no default route found"),
    };

    let link = match default.split(' ').last() {
        Some(link) => link.to_string(),
        None => bail!("could not find default link"),
    };

    Ok(link)
}

fn get_iface_ip(name: String) -> Result<std::net::Ipv4Addr, std::io::Error> {
    let ifaces = get_if_addrs::get_if_addrs()?;
    let err = Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        format!("iface not found with name: {}", name),
    ));

    match ifaces
        .into_iter()
        .filter(|iface| iface.name == name)
        .collect::<Vec<_>>()
        .first()
    {
        Some(iface) => match iface.ip() {
            std::net::IpAddr::V4(ip) => Ok(ip),
            _ => err,
        },
        None => err,
    }
}

fn get_user_by_name(username: String, app_data: &web::Data<AppData>) -> Option<User> {
    if let Ok(users) = app_data.db.all::<User>() {
        for (_, user) in users {
            if user.name == username {
                return Some(user);
            }
        }
    }
    None
}

// ---- Apis ("/api/*") ----

#[post("login")]
async fn login_request(
    data: web::Data<AppData>,
    login_data: web::Json<shared::Request>,
    id: Identity,
) -> impl Responder {
    // get username and password
    let (username, password) = match login_data.0 {
        shared::Request::Login { username, password } => (username, password),
        _ => return web::Json(shared::Response::LoginFailure),
    };

    // search for user with matching username
    let user = match get_user_by_name(username, &data) {
        Some(user) => user,
        _ => return web::Json(shared::Response::LoginFailure),
    };

    // check the password
    if let Ok(result) = verify(&password, &user.hashed_pass) {
        if result {
            id.remember(user.name.to_owned());
            return web::Json(shared::Response::LoginSuccess { session: user.name });
        }
    }

    web::Json(shared::Response::LoginFailure)
}

#[post("logout")]
async fn logout_request(id: Identity) -> impl Responder {
    id.forget();
    web::Json(shared::Response::Logout)
}

#[get("session")]
async fn session_request(id: Identity) -> impl Responder {
    if let Some(name) = id.identity() {
        web::Json(shared::Response::LoginSuccess { session: name })
    } else {
        web::Json(shared::Response::LoginFailure)
    }
}

fn current_wg_config(data: &web::Data<AppData>) -> shared::wg_conf::WireGuardConf {
    let output = Command::new("./wg_wrapper.bin")
        .arg("show")
        .output()
        .unwrap()
        .stdout;

    let config = str::from_utf8(&output).unwrap().to_string();
    let mut wg_config = shared::wg_conf::WireGuardConf::from(config);

    wg_config.interface.dns = INTERFACE_ADDRESS.parse().unwrap();
    if let Ok(ppkeys) = data.db.all::<PubPrivKey>() {
        for peer in &mut wg_config.peers {
            if let Some(ppk) = ppkeys
                .values()
                .filter(|ppk| ppk.public_key == peer.public_key)
                .collect::<Vec<_>>()
                .first()
            {
                peer.private_key = ppk.private_key.clone();
                peer.name = ppk.name.clone();
            };
        }
    }

    wg_config.interface.address.set_ip(data.ip);
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
async fn show_config(id: Identity, data: web::Data<AppData>) -> impl Responder {
    if id.identity().is_some() {
        let wg_config = current_wg_config(&data);
        HttpResponse::Ok().json(shared::Response::WireGuardConf { config: wg_config })
    } else {
        HttpResponse::Forbidden().body("")
    }
}

#[get("new_peer")]
async fn new_peer(id: Identity, data: web::Data<AppData>) -> impl Responder {
    if id.identity().is_some() {
        // get current config
        let mut wg_config = current_wg_config(&data);
        // get last peers allowed ip
        let allowed_ips = if !wg_config.peers.is_empty() {
            let addr = wg_config.peers.last().unwrap().allowed_ips.addr();
            let o = addr.octets();
            ipnet::Ipv4Net::new(std::net::Ipv4Addr::new(o[0], o[1], o[2], o[3] + 1), 32).unwrap()
        } else {
            ipnet::Ipv4Net::new(std::net::Ipv4Addr::new(10, 200, 100, 2), 32).unwrap()
        };
        // generate private key
        let output = Command::new("wg").arg("genkey").output().unwrap().stdout;

        let mut private_key = str::from_utf8(&output).unwrap().to_string();
        private_key.pop(); // remove linefeed

        let mut new_peer = shared::wg_conf::Peer::new();
        new_peer.allowed_ips = allowed_ips;
        // generate public key
        new_peer.set_private_key(&private_key);

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

        new_peer.endpoint = SocketAddrV4::new(data.ip, wg_config.interface.address.port());

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
    id: Identity,
    data: web::Data<AppData>,
    request_data: web::Json<shared::Request>,
) -> impl Responder {
    if id.identity().is_some() {
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

#[post("update_user")]
async fn update_user(
    id: Identity,
    data: web::Data<AppData>,
    request_data: web::Json<shared::Request>,
) -> impl Responder {
    if let Some(username) = id.identity() {
        let (name, old_password, new_password, password_confirmation) = match request_data.0 {
            shared::Request::UpdateUser {
                name,
                old_password,
                new_password,
                password_confirmation,
            } => (name, old_password, new_password, password_confirmation),
            _ => return web::Json(shared::Response::Failure),
        };

        if name == "" || new_password == "" || old_password == "" || password_confirmation == "" {
            return web::Json(shared::Response::Failure);
        }

        let user = match get_user_by_name(username, &data) {
            Some(user) => user,
            _ => return web::Json(shared::Response::Failure),
        };

        if verify(&old_password, &user.hashed_pass).is_ok() && new_password == password_confirmation
        {
            match hash(&new_password, DEFAULT_COST) {
                Ok(hashed_pass) => {
                    let _ = data.db.save_with_id(&User { name, hashed_pass }, "user");
                    return web::Json(shared::Response::Success);
                }
                Err(_) => return web::Json(shared::Response::Failure),
            }
        }

        web::Json(shared::Response::Failure)
    } else {
        web::Json(shared::Response::Failure)
    }
}

#[get("download_peer/{index}")]
async fn download_peer_file(
    id: Identity,
    data: web::Data<AppData>,
    index: web::Path<usize>,
) -> Result<NamedFile, std::io::Error> {
    if id.identity().is_some() {
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
    id: Identity,
    data: web::Data<AppData>,
    index: web::Path<usize>,
) -> impl Responder {
    if id.identity().is_some() {
        let mut wg_config = current_wg_config(&data);
        let peer = &wg_config.peers.remove(index.into_inner());
        let _res = wg_remove_peer(peer);

        match data.db.delete(&peer.allowed_ips.to_string()) {
            Ok(_) => {}
            Err(e) => println!("Could not delete peer: {}", e),
        }

        HttpResponse::Ok().json(shared::Response::WireGuardConf { config: wg_config })
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
    ip: std::net::Ipv4Addr,
    db: jfs::Store,
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let link = match default_route() {
        Ok(link) => link,
        Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
    };

    let ip: std::net::Ipv4Addr = get_iface_ip(link)?;
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
            .data(AppData { ip, db: db.clone() })
            .wrap(IdentityService::new(
                CookieIdentityPolicy::new(&[0; 32])
                    .name("auth-cookie")
                    .same_site(SameSite::Strict)
                    .secure(false),
            ))
            .service(
                web::scope("/api/")
                    .service(login_request)
                    .service(logout_request)
                    .service(new_peer)
                    .service(update_peer_name)
                    .service(download_peer_file)
                    .service(remove_peer)
                    .service(update_user)
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
