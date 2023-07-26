use async_osc::{OscMessage, OscType};
use serde::ser::{Serialize, Serializer, SerializeStruct};

pub enum OscTypeWrapper {
    Int(i32),
    Float(f32),
    String(String),
    Blob(Vec<u8>),
    Time((u32, u32)),
    Long(i64),
    Double(f64),
    Char(char),
    Color(OscColorWrapper),
    Midi(Vec<u8>),
    Bool(bool),
    Array(Vec<OscTypeWrapper>),
    Nil,
    Inf
}

impl OscTypeWrapper {
    pub fn new(osc_type: OscType) -> Self {
        match osc_type {
            OscType::Int(i) => Self::Int(i),
            OscType::Float(f) => Self::Float(f),
            OscType::String(s) => Self::String(s),
            OscType::Blob(b) => Self::Blob(b),
            OscType::Time(t) => Self::Time(t),
            OscType::Long(l) => Self::Long(l),
            OscType::Double(d) => Self::Double(d),
            OscType::Char(c) => Self::Char(c),
            OscType::Color(c) => Self::Color(OscColorWrapper {
                r: c.red,
                g: c.green,
                b: c.blue,
                a: c.alpha
            }),
            OscType::Midi(m) => Self::Midi(vec![m.port, m.status, m.data1, m.data2]),
            OscType::Bool(b) => Self::Bool(b),
            OscType::Array(a) => Self::Array(a.content.into_iter().map(|osc_type| OscTypeWrapper::new(osc_type)).collect()),
            OscType::Nil => Self::Nil,
            OscType::Inf => Self::Inf
        }
    }
}

#[derive(serde::Serialize)]
pub struct OscColorWrapper {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8
}

#[derive(serde::Serialize)]
pub struct OscMessageWrapper {
    pub address: String,
    pub args: Vec<OscTypeWrapper>
}

impl OscMessageWrapper {
    pub fn new(osc_msg: OscMessage) -> Self {
        let address = osc_msg.addr;
        let args = osc_msg.args.into_iter().map(|osc_type| OscTypeWrapper::new(osc_type)).collect();

        Self {
            address,
            args
        }
    }
}

impl Serialize for OscTypeWrapper {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer {
        match self {
            OscTypeWrapper::Int(i) => {
                let mut state = serializer.serialize_struct("OscTypeWrapper", 2)?;
                state.serialize_field("type", "i")?;
                state.serialize_field("value", i)?;
                state.end()
            },
            OscTypeWrapper::Float(f) => {
                let mut state = serializer.serialize_struct("OscTypeWrapper", 2)?;
                state.serialize_field("type", "f")?;
                state.serialize_field("value", f)?;
                state.end()
            },
            OscTypeWrapper::String(s) => {
                let mut state = serializer.serialize_struct("OscTypeWrapper", 2)?;
                state.serialize_field("type", "s")?;
                state.serialize_field("value", s)?;
                state.end()
            },
            OscTypeWrapper::Blob(b) => {
                let mut state = serializer.serialize_struct("OscTypeWrapper", 2)?;
                state.serialize_field("type", "b")?;
                state.serialize_field("value", b)?;
                state.end()
            },
            OscTypeWrapper::Time((secs_since_1900,frac_secs)) => {
                let mut state = serializer.serialize_struct("OscTypeWrapper", 2)?;
                // NOTE: 'rawNTP' is used instead of 'raw' in OSC.js.
                state.serialize_field("rawNTP", &[secs_since_1900, frac_secs])?;

                let secs_since_1970 = (secs_since_1900 - 2_208_988_800) as f64;
                let decimals = (*frac_secs as f64) / 4_294_967_296_f64;
                // NOTE: 'epochTimeMs' used instead of 'native' in OSC.js.
                state.serialize_field("epochTimeMs", &(((secs_since_1970 + decimals) * 1000.0) as u64))?;
                state.end()
            },
            OscTypeWrapper::Long(h) => {
                let mut state = serializer.serialize_struct("OscTypeWrapper", 2)?;
                state.serialize_field("type", "h")?;
                state.serialize_field("value", h)?;
                state.end()
            },
            OscTypeWrapper::Double(d) => {
                let mut state = serializer.serialize_struct("OscTypeWrapper", 2)?;
                state.serialize_field("type", "d")?;
                state.serialize_field("value", d)?;
                state.end()
            },
            OscTypeWrapper::Char(c) => {
                let mut state = serializer.serialize_struct("OscTypeWrapper", 2)?;
                state.serialize_field("type", "c")?;
                state.serialize_field("value", c)?;
                state.end()
            },
            OscTypeWrapper::Color(r) => {
                let mut state = serializer.serialize_struct("OscTypeWrapper", 2)?;
                state.serialize_field("type", "r")?;
                state.serialize_field("value", r)?;
                state.end()
            },
            OscTypeWrapper::Midi(m) => {
                let mut state = serializer.serialize_struct("OscTypeWrapper", 2)?;
                state.serialize_field("type", "m")?;
                state.serialize_field("value", m)?;
                state.end()
            },
            OscTypeWrapper::Bool(bool) => {
                serializer.serialize_bool(*bool)
            },
            OscTypeWrapper::Array(arr) => {
                serializer.collect_seq(arr)
            },
            OscTypeWrapper::Nil => serializer.serialize_none(),
            // This is the OSC impulse/bang/infinitum message. No idea what it means tho, just following what OSC.js does.
            OscTypeWrapper::Inf => serializer.serialize_f32(1.0),
        }
    }
}