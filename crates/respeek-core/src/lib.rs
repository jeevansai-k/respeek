/* respeek-core
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * All Respeek logic lives here. The UI layer only renders the serde view-models
 * produced by this crate and forwards user intents back to it.
 */

pub mod details;
pub mod devices;
pub mod engine;
pub mod history;
pub mod icons;
pub mod magpie;
pub mod model;
pub mod readings;
pub mod settings;
pub mod tables;
pub mod units;

pub use engine::{Engine, Page, ServiceAction, SetupScriptState, Signal, TableId};
pub use model::Snapshot;
pub use settings::Settings;
