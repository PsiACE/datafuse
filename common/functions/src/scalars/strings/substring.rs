// Copyright 2021 Datafuse Labs.
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

use std::fmt;

use common_datavalues2::prelude::*;
use common_exception::ErrorCode;
use common_exception::Result;

use crate::scalars::cast_column_field;
use crate::scalars::function_factory::FunctionFeatures;
use crate::scalars::Function2;
use crate::scalars::Function2Description;

#[derive(Clone)]
pub struct SubstringFunction {
    display_name: String,
}

impl SubstringFunction {
    pub fn try_create(display_name: &str) -> Result<Box<dyn Function2>> {
        Ok(Box::new(SubstringFunction {
            display_name: display_name.to_string(),
        }))
    }

    pub fn desc() -> Function2Description {
        Function2Description::creator(Box::new(Self::try_create)).features(
            FunctionFeatures::default()
                .deterministic()
                .variadic_arguments(2, 3),
        )
    }
}

impl Function2 for SubstringFunction {
    fn name(&self) -> &str {
        &*self.display_name
    }

    fn return_type(&self, args: &[&DataTypePtr]) -> Result<DataTypePtr> {
        if !args[0].data_type_id().is_string() && !args[0].data_type_id().is_null() {
            return Err(ErrorCode::IllegalDataType(format!(
                "Expected string or null, but got {}",
                args[0].data_type_id()
            )));
        }

        if !args[1].data_type_id().is_integer() && !args[1].data_type_id().is_null() {
            return Err(ErrorCode::IllegalDataType(format!(
                "Expected integer or string or null, but got {}",
                args[1].data_type_id()
            )));
        }

        if args.len() > 2
            && !args[2].data_type_id().is_integer()
            && !args[2].data_type_id().is_null()
        {
            return Err(ErrorCode::IllegalDataType(format!(
                "Expected integer or string or null, but got {}",
                args[2].data_type_id()
            )));
        }

        Ok(StringType::arc())
    }

    fn eval(&self, columns: &ColumnsWithField, input_rows: usize) -> Result<ColumnRef> {
        let s_column = cast_column_field(&columns[0], &StringType::arc())?;
        let s_viewer = ColumnViewer::<Vu8>::create(&s_column)?;

        let p_column = cast_column_field(&columns[1], &Int64Type::arc())?;
        let p_viewer = ColumnViewer::<i64>::create(&p_column)?;

        let mut builder = ColumnBuilder::<Vu8>::with_capacity(input_rows);

        if columns.len() > 2 {
            let p2_column = cast_column_field(&columns[2], &UInt64Type::arc())?;
            let p2_viewer = ColumnViewer::<u64>::create(&p2_column)?;

            for idx in 0..input_rows {
                let val = substr_from_for(
                    s_viewer.value(idx),
                    &p_viewer.value(idx),
                    &p2_viewer.value(idx),
                );
                builder.append(val);
            }
        } else {
            for idx in 0..input_rows {
                let val = substr_from(s_viewer.value(idx), &p_viewer.value(idx));
                builder.append(val);
            }
        }
        Ok(builder.build(input_rows))
    }
}

impl fmt::Display for SubstringFunction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.display_name)
    }
}

#[inline]
fn substr_from<'a>(str: &'a [u8], pos: &i64) -> &'a [u8] {
    substr(str, pos, &(str.len() as u64))
}

#[inline]
fn substr_from_for<'a>(str: &'a [u8], pos: &i64, len: &u64) -> &'a [u8] {
    substr(str, pos, len)
}

#[inline]
fn substr<'a>(str: &'a [u8], pos: &i64, len: &u64) -> &'a [u8] {
    if *pos > 0 && *pos <= str.len() as i64 {
        let l = str.len() as usize;
        let s = (*pos - 1) as usize;
        let mut e = *len as usize + s;
        if e > l {
            e = l;
        }
        return &str[s..e];
    }
    if *pos < 0 && -(*pos) <= str.len() as i64 {
        let l = str.len() as usize;
        let s = l - -*pos as usize;
        let mut e = *len as usize + s;
        if e > l {
            e = l;
        }
        return &str[s..e];
    }
    &str[0..0]
}
