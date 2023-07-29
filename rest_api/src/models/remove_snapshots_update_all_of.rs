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
pub struct RemoveSnapshotsUpdateAllOf {
    #[serde(rename = "snapshot-ids")]
    pub snapshot_ids: Vec<i64>,
}

impl RemoveSnapshotsUpdateAllOf {
    pub fn new(snapshot_ids: Vec<i64>) -> RemoveSnapshotsUpdateAllOf {
        RemoveSnapshotsUpdateAllOf {
            snapshot_ids,
        }
    }
}


