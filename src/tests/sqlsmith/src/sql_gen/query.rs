// Copyright 2021 Datafuse Labs
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use common_ast::ast::Identifier;
use common_ast::ast::Query;
use common_ast::ast::SelectStmt;
use common_ast::ast::SelectTarget;
use common_ast::ast::SetExpr;
use common_ast::ast::TableReference;
use rand::Rng;

use crate::sql_gen::SqlGenerator;

impl<'a, R: Rng> SqlGenerator<'a, R> {
    pub(crate) fn gen_query(&mut self) -> Query {
        let body = self.gen_set_expr();
        Query {
            span: None,
            // TODO
            with: None,
            body,
            // TODO
            order_by: vec![],
            // TODO
            limit: vec![],
            // TODO
            offset: None,
            ignore_result: false,
        }
    }

    fn gen_set_expr(&mut self) -> SetExpr {
        match self.rng.gen_range(0..=9) {
            0..=9 => {
                let select = self.gen_select();
                SetExpr::Select(Box::new(select))
            }
            // TODO
            _ => unreachable!(),
        }
    }

    fn gen_select(&mut self) -> SelectStmt {
        let select_list = self.gen_select_list();
        let from = self.gen_from();
        SelectStmt {
            span: None,
            // TODO
            hints: None,
            // TODO
            distinct: false,
            select_list,
            from,
            // TODO
            selection: None,
            // TODO
            group_by: None,
            // TODO
            having: None,
            // TODO
            window_list: None,
        }
    }

    fn gen_select_list(&mut self) -> Vec<SelectTarget> {
        let select_num = self.rng.gen_range(1..=5);
        let mut targets = Vec::with_capacity(select_num);
        for _ in 0..select_num {
            let target = match self.rng.gen_range(0..=9) {
                0..=9 => {
                    let expr = self.gen_expr();
                    SelectTarget::AliasedExpr {
                        expr: Box::new(expr),
                        // TODO
                        alias: None,
                    }
                }
                // TODO
                _ => unreachable!(),
            };
            targets.push(target)
        }
        targets
    }

    fn gen_from(&mut self) -> Vec<TableReference> {
        match self.rng.gen_range(0..=9) {
            0..=9 => {
                let table = &self.tables[0];
                let table_name = Identifier::from_name(table.name.clone());

                let table_ref = TableReference::Table {
                    span: None,
                    // TODO
                    catalog: None,
                    // TODO
                    database: None,
                    table: table_name,
                    // TODO
                    alias: None,
                    // TODO
                    travel_point: None,
                    // TODO
                    pivot: None,
                    // TODO
                    unpivot: None,
                };
                vec![table_ref]
            }
            // TODO
            _ => unreachable!(),
        }
    }
}
