Simple webinterface to create wireguard connection to a server.

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

TODO: Write how this project is structured. (client, server and wg_wrapper)
