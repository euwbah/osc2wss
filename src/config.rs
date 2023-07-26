use serde::Deserialize;


#[derive(Deserialize)]
pub struct ConfigToml {
    pub wss_port: u16,
    pub osc_port: u16,
    pub debug: bool
}