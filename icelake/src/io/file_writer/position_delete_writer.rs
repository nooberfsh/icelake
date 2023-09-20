//! A module provide `PositionDeleteWriter`.
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::config::TableConfigRef;
use crate::io::location_generator::FileLocationGenerator;
use crate::types::{Any, DataFileBuilder, Field, Primitive, Schema};
use crate::{types::Struct, Result};
use crate::{Error, ErrorKind};
use arrow_array::{ArrayRef, Int64Array, RecordBatch, StringArray};
use arrow_schema::SchemaRef;
use opendal::Operator;

use super::rolling_writer::RollingWriter;

/// A PositionDeleteWriter used to write position delete, it will sort the incoming delete by file_path and pos.
pub struct SortedPositionDeleteWriter {
    operator: Operator,
    table_location: String,
    location_generator: Arc<FileLocationGenerator>,
    table_config: TableConfigRef,

    delete_cache: BTreeMap<String, Vec<i64>>,
    record_num: usize,

    result: Vec<DataFileBuilder>,
}

impl SortedPositionDeleteWriter {
    /// Create a new `SortedPositionDeleteWriter`.
    pub fn new(
        operator: Operator,
        table_location: String,
        location_generator: Arc<FileLocationGenerator>,
        table_config: TableConfigRef,
    ) -> Self {
        Self {
            operator,
            table_location,
            location_generator,
            table_config,
            delete_cache: BTreeMap::new(),
            record_num: 0,
            result: vec![],
        }
    }

    /// Delete a row.
    ///
    /// #TODO
    /// - support delete with row
    pub async fn delete(&mut self, file_path: String, pos: i64) -> Result<()> {
        self.record_num += 1;
        let delete_list = self.delete_cache.entry(file_path).or_insert(vec![]);
        delete_list.push(pos);

        if self.record_num
            >= self
                .table_config
                .sorted_delete_position_writer
                .max_record_num
        {
            self.flush().await?;
        }

        Ok(())
    }

    /// Write the delete cache into delete file.
    async fn flush(&mut self) -> Result<()> {
        let mut writer = PositionDeleteWriter::try_new(
            self.operator.clone(),
            self.table_location.clone(),
            self.location_generator.clone(),
            None,
            self.table_config.clone(),
        )
        .await?;
        let delete_cache = std::mem::take(&mut self.delete_cache);
        for (file_path, mut delete_vec) in delete_cache.into_iter() {
            delete_vec.sort();
            writer.write_by_vec(file_path, delete_vec, None).await?;
        }
        self.result.extend(writer.close().await?);
        self.record_num = 0;
        Ok(())
    }

    /// Complte the write and return the list of `DataFileBuilder` as result.
    pub async fn close(mut self) -> Result<Vec<DataFileBuilder>> {
        if self.record_num > 0 {
            self.flush().await?;
        }
        Ok(self.result)
    }
}

/// A writer capable of splitting incoming delete into multiple files within one spec/partition based on the target file size.
/// When complete, it will return a list of `DataFile`.
///
///
/// # NOTE
/// According to spec, The data write to position delete file should be:
/// - Sorting by file_path allows filter pushdown by file in columnar storage formats.
/// - Sorting by pos allows filtering rows while scanning, to avoid keeping deletes in memory.
/// - They're belong to partition.
///
/// But PositionDeleteWriter will not gurantee and check above. It is the caller's responsibility to gurantee them.
pub struct PositionDeleteWriter {
    inner_writer: RollingWriter,
    table_location: String,
    schema: SchemaRef,
}

impl PositionDeleteWriter {
    /// Create a new `PositionDeleteWriter`.
    pub async fn try_new(
        operator: Operator,
        table_location: String,
        location_generator: Arc<FileLocationGenerator>,
        row_type: Option<Arc<Struct>>,
        table_config: TableConfigRef,
    ) -> Result<Self> {
        let mut fields = vec![
            Arc::new(Field::required(
                2147483546,
                "file_path",
                Any::Primitive(Primitive::String),
            )),
            Arc::new(Field::required(
                2147483545,
                "pos",
                Any::Primitive(Primitive::Long),
            )),
        ];
        if let Some(row_type) = row_type {
            fields.push(Arc::new(Field::required(
                2147483544,
                "row",
                Any::Struct(row_type),
            )));
        }
        let schema: SchemaRef = Arc::new(Schema::new(1, None, Struct::new(fields)).try_into()?);
        Ok(Self {
            inner_writer: RollingWriter::try_new(
                operator,
                location_generator,
                schema.clone(),
                table_config,
            )
            .await?,
            table_location,
            schema,
        })
    }

    /// Write delete pos in a file by pos vec. Pos vec should be sorted.
    pub async fn write_by_vec(
        &mut self,
        file_path: String,
        pos_vec: Vec<i64>,
        row_array: Option<ArrayRef>,
    ) -> Result<()> {
        let file_path_array = Arc::new(StringArray::from(vec![file_path; pos_vec.len()]));
        let pos_array = Arc::new(Int64Array::from(pos_vec));
        let batch = if let Some(row_array) = row_array {
            RecordBatch::try_new(
                self.schema.clone(),
                vec![file_path_array, pos_array, row_array],
            )
            .map_err(|err| Error::new(ErrorKind::ArrowError, format!("{err}")))?
        } else {
            RecordBatch::try_new(self.schema.clone(), vec![file_path_array, pos_array])
                .map_err(|err| Error::new(ErrorKind::ArrowError, format!("{err}")))?
        };
        self.write(batch).await?;
        Ok(())
    }

    /// Write a record batch.
    pub async fn write(&mut self, batch: RecordBatch) -> Result<()> {
        self.inner_writer.write(batch).await?;
        Ok(())
    }

    /// Complte the write and return the list of `DataFileBuilder` as result.
    pub async fn close(self) -> Result<Vec<DataFileBuilder>> {
        Ok(self
            .inner_writer
            .close()
            .await?
            .into_iter()
            .map(|builder| {
                builder
                    .with_content(crate::types::DataContentType::PostionDeletes)
                    .with_table_location(self.table_location.clone())
            })
            .collect())
    }
}
