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
pub struct MetricResult {
    #[serde(rename = "unit")]
    pub unit: String,
    #[serde(rename = "value")]
    pub value: i64,
    #[serde(rename = "time-unit")]
    pub time_unit: String,
    #[serde(rename = "count")]
    pub count: i64,
    #[serde(rename = "total-duration")]
    pub total_duration: i64,
}

impl MetricResult {
    pub fn new(unit: String, value: i64, time_unit: String, count: i64, total_duration: i64) -> MetricResult {
        MetricResult {
            unit,
            value,
            time_unit,
            count,
            total_duration,
        }
    }
}


