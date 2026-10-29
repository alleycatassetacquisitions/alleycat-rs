mod read;
mod write;

pub use read::*;
pub use write::*;

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, PartialEq, Serialize, utoipa::ToSchema)]
pub(super) struct DeviceRecord {
    pub id: Uuid,
    pub id_label: i32,
    pub device_kind: String,
    pub hardware_version: Option<i32>,
    pub mac_address: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// PDN observations: true means working, false means failed, null means not checked.
/// Values describe this inspection only; nothing carries forward from older checks.
#[derive(Debug, PartialEq, Serialize, utoipa::ToSchema)]
pub(super) struct PdnComponentChecks {
    pub screen: Option<bool>,
    pub primary_button: Option<bool>,
    pub secondary_button: Option<bool>,
    pub haptics: Option<bool>,
    pub leds: Option<bool>,
    pub power_switch: Option<bool>,
    pub input_jack: Option<bool>,
    pub output_jack: Option<bool>,
    pub housing: Option<bool>,
}

/// Checks are selected by device kind; each kind has its own component schema.
#[derive(Debug, PartialEq, Serialize, utoipa::ToSchema)]
#[serde(tag = "device_kind", content = "components", rename_all = "snake_case")]
pub(super) enum InspectionChecks {
    Pdn(PdnComponentChecks),
}

#[derive(Debug, PartialEq, Serialize, utoipa::ToSchema)]
pub(super) struct Inspection {
    pub id: Uuid,
    pub device_id: Uuid,
    pub inspected_at: DateTime<Utc>,
    pub received_at: DateTime<Utc>,
    pub firmware_commit_hash: Option<String>,
    pub notes: Option<String>,
    pub checks: InspectionChecks,
}

pub(super) struct InspectionRow {
    pub id: Uuid,
    pub device_id: Uuid,
    pub inspected_at: DateTime<Utc>,
    pub received_at: DateTime<Utc>,
    pub firmware_commit_hash: Option<String>,
    pub notes: Option<String>,
    pub screen: Option<bool>,
    pub primary_button: Option<bool>,
    pub secondary_button: Option<bool>,
    pub haptics: Option<bool>,
    pub leds: Option<bool>,
    pub power_switch: Option<bool>,
    pub input_jack: Option<bool>,
    pub output_jack: Option<bool>,
    pub housing: Option<bool>,
}

impl From<InspectionRow> for Inspection {
    fn from(row: InspectionRow) -> Self {
        Self {
            id: row.id,
            device_id: row.device_id,
            inspected_at: row.inspected_at,
            received_at: row.received_at,
            firmware_commit_hash: row.firmware_commit_hash,
            notes: row.notes,
            checks: InspectionChecks::Pdn(PdnComponentChecks {
                screen: row.screen,
                primary_button: row.primary_button,
                secondary_button: row.secondary_button,
                haptics: row.haptics,
                leds: row.leds,
                power_switch: row.power_switch,
                input_jack: row.input_jack,
                output_jack: row.output_jack,
                housing: row.housing,
            }),
        }
    }
}
