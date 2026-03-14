pub mod client;
pub mod commands;

pub use client::{RtlTcpClient, RtlTcpHeader};
pub use commands::{
    encode_command, SET_AGC_MODE, SET_FREQUENCY, SET_GAIN, SET_GAIN_MODE, SET_SAMPLE_RATE,
};
