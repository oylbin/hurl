/*
 * Hurl (https://hurl.dev)
 * Copyright (C) 2026 Orange
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *          http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
 */

//! Conversion between Hurl `Value` and JavaScript `JsValue`.

use boa_engine::{js_string, Context, JsObject, JsResult, JsValue};

use crate::runner::Number;
use crate::runner::Value;

use super::error::JsError;

/// Converts a Hurl `Value` to a JavaScript `JsValue`.
pub fn value_to_js(value: &Value, context: &mut Context) -> JsResult<JsValue> {
    match value {
        Value::Bool(b) => Ok(JsValue::from(*b)),
        Value::Null => Ok(JsValue::null()),
        Value::Number(n) => number_to_js(n),
        Value::String(s) => Ok(JsValue::from(js_string!(s.as_str()))),
        Value::List(items) => {
            let array = boa_engine::object::builtins::JsArray::new(context);
            for item in items {
                let js_item = value_to_js(item, context)?;
                array.push(js_item, context)?;
            }
            Ok(array.into())
        }
        Value::Object(pairs) => {
            let obj = JsObject::with_null_proto();
            for (key, val) in pairs {
                let js_val = value_to_js(val, context)?;
                obj.set(js_string!(key.as_str()), js_val, false, context)?;
            }
            Ok(obj.into())
        }
        Value::Bytes(bytes) => {
            // Convert bytes to Uint8Array
            let typed_array =
                boa_engine::object::builtins::JsUint8Array::from_iter(bytes.iter().copied(), context)?;
            Ok(typed_array.into())
        }
        Value::Date(dt) => {
            // Convert to ISO string for JavaScript
            let iso_string = dt.to_rfc3339();
            Ok(JsValue::from(js_string!(iso_string.as_str())))
        }
        Value::Unit => Ok(JsValue::undefined()),
        Value::Regex(r) => {
            // Pass regex as its pattern string
            Ok(JsValue::from(js_string!(r.as_str())))
        }
        Value::Nodeset(size) => {
            // Nodesets are represented as their size
            Ok(JsValue::from(*size as i32))
        }
        Value::HttpResponse(resp) => {
            // Convert to object with status and optional location
            let obj = JsObject::with_null_proto();
            obj.set(
                js_string!("status"),
                JsValue::from(resp.status() as i32),
                false,
                context,
            )?;
            if let Some(loc) = resp.location() {
                obj.set(
                    js_string!("location"),
                    JsValue::from(js_string!(loc.to_string().as_str())),
                    false,
                    context,
                )?;
            }
            Ok(obj.into())
        }
    }
}

/// Converts a Hurl `Number` to a JavaScript `JsValue`.
fn number_to_js(number: &Number) -> JsResult<JsValue> {
    match number {
        Number::Integer(i) => Ok(JsValue::from(*i as f64)),
        Number::Float(f) => Ok(JsValue::from(*f)),
        Number::BigInteger(s) => {
            // Try to parse as f64, fall back to string if too large
            if let Ok(f) = s.parse::<f64>() {
                Ok(JsValue::from(f))
            } else {
                Ok(JsValue::from(js_string!(s.as_str())))
            }
        }
    }
}

/// Converts a JavaScript `JsValue` to a Hurl `Value`.
pub fn js_to_value(js_value: &JsValue, context: &mut Context) -> Result<Value, JsError> {
    if js_value.is_undefined() || js_value.is_null() {
        return Ok(Value::Null);
    }

    if let Some(b) = js_value.as_boolean() {
        return Ok(Value::Bool(b));
    }

    if let Some(n) = js_value.as_number() {
        // Check if it's an integer
        if n.fract() == 0.0 && n >= i64::MIN as f64 && n <= i64::MAX as f64 {
            return Ok(Value::Number(Number::Integer(n as i64)));
        }
        return Ok(Value::Number(Number::Float(n)));
    }

    if let Some(s) = js_value.as_string() {
        return Ok(Value::String(s.to_std_string_escaped()));
    }

    if let Some(obj) = js_value.as_object() {
        // Check if it's an array
        if obj.is_array() {
            let length_value = obj
                .get(js_string!("length"), context)
                .map_err(|e| JsError::ConversionError {
                    message: e.to_string(),
                })?;
            let length: u32 = length_value
                .as_number()
                .map(|n| n as u32)
                .unwrap_or(0);

            let mut items = Vec::with_capacity(length as usize);
            for i in 0..length {
                let item = obj.get(i, context).map_err(|e| JsError::ConversionError {
                    message: e.to_string(),
                })?;
                items.push(js_to_value(&item, context)?);
            }
            return Ok(Value::List(items));
        }

        // Check if it's a Uint8Array (bytes)
        if let Ok(typed_array) = boa_engine::object::builtins::JsUint8Array::from_object(obj.clone()) {
            let length = typed_array.length(context).map_err(|e| JsError::ConversionError {
                message: e.to_string(),
            })?;
            let mut bytes = Vec::with_capacity(length as usize);
            for i in 0..length {
                let byte = typed_array.get(i, context).map_err(|e| JsError::ConversionError {
                    message: e.to_string(),
                })?;
                if let Some(n) = byte.as_number() {
                    bytes.push(n as u8);
                }
            }
            return Ok(Value::Bytes(bytes));
        }

        // Regular object
        let keys = obj
            .own_property_keys(context)
            .map_err(|e| JsError::ConversionError {
                message: e.to_string(),
            })?;
        let mut pairs = Vec::with_capacity(keys.len());
        for key in keys {
            let key_str = key.to_string();
            let val = obj
                .get(key.clone(), context)
                .map_err(|e| JsError::ConversionError {
                    message: e.to_string(),
                })?;
            pairs.push((key_str, js_to_value(&val, context)?));
        }
        return Ok(Value::Object(pairs));
    }

    // Fallback: convert to string
    let s = js_value.to_string(context).map_err(|e| JsError::ConversionError {
        message: e.to_string(),
    })?;
    Ok(Value::String(s.to_std_string_escaped()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_to_js_primitives() {
        let mut context = Context::default();

        // Bool
        let js = value_to_js(&Value::Bool(true), &mut context).unwrap();
        assert_eq!(js.as_boolean(), Some(true));

        // Null
        let js = value_to_js(&Value::Null, &mut context).unwrap();
        assert!(js.is_null());

        // Integer
        let js = value_to_js(&Value::Number(Number::Integer(42)), &mut context).unwrap();
        assert_eq!(js.as_number(), Some(42.0));

        // Float
        let js = value_to_js(&Value::Number(Number::Float(3.14)), &mut context).unwrap();
        assert!((js.as_number().unwrap() - 3.14).abs() < f64::EPSILON);

        // String
        let js = value_to_js(&Value::String("hello".to_string()), &mut context).unwrap();
        assert_eq!(
            js.as_string().map(|s| s.to_std_string_escaped()),
            Some("hello".to_string())
        );
    }

    #[test]
    fn test_js_to_value_primitives() {
        let mut context = Context::default();

        // Bool
        let val = js_to_value(&JsValue::from(true), &mut context).unwrap();
        assert_eq!(val, Value::Bool(true));

        // Null
        let val = js_to_value(&JsValue::null(), &mut context).unwrap();
        assert_eq!(val, Value::Null);

        // Number (integer)
        let val = js_to_value(&JsValue::from(42), &mut context).unwrap();
        assert_eq!(val, Value::Number(Number::Integer(42)));

        // Number (float)
        let val = js_to_value(&JsValue::from(3.14), &mut context).unwrap();
        assert_eq!(val, Value::Number(Number::Float(3.14)));

        // String
        let val = js_to_value(&JsValue::from(js_string!("hello")), &mut context).unwrap();
        assert_eq!(val, Value::String("hello".to_string()));
    }
}
