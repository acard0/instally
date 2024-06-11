#![allow(dead_code, unused_variables)]

pub use rust_i18n::*;

pub mod http;
pub mod definitions;
pub mod workloads;
pub mod archiving;
pub mod helpers;
pub mod extensions;
pub mod factory;
pub mod scripting;

pub(crate) mod target;

#[cfg(target_os = "windows")]
pub use target::windows as sys;
#[cfg(not(target_os = "windows"))]
pub use target::unix as sys;

rust_i18n::i18n!("locales", backend = definitions::i18n::I18n::new());