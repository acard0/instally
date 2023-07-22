#![allow(dead_code, unused_variables)]

pub mod http;
pub mod workloads;
pub mod archiving;
pub mod helpers;
pub mod extensions;
pub mod factory;
pub mod scripting;
pub mod error;

pub(crate) mod target;
#[cfg(target_os = "windows")]
pub use target::windows as sys;
#[cfg(not(target_os = "windows"))]
pub use target::unix as sys;

pub use rust_i18n::*;
rust_i18n::i18n!("locales", backend = workloads::definitions::I18n::new());
