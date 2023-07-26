use async_osc::{OscPacket, OscSocket};
use futures::{SinkExt, StreamExt};
use local_ip_address::local_ip;
use openssl::rsa::Rsa;
use rcgen::{date_time_ymd, CertificateParams, DistinguishedName};
use serde_json;
use tokio::sync::broadcast;
use warp::Filter;

use std::{fs, net::SocketAddr};

mod config;
mod types;

use config::ConfigToml;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate new X.509 pub/priv key cert every time the server starts
    // This is necessary since the computer may have a different local ip address every time,
    // and the hosted WSS server needs to work over LAN, so 'localhost' wouldn't suffice.

    let local_ip = local_ip().expect("failed to get local ip address");
    let cert_domain_names = vec!["localhost".to_string(), local_ip.to_string()];

    let mut cert_params: CertificateParams = Default::default();
    cert_params.not_before = date_time_ymd(2023, 07, 01);
    cert_params.not_after = date_time_ymd(4096, 01, 01);
    cert_params.distinguished_name = DistinguishedName::new();
    cert_params.alg = &rcgen::PKCS_RSA_SHA256;

    fs::create_dir_all("certs")?; // make certs dir if it doesn't exist

    let priv_key_pem = {
        // If local priv key not already created, make one.
        if let Ok(pk) = fs::read("./certs/priv_key_rsa.pem") {
            pk
        } else {
            let rsa = Rsa::generate(2048)?;
            let rsa_pem_str = rsa.private_key_to_pem()?;
            fs::write("./certs/priv_key_rsa.pem", &rsa_pem_str)?;
            rsa_pem_str
        }
    };
    let priv_key =
        Rsa::private_key_from_pem(&priv_key_pem).expect("couldn't parse priv_key_rsa.pem");
    let pkey =
        openssl::pkey::PKey::from_rsa(priv_key).expect("couldn't convert priv_key_rsa.pem to pkey");
    let key_pair_pem = String::from_utf8(
        pkey.private_key_to_pem_pkcs8()
            .expect("fail convert to pem pkcs8"),
    )
    .unwrap();

    let key_pair = rcgen::KeyPair::from_pem(&key_pair_pem).expect("failed to make KeyPair");

    cert_params.key_pair = Some(key_pair);

    let cert = rcgen::Certificate::from_params(cert_params).expect("failed to make Certificate");
    let cert_pem_serialized = cert
        .serialize_pem()
        .expect("failed to serialize cert to pem");
    fs::write("./certs/cert.pem", &cert_pem_serialized.as_bytes())
        .expect("failed to write to file certs/cert.pem");
    fs::write(
        "./certs/key.pem",
        &cert.serialize_private_key_pem().as_bytes(),
    )
    .expect("failed to write to file certs/key.pem");

    println!(
        "Created TLS cert for domains: {}",
        cert_domain_names.join(", ")
    );

    // End of cert generation.

    // _____________________________________________________________________________________________________________________
    //
    // Read config.toml
    // _____________________________________________________________________________________________________________________

    let config_toml_str = fs::read_to_string("./config.toml").expect("failed to read config.toml");
    let configs: ConfigToml =
        toml::from_str(&config_toml_str).expect("Failed to parse config.toml");

    // Setup OSC receiver.

    let osc_addr = format!("{}:{}", local_ip, configs.osc_port);
    let mut osc_socket = OscSocket::bind(&osc_addr)
        .await
        .expect(&format!("Failed to bind osc listener at {}", osc_addr));

    // internal broadcast channels
    let (osc_tx, mut osc_rx) = broadcast::channel(16);
    let osc_tx_for_subscribing = osc_tx.clone();

    tokio::task::spawn(async move {
        let mut warned_bundle = false;

        while let Some(osc_packet) = osc_socket.next().await {
            let (osc_packet, peer_addr) = osc_packet.unwrap();
            match osc_packet {
                OscPacket::Message(osc_msg) => {
                    let json_string =
                        serde_json::to_string(&types::OscMessageWrapper::new(osc_msg)).unwrap();

                    if configs.debug {
                        println!("Received OSC message from {}: {}", peer_addr, json_string);
                    }
                    osc_tx.send(json_string).unwrap();
                }
                OscPacket::Bundle(_) => {
                    if !warned_bundle {
                        println!("Warning: Received an OSC Bundle from {}, but it is not currently supported and will be ignored.", peer_addr);
                        warned_bundle = true;
                    }
                }
            }
        }
    });

    println!("OSC receiver is listening at {}", osc_addr);

    // _____________________________________________________________________________________________________________________
    //
    // Setup websocket handler. WSS path is root: /
    // _____________________________________________________________________________________________________________________

    let websocket_path = warp::path::end() // <- Specifies Root path "https://localhost/"
        .and(warp::ws())
        .and(warp::any().map(move || osc_tx_for_subscribing.clone()))
        .and(warp::filters::addr::remote()) // to get client ip addr as param
        .map(
            |ws: warp::ws::Ws,
             osc_tx: tokio::sync::broadcast::Sender<String>,
             addr: Option<SocketAddr>| {
                ws.on_upgrade(move |mut websocket| {
                    let ip_str = addr.and_then(|x| Some(x.ip().to_string()))
                        .unwrap_or("unknown ip".to_string());
                    println!("Websocket client connected: {}", ip_str);
                    async move {
                        let mut osc_rx = osc_tx.subscribe();
                        loop {
                            match osc_rx.recv().await {
                                Ok(msg) => {
                                    let send_res = websocket
                                        .send(warp::ws::Message::text(msg))
                                        .await;
                                    match send_res {
                                        Ok(_) => {},
                                        Err(e) => {
                                            println!("Closing websocket connection with client {} due to disconnection/error: {}", ip_str, e);
                                            let _ = websocket.close().await;
                                            break;
                                        },
                                    }
                                },
                                Err(e) => {
                                    println!("FATAL: couldn't recv from internal broadcast channel osc_rx: {}", e);
                                    break;
                                }
                            }
                        }
                    }
                })
            },
        );

    // _____________________________________________________________________________________________________________________
    //
    // Setup Warp and serve
    // _____________________________________________________________________________________________________________________

    let pwd = std::env::current_dir().expect("failed to get present working directory");
    let routes = warp::get().and(websocket_path.or(warp::fs::dir(pwd)));

    println!(
        "WSS server started at wss://{}:{}",
        local_ip, configs.wss_port
    );

    warp::serve(routes)
        .tls()
        .cert_path("./certs/cert.pem")
        .key_path("./certs/key.pem")
        .run(([0, 0, 0, 0], configs.wss_port))
        .await;

    Ok(())
}
