# osc2wss

Forwards incoming Open Sound Control (OSC) messages to a Web Socket Secure (WSS) server over LAN.

## What/Who it's for

[Hydra](https://hydra.ojack.xyz/), [Gibber](https://gibber.cc/), and other browser-based creative coding environments can't read incoming UDP packets which OSC uses to transmit data (browsers don't have access to UDP).

This project was made as a quick and easy solution for a self-signed HTTPS server which forwards any incoming OSC messages (over UDP transport) to all connected Secure Web Socket clients (over HTTPS/WSS) within the same LAN, so connected clients can access these OSC messages over WebSocket.

Only the devices which are creating/broadcasting the OSC messages should run. If multiple devices are broadcasting OSC messages, then the receiving clients should create one WebSocket connection per broadcasting device.

New TLS private/public key certs for the current local IP address are generated every time the app is run. There is no need to manually setup any certs.

This is for LAN use only, and is not intended for production due to obvious security concerns.

In the far future, when Algoraves (Audio-Visual livecoding in front of an audience) are normalized, and one should deem an Algorave-hacking a serious vulnerability, feel free to replace the provided private key with your own (kekw).

## How to use

1. Download the project's source.
   Otherwise, [Build from source](#build-instructions).
2. Modify [`config.toml`](./config.toml) to reflect the desired OSC port to receive from, and the WebSocket port to serve to.
3. Run the executable.
4. ⚠️ **Before connecting to the websocket, you are required to trust the self-signed certificate in your browser**.
   - Once the WebSocket server is running, key in the local IP address of the device running the WebSocket server into the browser URL and try to access the README.md file: 
   - e.g. if your server's local IP address is `10.0.0.2` and is hosted on port 2700 as per [`config.toml`](./config.toml) try to access `https://10.0.0.2:2700/README.md`
   - You should get a warning message about the certificate being from a non-trusted authority.
   - Trust the certificate by clicking Advanced > Proceed to unsafe (this differs depending on your browser).
   - You will need to repeat this step every time the WebSocket server is restarted as it generates new certs every time.
5. After doing the above, you should be able to retrieve OSC messages like so:

```js
// exclude 'let' if making a top-level variable in Hydra.

let ws = new WebSocket('wss://127.0.0.1:2700'); // use WSS server's local IP & port.

ws.onmessage = (e) => {
  let data = JSON.parse(e.data);
  console.log(data); // prints the OSC data as JSON in the browser console
}
```

The above OSC `data` object should be almost identical to that of [OSC.js](https://github.com/colinbdclark/osc.js/), with two exceptions:

- OSC time data is represented with the properties `rawNTP` instead of `raw`, and `epochTimeMs` instead of `native` for better readability.
- Only OSC Messages are supported. **OSC Bundles are not supported**.

## Requirements

`openssl` must be installed.

On Windows, using the [chocolatey](https://chocolatey.org/install) package manager is recommended.

    choco install openssl

- Set up environment variables (just to be safe):
  - `OPENSSL_CFG` should point to `/path/to/openssl/bin/openssl.cfg`
  - `OPENSSL_DIR` should point to `/path/to/openssl/`
  - `PATH` should contain `/path/to/openssl/bin/`




## Build instructions

🟦 Install `rustc` and `cargo` via rustup: https://rustup.rs/

🟦 Ensure `openssl` is set up as per requirements above.

🟦 Run `cargo build --release`.
