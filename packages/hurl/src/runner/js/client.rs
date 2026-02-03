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

//! JavaScript `client` global object.
//!
//! The `client` object stores session metadata that persists across requests.
//! Similar to JetBrains HTTP Client's client object.

use std::collections::HashMap;

use boa_engine::{js_string, Context, JsObject, JsResult, JsValue};

use crate::runner::Value;

use super::convert::value_to_js;

/// Client object that stores session metadata.
///
/// This object persists throughout the Hurl session and can be used
/// to store custom data that needs to be shared across requests.
#[derive(Clone, Debug, Default)]
pub struct JsClient {
    /// Global storage for custom variables.
    pub global: HashMap<String, Value>,
}

impl JsClient {
    /// Creates a new empty client.
    pub fn new() -> Self {
        Self {
            global: HashMap::new(),
        }
    }

    /// Converts the client to a JavaScript object and registers it in the context.
    pub fn to_js_object(&self, context: &mut Context) -> JsResult<JsObject> {
        let obj = JsObject::with_null_proto();

        // Create the global storage object
        let global_obj = JsObject::with_null_proto();
        for (key, value) in &self.global {
            let js_value = value_to_js(value, context)?;
            global_obj.set(js_string!(key.as_str()), js_value, false, context)?;
        }
        obj.set(js_string!("global"), JsValue::from(global_obj), false, context)?;

        Ok(obj)
    }

    /// Updates the client from a JavaScript object.
    ///
    /// This is called after JavaScript execution to persist any changes
    /// made to the client object.
    pub fn update_from_js(&mut self, js_obj: &JsObject, context: &mut Context) -> JsResult<()> {
        // Get the global object
        let global_value = js_obj.get(js_string!("global"), context)?;
        if let Some(global_obj) = global_value.as_object() {
            let keys = global_obj.own_property_keys(context)?;
            self.global.clear();
            for key in keys {
                let key_str = key.to_string();
                let value = global_obj.get(key.clone(), context)?;
                if let Ok(hurl_value) = super::convert::js_to_value(&value, context) {
                    self.global.insert(key_str, hurl_value);
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::Number;

    #[test]
    fn test_client_to_js_object() {
        let mut context = Context::default();
        let mut client = JsClient::new();
        client
            .global
            .insert("counter".to_string(), Value::Number(Number::Integer(42)));
        client
            .global
            .insert("name".to_string(), Value::String("test".to_string()));

        let js_obj = client.to_js_object(&mut context).unwrap();

        let global = js_obj.get(js_string!("global"), &mut context).unwrap();
        assert!(global.is_object());

        let global_obj = global.as_object().unwrap();
        let counter = global_obj.get(js_string!("counter"), &mut context).unwrap();
        assert_eq!(counter.as_number(), Some(42.0));
    }
}
