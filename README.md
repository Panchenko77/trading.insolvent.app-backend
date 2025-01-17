# Cross-Exchange Pair Trading

## Crates included in this workspace

| Crate              | Feature                          | Type     |
|--------------------|----------------------------------|----------|
| codegen            | endpoint lib crate generator     | bin, lib |
| build              | endpoint lib crate               | lib      |
| lib                | common across projects           | lib      |
| service/shared/api | api crate shared across services | lib      |
| service/user       | user (core) service crate        | bin      |
| service/auth       | log in service crate             | bin      |

## Backend Setup Instruction

this is the server side instruction to

- get the backend code up and running on the server
- set up a /usr/local/trading/user directory with project executable and config
- open encrypted port 8443 for endpoint
- create a user system service and run the code as a service

### Install dependencies

##### update apt package list

```
sudo apt update
```

##### install git, for cloning/pulling remote repository

```
sudo apt install git
```

##### install cargo/rustfmt, for building/running the rust project

```
sudo apt install cargo
sudo apt install rustfmt
```

##### install certbot, for setting up encrypted port connection

```
sudo add-apt-repository ppa:certbot/certbot
sudo apt-get install certbot
```

### Prepare backend executable and config files

##### set up a user service directory for final server resources

```
mkdir -p /usr/local/trading_be
```

##### log into the git cli tool

```
gh auth login
```

##### clone the backend source code

```
gh repo clone git@github.com:pathscale/trading.insolvent.app-backend.git
```

### Generate public and private keys

```
certbot certonly --non-interactive --agree-tos --email "YOUR_EMAIL_HERE" --standalone --preferred-challenges http -d trading-be.insolvent.app
```

### Install the debian package

The debian package includes:

- binary
- config json
- systemd service
- controller script
  Installing debian package updates will automatically start the service.
  either download the debian config file from the CDN or generate from the `trading.insolvent.app-backend` repo using

```
cargo deb --deb-version 1.0.$GITHUB_RUN_NUMBER --no-strip -p trading_be
```

install packge

```
sudo dpkg -i trading_be_1.0.0.deb
```

##### check if the service is running

```
systemctl --user status trading_user.service
```

##### (OPTIONAL) stop service

```
systemctl --user stop trading_user.service
```

##### (OPTIONAL) purge service

purging service will clear the configs, logs and database file that was generated by this program

```
sudo dpkg --purge trading_be
```

### Obtain ciphertext and enable trading

#### Generate encrypted private key ciphertext

git pull enc_file code

```
git pull git@github.com:kanekoshoyu/enc_file.git
```

make a file, `hyper.key`, to store the ETH wallet private key for encryption

```
code ./hyper.key
```

run the chacha_poly code

```
cargo run
```

- select 1 to create encryption key, it prints the `encryption key` as below

```
Keys found in key.file:
{"YOUR_KEY_NAME_HERE": "YOUR_ENCRYPTION_KEY_HERE"}
```

- select 3 to encrypt the hypper.key content (wallet private key) and get `hyper.key.crpt`, which has the `ciphertext`
  as the content

#### Activate trading with ciphertext and encryption key

while the service is running, first send command to register `ciphertext` per exchange

```
{
  "method": 21000,
  "seq": 0,
  "params": {
    "keys": [
      {
        "exchange": "hyperliquid",
        "accountId": "YOUR_WALLET_ADDRESS_HERE",
        "ciphertext": "YOUR_CIPHERTEXT_HERE"
      }
    ]
  }
}
```

it should return `success: true` when registered correctly

```
{"type":"Immediate","method":21000,"seq":0,"params":{"success":true}}
``` 

and then activate trading by providing `encryption key` as well (run only once!)

```
{
  "method": 21010,
  "seq": 0,
  "params": {
    "encryptionKey": "YOUR_ENCRYPTION_KEY_HERE",
    "keys": [
      {
        "exchange": "hyperliquid",
        "accountId": "YOUR_WALLET_ADDRESS_HERE"
      }
    ]
  }
}
```

it should return `success: true` when hyper trade manager is activated properly

```
{"type":"Immediate","method":21010,"seq":0,"params":{"success":true}}
```

it return false when it the trade manager was already activated, or there is a problem with the activation

## Deploy to AWS EC2

- Create Instance
- Forward SSH Port 443 && TCP Port 8443

connect server

```
ssh -i "aws.pem" ubuntu@ec2-0.0.0.0.ap-northeast-1.compute.amazonaws.com
```

install debian package

```
wget https://cefi.insolvent.app/trading-be_1.0.999_amd64.deb 
sudo dpkg -i trading-be_1.0.267_amd64.deb
```

## Trading Terminal

endpoints needed

- [x] UserSubPrice
- [x] UserSubPosition
- [x] UserSubOrders
- [ ] UserSubTrades
- [x] UserCancelOrClosePosition
- [x] UserPlaceOrderLimit
- [x] UserPlaceOrderMarket
- [x] UserCancelOrder