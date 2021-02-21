use nix::unistd::{setuid, Uid};
use std::env;
use std::process::Command;
use std::str;

fn main() {
    let _res = setuid(Uid::from_raw(0));

    let args = env::args_os()
        .map(|a| a.into_string().unwrap())
        .collect::<Vec<_>>();

    match args[1].as_str() {
        "show" => {
            run_show();
        }
        "showconf" => {
            run_showconf(&args[2]);
        }
        "add" => {
            run_add_conf(&args[2], &args[3]);
        }
        "remove" => {
            run_remove_peer(&args[2], &args[3]);
        }
        _ => {}
    }
}

fn run_show() {
    let output = Command::new("/usr/bin/wg")
        .args(&["show"])
        .output()
        .unwrap();
    println!("{}", str::from_utf8(&output.stdout).unwrap());
}

fn run_showconf(iface: &str) {
    let output = Command::new("/usr/bin/wg")
        .args(&["showconf", iface])
        .output()
        .unwrap();
    println!("{}", str::from_utf8(&output.stdout).unwrap());
}

fn run_add_conf(iface: &str, path: &str) {
    let _output = Command::new("/usr/bin/wg")
        .args(&["addconf", iface, path])
        .output()
        .unwrap();
}

fn run_remove_peer(iface: &str, key: &str) {
    let _output = Command::new("/usr/bin/wg")
        .args(&["set", iface, "peer", &key, "remove"])
        .output()
        .unwrap();
}
