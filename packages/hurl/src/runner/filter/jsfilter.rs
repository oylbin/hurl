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

//! JavaScript filter evaluator.

use std::cell::RefCell;
use std::path::PathBuf;

use hurl_core::ast::{SourceInfo, Template, Whitespace};

use crate::runner::js::{JsError, JsRuntime};
use crate::runner::template::eval_template;
use crate::runner::{RunnerError, RunnerErrorKind, Value, VariableSet};

thread_local! {
    /// Thread-local JavaScript runtime.
    /// Each thread gets its own runtime instance, initialized lazily from the file path.
    static JS_RUNTIME: RefCell<Option<(PathBuf, JsRuntime)>> = const { RefCell::new(None) };
}

/// Gets or creates a thread-local JsRuntime for the given path.
fn with_js_runtime<F, R>(
    path: &PathBuf,
    source_info: SourceInfo,
    in_assert: bool,
    f: F,
) -> Result<R, RunnerError>
where
    F: FnOnce(&mut JsRuntime) -> Result<R, JsError>,
{
    JS_RUNTIME.with(|cell| {
        let mut opt = cell.borrow_mut();

        // Check if we need to initialize or re-initialize the runtime
        let needs_init = match &*opt {
            None => true,
            Some((existing_path, _)) => existing_path != path,
        };

        if needs_init {
            let mut runtime = JsRuntime::new();
            runtime
                .load_file(path)
                .map_err(|e| js_error_to_runner_error(e, source_info, in_assert))?;
            *opt = Some((path.clone(), runtime));
        }

        let (_, runtime) = opt.as_mut().unwrap();
        f(runtime).map_err(|e| js_error_to_runner_error(e, source_info, in_assert))
    })
}

/// Evaluates a JavaScript filter.
///
/// The filter function is looked up in the JavaScript runtime by name with
/// the `filter_` prefix. For example, `jsfilter add 1` will call `filter_add(input, 1)`.
pub fn eval_jsfilter(
    value: &Value,
    name: &Template,
    args: &[(Whitespace, Template)],
    variables: &VariableSet,
    jsfilter_path: &Option<PathBuf>,
    source_info: SourceInfo,
    in_assert: bool,
) -> Result<Option<Value>, RunnerError> {
    // Check if JS filter path is configured
    let path = jsfilter_path.as_ref().ok_or_else(|| {
        RunnerError::new(source_info, RunnerErrorKind::JsFilterNotConfigured, in_assert)
    })?;

    // Evaluate the function name
    let name_str = eval_template(name, variables)?;

    // Evaluate arguments
    let mut evaluated_args = Vec::with_capacity(args.len());
    for (_, arg_template) in args {
        let arg_str = eval_template(arg_template, variables)?;
        // Try to parse as number, otherwise keep as string
        let arg_value = parse_arg_value(&arg_str);
        evaluated_args.push(arg_value);
    }

    // Call the JavaScript function using thread-local runtime
    let result = with_js_runtime(path, source_info, in_assert, |runtime| {
        runtime.call_filter(&name_str, value, &evaluated_args)
    })?;

    Ok(Some(result))
}

/// Converts a JavaScript error to a runner error.
fn js_error_to_runner_error(
    error: JsError,
    source_info: SourceInfo,
    in_assert: bool,
) -> RunnerError {
    let kind = match error {
        JsError::FunctionNotFound { name } => RunnerErrorKind::JsFilterFunctionNotFound { name },
        JsError::RuntimeError { message } => RunnerErrorKind::JsFilterRuntimeError { message },
        JsError::ConversionError { message } => RunnerErrorKind::JsFilterRuntimeError { message },
        JsError::FileLoadError { message, .. } => RunnerErrorKind::JsFilterRuntimeError { message },
        JsError::ParseError { message } => RunnerErrorKind::JsFilterRuntimeError { message },
    };
    RunnerError::new(source_info, kind, in_assert)
}

/// Parses an argument string into a Value.
///
/// Tries to parse as integer, then float, then keeps as string.
fn parse_arg_value(s: &str) -> Value {
    // Try integer
    if let Ok(i) = s.parse::<i64>() {
        return Value::Number(crate::runner::Number::Integer(i));
    }
    // Try float
    if let Ok(f) = s.parse::<f64>() {
        return Value::Number(crate::runner::Number::Float(f));
    }
    // Keep as string
    Value::String(s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::Number;

    #[test]
    fn test_parse_arg_value_integer() {
        assert_eq!(
            parse_arg_value("42"),
            Value::Number(Number::Integer(42))
        );
        assert_eq!(
            parse_arg_value("-10"),
            Value::Number(Number::Integer(-10))
        );
    }

    #[test]
    fn test_parse_arg_value_float() {
        assert_eq!(
            parse_arg_value("3.14"),
            Value::Number(Number::Float(3.14))
        );
    }

    #[test]
    fn test_parse_arg_value_string() {
        assert_eq!(
            parse_arg_value("hello"),
            Value::String("hello".to_string())
        );
    }
}
