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

//! JavaScript runtime support for custom filters.
//!
//! This module provides the ability to extend Hurl's filter capabilities
//! using JavaScript code loaded via the `--jsfilter` CLI option.

mod client;
mod convert;
mod error;
mod response;
mod runtime;

pub use client::JsClient;
pub use error::JsError;
pub use response::JsResponse;
pub use runtime::JsRuntime;
