//  Copyright 2021 Datafuse Labs.
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.

use std::sync::Arc;

use common_exception::Result;
use common_expression::types::DataType;
use common_expression::Chunk;
use common_expression::Column;
use common_expression::ColumnFrom;
use common_expression::TableDataType;
use common_expression::TableField;
use common_expression::TableSchema;
use common_expression::TableSchemaRefExt;
use common_expression::Value;
use common_storages_table_meta::meta::Statistics;
use common_storages_table_meta::meta::TableSnapshotStatistics;

use crate::sessions::TableContext;
use crate::FuseTable;

pub struct FuseStatistic<'a> {
    pub ctx: Arc<dyn TableContext>,
    pub table: &'a FuseTable,
}

impl<'a> FuseStatistic<'a> {
    pub fn new(ctx: Arc<dyn TableContext>, table: &'a FuseTable) -> Self {
        Self { ctx, table }
    }

    pub async fn get_statistic(self) -> Result<Chunk> {
        let snapshot_opt = self.table.read_table_snapshot().await?;
        if let Some(snapshot) = snapshot_opt {
            let table_statistics = self
                .table
                .read_table_snapshot_statistics(Some(&snapshot))
                .await?;
            return self.to_chunk(&snapshot.summary, &table_statistics);
        }
        Ok(Chunk::empty())
    }

    fn to_chunk(
        &self,
        _summy: &Statistics,
        table_statistics: &Option<Arc<TableSnapshotStatistics>>,
    ) -> Result<Chunk> {
        let mut col_ndvs: Vec<Vec<u8>> = Vec::with_capacity(1);
        if let Some(table_statistics) = table_statistics {
            let mut ndvs: String = "".to_string();
            for (i, n) in table_statistics.column_distinct_values.iter() {
                ndvs.push_str(&format!("({},{});", *i, *n));
            }
            col_ndvs.push(ndvs.into_bytes());
        };

        Ok(Chunk::new_from_sequence(
            vec![(Value::Column(Column::from_data(col_ndvs)), DataType::String)],
            1,
        ))
    }

    pub fn schema() -> Arc<TableSchema> {
        TableSchemaRefExt::create(vec![TableField::new(
            "column_distinct_values",
            TableDataType::String,
        )])
    }
}