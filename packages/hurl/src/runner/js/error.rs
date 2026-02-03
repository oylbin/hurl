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

//! JavaScript runtime error types.

use std::fmt;

/// Errors that can occur during JavaScript execution.
#[derive(Clone, Debug)]
pub enum JsError {
    /// Error loading the JavaScript file.
    FileLoadError { path: String, message: String },

    /// Error parsing the JavaScript code.
    ParseError { message: String },

    /// The requested filter function was not found.
    FunctionNotFound { name: String },

    /// Runtime error during JavaScript execution.
    RuntimeError { message: String },

    /// Error converting values between Hurl and JavaScript.
    ConversionError { message: String },
}

impl fmt::Display for JsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JsError::FileLoadError { path, message } => {
                write!(f, "Failed to load JavaScript file '{path}': {message}")
            }
            JsError::ParseError { message } => {
                write!(f, "JavaScript parse error: {message}")
            }
            JsError::FunctionNotFound { name } => {
                write!(f, "JavaScript filter function 'filter_{name}' not found")
            }
            JsError::RuntimeError { message } => {
                write!(f, "JavaScript runtime error: {message}")
            }
            JsError::ConversionError { message } => {
                write!(f, "Value conversion error: {message}")
            }
        }
    }
}

impl std::error::Error for JsError {}
