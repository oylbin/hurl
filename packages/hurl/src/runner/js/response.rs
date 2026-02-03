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

//! JavaScript `response` global object.
//!
//! The `response` object provides read-only access to the current HTTP response.
//! Similar to JetBrains HTTP Client's response object.

use boa_engine::{js_string, Context, JsObject, JsResult, JsValue};

use crate::http::Response;

/// Response object that provides access to HTTP response data.
///
/// This object is read-only and is updated before each filter evaluation.
#[derive(Clone, Debug)]
pub struct JsResponse {
    /// HTTP status code.
    pub status: u32,
    /// Response headers.
    pub headers: Vec<(String, String)>,
    /// Response body as bytes.
    pub body: Vec<u8>,
    /// Content-Type header value.
    pub content_type: Option<String>,
    /// Response URL.
    pub url: String,
}

impl JsResponse {
    /// Creates a new response object from an HTTP response.
    pub fn from_http_response(response: &Response) -> Self {
        let headers: Vec<(String, String)> = response
            .headers
            .iter()
            .map(|h| (h.name.clone(), h.value.clone()))
            .collect();

        let content_type = response
            .headers
            .get("Content-Type")
            .map(|h| h.value.clone());

        Self {
            status: response.status,
            headers,
            body: response.body.clone(),
            content_type,
            url: response.url.to_string(),
        }
    }

    /// Creates a default/empty response object.
    pub fn empty() -> Self {
        Self {
            status: 0,
            headers: Vec::new(),
            body: Vec::new(),
            content_type: None,
            url: String::new(),
        }
    }

    /// Converts the response to a JavaScript object.
    pub fn to_js_object(&self, context: &mut Context) -> JsResult<JsObject> {
        let obj = JsObject::with_null_proto();

        // status (number)
        obj.set(
            js_string!("status"),
            JsValue::from(self.status as i32),
            false,
            context,
        )?;

        // url (string)
        obj.set(
            js_string!("url"),
            JsValue::from(js_string!(self.url.as_str())),
            false,
            context,
        )?;

        // contentType (string or null)
        let content_type_value = match &self.content_type {
            Some(ct) => JsValue::from(js_string!(ct.as_str())),
            None => JsValue::null(),
        };
        obj.set(js_string!("contentType"), content_type_value, false, context)?;

        // headers (object with arrays for multi-value headers)
        let headers_obj = JsObject::with_null_proto();
        for (name, value) in &self.headers {
            let existing = headers_obj.get(js_string!(name.as_str()), context)?;
            if existing.is_undefined() {
                // First occurrence - create array
                let arr = boa_engine::object::builtins::JsArray::new(context);
                arr.push(JsValue::from(js_string!(value.as_str())), context)?;
                headers_obj.set(js_string!(name.as_str()), JsValue::from(arr), false, context)?;
            } else if let Some(arr_obj) = existing.as_object() {
                // Subsequent occurrence - push to array
                let arr = boa_engine::object::builtins::JsArray::from_object(arr_obj.clone())?;
                arr.push(JsValue::from(js_string!(value.as_str())), context)?;
            }
        }
        obj.set(js_string!("headers"), JsValue::from(headers_obj), false, context)?;

        // body (string, attempting UTF-8 decode)
        let body_str = String::from_utf8_lossy(&self.body);
        obj.set(
            js_string!("body"),
            JsValue::from(js_string!(body_str.as_ref())),
            false,
            context,
        )?;

        Ok(obj)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_to_js_object() {
        let mut context = Context::default();
        let response = JsResponse {
            status: 200,
            headers: vec![
                ("Content-Type".to_string(), "application/json".to_string()),
                ("X-Custom".to_string(), "value1".to_string()),
                ("X-Custom".to_string(), "value2".to_string()),
            ],
            body: b"hello".to_vec(),
            content_type: Some("application/json".to_string()),
            url: "http://example.com".to_string(),
        };

        let js_obj = response.to_js_object(&mut context).unwrap();

        // Check status
        let status = js_obj.get(js_string!("status"), &mut context).unwrap();
        assert_eq!(status.as_number(), Some(200.0));

        // Check contentType
        let ct = js_obj.get(js_string!("contentType"), &mut context).unwrap();
        assert_eq!(
            ct.as_string().map(|s| s.to_std_string_escaped()),
            Some("application/json".to_string())
        );

        // Check body
        let body = js_obj.get(js_string!("body"), &mut context).unwrap();
        assert_eq!(
            body.as_string().map(|s| s.to_std_string_escaped()),
            Some("hello".to_string())
        );
    }
}
