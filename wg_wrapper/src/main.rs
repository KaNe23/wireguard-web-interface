use nix::unistd::{setuid, Uid};
use std::env;
use std::process::Command;
use std::str;

fn main() {
    let _res = setuid(Uid::from_raw(0));

    let args = env::args_os()
        .into_iter()
        .map(|a| a.into_string().unwrap())
        .collect::<Vec<_>>()
        .clone();

    match args[1].as_str() {
        "show" => {
            run_show();
        }
        "add" => {
            run_add_conf(args[2].clone());
        }
        "remove" => {
            run_remove_peer(args[2].clone());
        }
        _ => {}
    }
}

fn run_show() {
    let output = Command::new("/usr/bin/wg")
        .args(&["showconf", "wg0"])
        .output()
        .unwrap();
    println!("{}", str::from_utf8(&output.stdout).unwrap());
}

fn run_add_conf(path: String) {
    let _output = Command::new("/usr/bin/wg")
        .args(&["addconf", "wg0", &path])
        .output()
        .unwrap();
}

fn run_remove_peer(key: String) {
    let _output = Command::new("/usr/bin/wg")
        .args(&["set", "wg0", "peer", &key, "remove"])
        .output()
        .unwrap();
}
