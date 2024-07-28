#!/bin/bash -xe
HOST="$1"
shift
PROJECT=trading
HOST_USER=trading
CROSS_TARGET=x86_64-unknown-linux-gnu
PACKAGES="trading_be"
for package in $PACKAGES; do
    cargo zigbuild --target=$CROSS_TARGET --package $package --release
done
ssh $HOST_USER@$HOST "mkdir -p $PROJECT/bin $PROJECT/log $PROJECT/etc ~/.config/systemd/user"
# ssh $HOST_USER@$HOST "loginctl enable-linger $HOST_USER"
BINARIES=`find target/x86_64-unknown-linux-gnu/release -maxdepth 1 -type f ! -name "*.*"`

rsync -avizh $BINARIES $HOST_USER@$HOST:$PROJECT/bin/
ssh $HOST "sudo find /home/$HOST_USER/$PROJECT/bin/* -exec setcap 'cap_net_bind_service=+ep' {} \;"
#rsync -avizh etc/config.prod.json $HOST_USER@$HOST:$PROJECT/etc/config.json
rsync -avizh etc/systemd/*.service $HOST_USER@$HOST:~/.config/systemd/user/
ssh $HOST_USER@$HOST 'bash -s' < scripts/restart_services.sh
#scripts/upload_docs.sh

