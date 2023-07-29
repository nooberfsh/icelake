/*
 * Apache Iceberg REST Catalog API
 *
 * Defines the specification for the first version of the REST Catalog API. Implementations should ideally support both Iceberg table specs v1 and v2, with priority given to v2.
 *
 * The version of the OpenAPI document: 0.0.1
 * 
 * Generated by: https://openapi-generator.tech
 */




#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct SetPropertiesUpdate {
    #[serde(rename = "action")]
    pub action: Action,
    #[serde(rename = "updates")]
    pub updates: ::std::collections::HashMap<String, String>,
}

impl SetPropertiesUpdate {
    pub fn new(action: Action, updates: ::std::collections::HashMap<String, String>) -> SetPropertiesUpdate {
        SetPropertiesUpdate {
            action,
            updates,
        }
    }
}

/// 
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum Action {
    #[serde(rename = "upgrade-format-version")]
    UpgradeFormatVersion,
    #[serde(rename = "add-schema")]
    AddSchema,
    #[serde(rename = "set-current-schema")]
    SetCurrentSchema,
    #[serde(rename = "add-spec")]
    AddSpec,
    #[serde(rename = "set-default-spec")]
    SetDefaultSpec,
    #[serde(rename = "add-sort-order")]
    AddSortOrder,
    #[serde(rename = "set-default-sort-order")]
    SetDefaultSortOrder,
    #[serde(rename = "add-snapshot")]
    AddSnapshot,
    #[serde(rename = "set-snapshot-ref")]
    SetSnapshotRef,
    #[serde(rename = "remove-snapshots")]
    RemoveSnapshots,
    #[serde(rename = "remove-snapshot-ref")]
    RemoveSnapshotRef,
    #[serde(rename = "set-location")]
    SetLocation,
    #[serde(rename = "set-properties")]
    SetProperties,
    #[serde(rename = "remove-properties")]
    RemoveProperties,
}

impl Default for Action {
    fn default() -> Action {
        Self::UpgradeFormatVersion
    }
}

