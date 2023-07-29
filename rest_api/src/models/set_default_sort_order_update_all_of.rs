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
pub struct SetDefaultSortOrderUpdateAllOf {
    /// Sort order ID to set as the default, or -1 to set last added sort order
    #[serde(rename = "sort-order-id")]
    pub sort_order_id: i32,
}

impl SetDefaultSortOrderUpdateAllOf {
    pub fn new(sort_order_id: i32) -> SetDefaultSortOrderUpdateAllOf {
        SetDefaultSortOrderUpdateAllOf {
            sort_order_id,
        }
    }
}


