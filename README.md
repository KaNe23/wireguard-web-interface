### Simple webinterface to create wireguard connection to a server.

Install cargo-make:
```
cargo install --force cargo-make
```

Prepare a neat little release:
```
cargo build --release
cargo make build_release

sudo chown root ./target/release/wg_wrapper
sudo chmod 6755 ./target/release/wg_wrapper

mkdir build -p

cp ./target/release/wg_wrapper build/wg_wrapper.bin
cp ./target/release/server build/server

mkdir ./build/client/ -p
cp -r ./client/index.html ./build/client/
cp -r ./client/pkg ./build/client/
cp -r ./client/public ./build/client/
cd build

tar -zcvf ../release.tar.gz *
```

The project has three parts:
1. `server` implemented with `actix`
2. `client` implemented with `seed`
3. `wg_wrapper` simple wrapper that calls the wireguard tool `wg` that can be used with the setuid bit set

#### Server

The server exposes a simple API to create and delete wireguard peers.
It is possible to rename the peers and download the config.
The password and username can be updated, but has to be set on the first run.

You can create this example config (`data.json`):
```json
{"user":{"hashed_pass":"$2b$12$hdOnw77DyD2YwuKvaZYbIuMlNADxwqXgvyo3LjCoLTcXRimw01h32","name":"admin"}}
```
for a simple user:

user: admin

pass: admin

Be aware, that the private key for each peer is also saved in the json store on the server
to generate the wireguard peer configuration.
