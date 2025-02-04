use std::{collections::HashMap, sync::Arc};

use super::{
    create_transform_function, Any, AnyValue, BoxedTransformFunction, PartitionSpec, StructValue,
};
use crate::{types::struct_to_anyvalue_array_with_type, Error, ErrorKind, Result};
use arrow_array::{Array, ArrayRef, BooleanArray, RecordBatch, StructArray};
use arrow_cast::cast;
use arrow_row::{OwnedRow, RowConverter, SortField};
use arrow_schema::{DataType, FieldRef, Fields, SchemaRef};
use arrow_select::filter::filter_record_batch;

/// `PartitionSplitter` is used to splite a given record according partition value.
pub struct PartitionSplitter {
    field_infos: Vec<PartitionFieldComputeInfo>,
    partition_type: Any,
    row_converter: RowConverter,
}

/// Internal info used to compute single partition field .
struct PartitionFieldComputeInfo {
    pub index_vec: Vec<usize>,
    pub field: FieldRef,
    pub transform: BoxedTransformFunction,
}

#[derive(Hash, PartialEq, PartialOrd, Eq, Ord, Clone)]
/// `PartitionKey` is the wrapper of OwnedRow to avoid user depend OwnedRow directly.
pub struct PartitionKey {
    inner: OwnedRow,
}

impl From<OwnedRow> for PartitionKey {
    fn from(value: OwnedRow) -> Self {
        Self { inner: value }
    }
}

impl PartitionSplitter {
    /// Create a new `PartitionSplitter`.
    pub fn try_new(
        partition_spec: &PartitionSpec,
        schema: &SchemaRef,
        partition_type: Any,
    ) -> Result<Self> {
        let arrow_partition_type: DataType = partition_type.clone().try_into()?;
        let row_converter = RowConverter::new(vec![SortField::new(arrow_partition_type.clone())])
            .map_err(|e| {
            crate::error::Error::new(crate::ErrorKind::ArrowError, format!("{}", e))
        })?;

        let field_infos = if let DataType::Struct(struct_type) = arrow_partition_type {
            if struct_type.len() != partition_spec.fields.len() {
                return Err(Error::new(
                    ErrorKind::IcebergDataInvalid,
                    format!(
                        "Partition spec fields length {} not match partition type fields length {}",
                        partition_spec.fields.len(),
                        struct_type.len()
                    ),
                ));
            }
            struct_type
                .iter()
                .zip(partition_spec.fields.iter())
                .map(|(arrow_field, spec_field)| {
                    let transform = create_transform_function(&spec_field.transform)?;
                    let mut index_vec = vec![];
                    Self::fetch_column_index(
                        schema.fields(),
                        &mut index_vec,
                        spec_field.source_column_id as i64,
                    );
                    if index_vec.is_empty() {
                        return Err(Error::new(
                            ErrorKind::IcebergDataInvalid,
                            format!(
                                "Can't find source column id: {}",
                                spec_field.source_column_id
                            ),
                        ));
                    }
                    Ok(PartitionFieldComputeInfo {
                        index_vec,
                        field: arrow_field.clone(),
                        transform,
                    })
                })
                .collect::<Result<Vec<_>>>()?
        } else {
            unreachable!("Partition type should be struct type")
        };
        Ok(Self {
            field_infos,
            partition_type,
            row_converter,
        })
    }

    /// Fetch the column index vector of the column id (We store it in Field of arrow as dict id).
    /// e.g.
    /// struct<struct<x:1,y:2>,z:3>
    /// for source column id 2,
    /// you will get the source column index vector [1,0]
    fn fetch_column_index(fields: &Fields, index_vec: &mut Vec<usize>, col_id: i64) {
        for (pos, field) in fields.iter().enumerate() {
            let id: i64 = field
                .metadata()
                .get("column_id")
                .expect("column_id must be set")
                .parse()
                .expect("column_id must can be parse as i64");
            if col_id == id {
                index_vec.push(pos);
                return;
            }
            if let DataType::Struct(inner) = field.data_type() {
                Self::fetch_column_index(inner, index_vec, col_id);
                if !index_vec.is_empty() {
                    index_vec.push(pos);
                    return;
                }
            }
        }
    }

    /// This function do two things:
    /// 1. Partition the batch by partition spec.
    /// 2. Create the partition value.
    pub fn split_by_partition(
        &mut self,
        batch: &RecordBatch,
    ) -> Result<HashMap<PartitionKey, RecordBatch>> {
        let value_array = Arc::new(StructArray::from(
            self.field_infos
                .iter()
                .map(
                    |PartitionFieldComputeInfo {
                         index_vec,
                         field,
                         transform,
                     }| {
                        let array = Self::get_column_by_index_vec(batch, index_vec);
                        let mut array = transform.transform(array)?;
                        // Try avoid different timestamp time zone.
                        if array.data_type() != field.data_type() {
                            if let DataType::Timestamp(unit, _) = array.data_type() {
                                if let DataType::Timestamp(field_unit, _) = field.data_type() {
                                    if unit == field_unit {
                                        array =
                                            cast(&transform.transform(array)?, field.data_type())
                                                .map_err(|e| {
                                                    crate::error::Error::new(
                                                        crate::ErrorKind::ArrowError,
                                                        format!("{e}"),
                                                    )
                                                })?
                                    }
                                }
                            }
                        }
                        Ok((field.clone(), array))
                    },
                )
                .collect::<Result<Vec<_>>>()?,
        ));

        let rows = self
            .row_converter
            .convert_columns(&[value_array])
            .map_err(|e| {
                crate::error::Error::new(crate::ErrorKind::ArrowError, format!("{}", e))
            })?;

        // Group the batch by row value.
        let mut group_ids = HashMap::new();
        rows.into_iter().enumerate().for_each(|(row_id, row)| {
            group_ids.entry(row.owned()).or_insert(vec![]).push(row_id);
        });

        // Partition the batch with same partition partition_values
        let mut partition_batches = HashMap::new();
        for (row, row_ids) in group_ids.into_iter() {
            // generate the bool filter array from column_ids
            let filter_array: BooleanArray = {
                let mut filter = vec![false; batch.num_rows()];
                row_ids.into_iter().for_each(|row_id| {
                    filter[row_id] = true;
                });
                filter.into()
            };

            // filter the RecordBatch
            partition_batches.insert(
                row.clone().into(),
                filter_record_batch(batch, &filter_array)
                    .expect("We should guarantee the filter array is valid"),
            );
        }

        Ok(partition_batches)
    }

    fn get_column_by_index_vec(batch: &RecordBatch, index_vec: &[usize]) -> ArrayRef {
        let mut rev_iterator = index_vec.iter().rev();
        let mut array = batch.column(*rev_iterator.next().unwrap()).clone();
        for idx in rev_iterator {
            array = array
                .as_any()
                .downcast_ref::<StructArray>()
                .unwrap()
                .column(*idx)
                .clone();
        }
        array
    }

    /// Convert the `PartitionKey` to `PartitionValue`
    ///
    /// The reason we seperate them is to save memmory cost, when in write process, we only need to
    /// keep the `PartitionKey`. It's effiect to used in Hash. When write complete, we can use it to convert `PartitionKey` to
    /// `PartitionValue` to store it in `DataFile`.
    pub fn convert_key_to_value(&self, key: PartitionKey) -> Result<StructValue> {
        let array = {
            let mut arrays = self
                .row_converter
                .convert_rows([key.inner.row()].into_iter())
                .map_err(|e| {
                    crate::error::Error::new(crate::ErrorKind::ArrowError, format!("{e}"))
                })?;
            assert!(arrays.len() == 1);
            arrays.pop().unwrap()
        };
        let struct_array = array.as_any().downcast_ref::<StructArray>().unwrap();

        let mut value_array =
            struct_to_anyvalue_array_with_type(struct_array, self.partition_type.clone())?;

        assert!(value_array.len() == 1);
        let value = value_array.pop().unwrap().unwrap();
        if let AnyValue::Struct(value) = value {
            Ok(value)
        } else {
            unreachable!()
        }
    }
}
