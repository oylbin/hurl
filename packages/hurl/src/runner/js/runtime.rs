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

//! JavaScript runtime for executing custom filters.

use std::path::Path;

use boa_engine::{js_string, Context, JsValue, Source};

use crate::runner::Value;

use super::client::JsClient;
use super::convert::{js_to_value, value_to_js};
use super::error::JsError;
use super::response::JsResponse;

/// JavaScript runtime for executing custom filter functions.
///
/// The runtime maintains a boa_engine Context and provides methods to:
/// - Load JavaScript files containing filter functions
/// - Call filter functions with Hurl values
/// - Manage the `client` and `response` global objects
pub struct JsRuntime {
    context: Context,
    client: JsClient,
    response: JsResponse,
}

impl JsRuntime {
    /// Creates a new JavaScript runtime.
    pub fn new() -> Self {
        let context = Context::default();
        Self {
            context,
            client: JsClient::new(),
            response: JsResponse::empty(),
        }
    }

    /// Loads and executes a JavaScript file.
    ///
    /// The file should contain filter function definitions in the form:
    /// ```javascript
    /// function filter_name(input, arg1, arg2, ...) {
    ///     return transformedValue;
    /// }
    /// ```
    pub fn load_file(&mut self, path: &Path) -> Result<(), JsError> {
        let source = std::fs::read_to_string(path).map_err(|e| JsError::FileLoadError {
            path: path.display().to_string(),
            message: e.to_string(),
        })?;

        self.context
            .eval(Source::from_bytes(&source))
            .map_err(|e| JsError::ParseError {
                message: e.to_string(),
            })?;

        Ok(())
    }

    /// Updates the `response` global object with new response data.
    pub fn set_response(&mut self, response: JsResponse) {
        self.response = response;
    }

    /// Returns a reference to the client object.
    pub fn client(&self) -> &JsClient {
        &self.client
    }

    /// Returns a mutable reference to the client object.
    pub fn client_mut(&mut self) -> &mut JsClient {
        &mut self.client
    }

    /// Calls a filter function with the given input and arguments.
    ///
    /// The function is looked up by name with the `filter_` prefix.
    /// For example, calling `call_filter("add", ...)` will look for a function
    /// named `filter_add`.
    pub fn call_filter(
        &mut self,
        name: &str,
        input: &Value,
        args: &[Value],
    ) -> Result<Value, JsError> {
        let function_name = format!("filter_{name}");

        // Set up global objects
        self.setup_globals()?;

        // Look up the function
        let global = self.context.global_object();
        let func_value = global
            .get(js_string!(function_name.as_str()), &mut self.context)
            .map_err(|e| JsError::RuntimeError {
                message: e.to_string(),
            })?;

        if !func_value.is_callable() {
            return Err(JsError::FunctionNotFound {
                name: name.to_string(),
            });
        }

        let func = func_value
            .as_callable()
            .ok_or_else(|| JsError::FunctionNotFound {
                name: name.to_string(),
            })?;

        // Convert input and arguments to JavaScript values
        let js_input =
            value_to_js(input, &mut self.context).map_err(|e| JsError::ConversionError {
                message: e.to_string(),
            })?;

        let mut js_args = vec![js_input];
        for arg in args {
            let js_arg =
                value_to_js(arg, &mut self.context).map_err(|e| JsError::ConversionError {
                    message: e.to_string(),
                })?;
            js_args.push(js_arg);
        }

        // Call the function
        let result = func
            .call(&JsValue::undefined(), &js_args, &mut self.context)
            .map_err(|e| JsError::RuntimeError {
                message: e.to_string(),
            })?;

        // Update client from any changes made in JavaScript
        self.update_client_from_js()?;

        // Convert result back to Hurl value
        js_to_value(&result, &mut self.context)
    }

    /// Sets up the `client` and `response` global objects.
    fn setup_globals(&mut self) -> Result<(), JsError> {
        let global = self.context.global_object();

        // Set up client global
        let client_obj = self
            .client
            .to_js_object(&mut self.context)
            .map_err(|e| JsError::RuntimeError {
                message: e.to_string(),
            })?;
        global
            .set(
                js_string!("client"),
                JsValue::from(client_obj),
                false,
                &mut self.context,
            )
            .map_err(|e| JsError::RuntimeError {
                message: e.to_string(),
            })?;

        // Set up response global
        let response_obj = self
            .response
            .to_js_object(&mut self.context)
            .map_err(|e| JsError::RuntimeError {
                message: e.to_string(),
            })?;
        global
            .set(
                js_string!("response"),
                JsValue::from(response_obj),
                false,
                &mut self.context,
            )
            .map_err(|e| JsError::RuntimeError {
                message: e.to_string(),
            })?;

        Ok(())
    }

    /// Updates the internal client state from the JavaScript global.
    fn update_client_from_js(&mut self) -> Result<(), JsError> {
        let global = self.context.global_object();
        let client_value = global
            .get(js_string!("client"), &mut self.context)
            .map_err(|e| JsError::RuntimeError {
                message: e.to_string(),
            })?;

        if let Some(client_obj) = client_value.as_object() {
            self.client
                .update_from_js(client_obj, &mut self.context)
                .map_err(|e| JsError::RuntimeError {
                    message: e.to_string(),
                })?;
        }

        Ok(())
    }
}

impl Default for JsRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::Number;

    #[test]
    fn test_call_simple_filter() {
        let mut runtime = JsRuntime::new();

        // Load a simple add filter
        let source = r#"
            function filter_add(input, n) {
                return input + n;
            }
        "#;
        runtime
            .context
            .eval(Source::from_bytes(source))
            .expect("Failed to load source");

        let result = runtime
            .call_filter(
                "add",
                &Value::Number(Number::Integer(10)),
                &[Value::Number(Number::Integer(5))],
            )
            .unwrap();

        assert_eq!(result, Value::Number(Number::Integer(15)));
    }

    #[test]
    fn test_call_string_filter() {
        let mut runtime = JsRuntime::new();

        let source = r#"
            function filter_upper(input) {
                return input.toUpperCase();
            }
        "#;
        runtime
            .context
            .eval(Source::from_bytes(source))
            .expect("Failed to load source");

        let result = runtime
            .call_filter("upper", &Value::String("hello".to_string()), &[])
            .unwrap();

        assert_eq!(result, Value::String("HELLO".to_string()));
    }

    #[test]
    fn test_filter_not_found() {
        let mut runtime = JsRuntime::new();

        let result = runtime.call_filter(
            "nonexistent",
            &Value::Number(Number::Integer(10)),
            &[],
        );

        assert!(matches!(result, Err(JsError::FunctionNotFound { .. })));
    }

    #[test]
    fn test_client_persistence() {
        let mut runtime = JsRuntime::new();

        let source = r#"
            function filter_increment(input) {
                if (!client.global.counter) {
                    client.global.counter = 0;
                }
                client.global.counter++;
                return input + client.global.counter;
            }
        "#;
        runtime
            .context
            .eval(Source::from_bytes(source))
            .expect("Failed to load source");

        // First call
        let result1 = runtime
            .call_filter("increment", &Value::Number(Number::Integer(100)), &[])
            .unwrap();
        assert_eq!(result1, Value::Number(Number::Integer(101)));

        // Second call - counter should be 2
        let result2 = runtime
            .call_filter("increment", &Value::Number(Number::Integer(100)), &[])
            .unwrap();
        assert_eq!(result2, Value::Number(Number::Integer(102)));
    }

    #[test]
    fn test_response_access() {
        let mut runtime = JsRuntime::new();

        // Set response
        runtime.set_response(JsResponse {
            status: 200,
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            body: b"test body".to_vec(),
            content_type: Some("application/json".to_string()),
            url: "http://example.com".to_string(),
        });

        let source = r#"
            function filter_with_status(input) {
                return input + " (status=" + response.status + ")";
            }
        "#;
        runtime
            .context
            .eval(Source::from_bytes(source))
            .expect("Failed to load source");

        let result = runtime
            .call_filter("with_status", &Value::String("result".to_string()), &[])
            .unwrap();

        assert_eq!(
            result,
            Value::String("result (status=200)".to_string())
        );
    }
}
