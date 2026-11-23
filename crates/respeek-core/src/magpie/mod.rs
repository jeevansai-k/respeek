/* magpie/mod.rs
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

#[macro_use]
pub(crate) mod shim;
mod client;

pub use client::{
    App, Battery, Client, Connection, Cpu, Disk, DiskKind, ErrorEjectFailed, Fan, Gpu, Memory,
    MemoryDevice, Process, ScriptName, ScriptRunResult, ScriptStatus, Service, SmartData,
};
pub use magpie_types as types;
pub use shim::{set_fatal_handler, ENV_MAGPIE_PATH};
