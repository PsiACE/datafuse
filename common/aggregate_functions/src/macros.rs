// Copyright 2020-2021 The Datafuse Authors.
//
// SPDX-License-Identifier: Apache-2.0.

macro_rules! typed_array_op_to_data_value {
    ($VALUES:expr, $ARRAYTYPE:ident, $SCALAR:ident, $OP:ident) => {{
        let array = downcast_array!($VALUES, $ARRAYTYPE)?;
        let value = common_arrow::arrow::compute::$OP(array);
        Result::Ok(DataValue::$SCALAR(value))
    }};
}

macro_rules! dispatch_primitive_array {
    // $DISPATCH_M: the inner macro to use
    // $ARRAY: the array to dispatch
    ($DISPATCH_M:ident, $ARRAY:expr, $OP:ident) => {{
        match $ARRAY.data_type() {
            DataType::Int8 => $DISPATCH_M! {$ARRAY,Int8Array,Int8,$OP},
            DataType::Int16 => $DISPATCH_M! {$ARRAY,Int16Array,Int16,$OP},
            DataType::Int32 => $DISPATCH_M! {$ARRAY,Int32Array,Int32,$OP},
            DataType::Int64 => $DISPATCH_M! {$ARRAY,Int64Array,Int64,$OP},
            DataType::UInt8 => $DISPATCH_M! {$ARRAY,UInt8Array,UInt8,$OP},
            DataType::UInt16 => $DISPATCH_M! {$ARRAY,UInt16Array,UInt16,$OP},
            DataType::UInt32 => $DISPATCH_M! {$ARRAY,UInt32Array,UInt32,$OP},
            DataType::UInt64 => $DISPATCH_M! {$ARRAY,UInt64Array,UInt64,$OP},
            DataType::Float32 => $DISPATCH_M! {$ARRAY,Float32Array,Float32,$OP},
            DataType::Float64 => $DISPATCH_M! {$ARRAY,Float64Array,Float64,$OP},

            other => Err(ErrorCodes::BadDataValueType(format!(
                "DataValue Error: Unsupported aggregate operation: {} for data type: {}",
                stringify!($OP),
                other,
            ))),
        }
    }};
}

macro_rules! typed_utf8_array_op_to_data_value {
    ($VALUES:expr, $ARRAYTYPE:ident, $SCALAR:ident, $OP:ident) => {{
        let array = downcast_array!($VALUES, $ARRAYTYPE)?;
        let value = common_arrow::arrow::compute::$OP(array);
        let value = value.and_then(|e| Some(e.to_string()));
        Ok(DataValue::$SCALAR(value))
    }};
}
macro_rules! dispatch_utf8_array {
    // $DISPATCH_M: the inner macro to use
    // $ARRAY: the array to dispatch
    ($DISPATCH_M:ident, $ARRAY:expr, $OP:ident) => {{
        match $ARRAY.data_type() {
            DataType::Utf8 => $DISPATCH_M! {$ARRAY,StringArray,Utf8,$OP},

            other => Err(ErrorCodes::BadDataValueType(format!(
                "DataValue Error: Unsupported aggregate operation: {} for data type: {}",
                stringify!($OP),
                other,
            ))),
        }
    }};
}
