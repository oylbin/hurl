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
use hurl_core::ast::{NumberValue, Placeholder, SourceInfo};

use crate::runner::{expr, Number, RunnerError, RunnerErrorKind, Value, VariableSet};

/// Adds a number `addend` to the input `value`.
pub fn eval_add(
    value: &Value,
    addend: &NumberValue,
    variables: &VariableSet,
    source_info: SourceInfo,
    assert: bool,
) -> Result<Option<Value>, RunnerError> {
    let addend = eval_number_value(addend, variables)?;

    match value {
        Value::Number(n) => {
            let result = add_numbers(n, &addend);
            Ok(Some(Value::Number(result)))
        }
        Value::String(s) => {
            // Try to parse string as number
            if let Ok(i) = s.parse::<i64>() {
                let n = Number::Integer(i);
                let result = add_numbers(&n, &addend);
                Ok(Some(Value::Number(result)))
            } else if let Ok(f) = s.parse::<f64>() {
                let n = Number::Float(f);
                let result = add_numbers(&n, &addend);
                Ok(Some(Value::Number(result)))
            } else {
                let kind = RunnerErrorKind::FilterInvalidInput(value.repr());
                Err(RunnerError::new(source_info, kind, assert))
            }
        }
        v => {
            let kind = RunnerErrorKind::FilterInvalidInput(v.repr());
            Err(RunnerError::new(source_info, kind, assert))
        }
    }
}

/// Adds two numbers, handling type promotion.
fn add_numbers(a: &Number, b: &Number) -> Number {
    match (a, b) {
        (Number::Integer(i1), Number::Integer(i2)) => {
            // Use wrapping_add for silent overflow
            Number::Integer(i1.wrapping_add(*i2))
        }
        (Number::Integer(i), Number::Float(f)) => Number::Float(*i as f64 + f),
        (Number::Float(f), Number::Integer(i)) => Number::Float(f + *i as f64),
        (Number::Float(f1), Number::Float(f2)) => Number::Float(f1 + f2),
        // BigInteger is not supported for arithmetic operations
        (Number::BigInteger(_), _) | (_, Number::BigInteger(_)) => {
            // Fall back to float conversion for BigInteger
            let f1 = number_to_f64(a);
            let f2 = number_to_f64(b);
            Number::Float(f1 + f2)
        }
    }
}

/// Converts a Number to f64.
fn number_to_f64(n: &Number) -> f64 {
    match n {
        Number::Integer(i) => *i as f64,
        Number::Float(f) => *f,
        Number::BigInteger(s) => s.parse::<f64>().unwrap_or(f64::NAN),
    }
}

/// Evaluates a [`NumberValue`] against a variable set.
fn eval_number_value(n: &NumberValue, variables: &VariableSet) -> Result<Number, RunnerError> {
    match n {
        NumberValue::Literal(number) => Ok(ast_number_to_runner_number(number)),
        NumberValue::Placeholder(Placeholder { expr, .. }) => match expr::eval(expr, variables)? {
            Value::Number(number) => Ok(number),
            v => {
                let kind = RunnerErrorKind::ExpressionInvalidType {
                    value: v.repr(),
                    expecting: "number".to_string(),
                };
                Err(RunnerError::new(expr.source_info, kind, false))
            }
        },
    }
}

/// Converts an AST Number to a runner Number.
fn ast_number_to_runner_number(n: &hurl_core::ast::Number) -> Number {
    match n {
        hurl_core::ast::Number::Integer(i) => Number::Integer(i.as_i64()),
        hurl_core::ast::Number::Float(f) => Number::Float(f.as_f64()),
        hurl_core::ast::Number::BigInteger(s) => Number::BigInteger(s.clone()),
    }
}

#[cfg(test)]
mod tests {
    use hurl_core::ast::{Filter, FilterValue, NumberValue, SourceInfo, Whitespace, I64};
    use hurl_core::reader::Pos;
    use hurl_core::types::ToSource;

    use crate::runner::filter::eval::eval_filter;
    use crate::runner::{Number, RunnerErrorKind, Value, VariableSet};

    fn whitespace() -> Whitespace {
        Whitespace {
            value: String::new(),
            source_info: SourceInfo::new(Pos::new(0, 0), Pos::new(0, 0)),
        }
    }

    #[test]
    fn eval_filter_add_integer_to_integer() {
        let variables = VariableSet::new();
        let filter = Filter {
            source_info: SourceInfo::new(Pos::new(1, 1), Pos::new(1, 1)),
            value: FilterValue::Add {
                space0: whitespace(),
                value: NumberValue::Literal(hurl_core::ast::Number::Integer(I64::new(
                    5,
                    "5".to_source(),
                ))),
            },
        };

        assert_eq!(
            eval_filter(
                &filter,
                &Value::Number(Number::Integer(10)),
                &variables,
                false
            )
            .unwrap()
            .unwrap(),
            Value::Number(Number::Integer(15))
        );
    }

    #[test]
    fn eval_filter_add_negative() {
        let variables = VariableSet::new();
        let filter = Filter {
            source_info: SourceInfo::new(Pos::new(1, 1), Pos::new(1, 1)),
            value: FilterValue::Add {
                space0: whitespace(),
                value: NumberValue::Literal(hurl_core::ast::Number::Integer(I64::new(
                    -3,
                    "-3".to_source(),
                ))),
            },
        };

        assert_eq!(
            eval_filter(
                &filter,
                &Value::Number(Number::Integer(10)),
                &variables,
                false
            )
            .unwrap()
            .unwrap(),
            Value::Number(Number::Integer(7))
        );
    }

    #[test]
    fn eval_filter_add_float_to_integer() {
        let variables = VariableSet::new();
        let filter = Filter {
            source_info: SourceInfo::new(Pos::new(1, 1), Pos::new(1, 1)),
            value: FilterValue::Add {
                space0: whitespace(),
                value: NumberValue::Literal(hurl_core::ast::Number::Float(
                    hurl_core::ast::Float::new(1.5, "1.5".to_source()),
                )),
            },
        };

        assert_eq!(
            eval_filter(
                &filter,
                &Value::Number(Number::Integer(10)),
                &variables,
                false
            )
            .unwrap()
            .unwrap(),
            Value::Number(Number::Float(11.5))
        );
    }

    #[test]
    fn eval_filter_add_integer_to_float() {
        let variables = VariableSet::new();
        let filter = Filter {
            source_info: SourceInfo::new(Pos::new(1, 1), Pos::new(1, 1)),
            value: FilterValue::Add {
                space0: whitespace(),
                value: NumberValue::Literal(hurl_core::ast::Number::Integer(I64::new(
                    5,
                    "5".to_source(),
                ))),
            },
        };

        assert_eq!(
            eval_filter(
                &filter,
                &Value::Number(Number::Float(10.5)),
                &variables,
                false
            )
            .unwrap()
            .unwrap(),
            Value::Number(Number::Float(15.5))
        );
    }

    #[test]
    fn eval_filter_add_float_to_float() {
        let variables = VariableSet::new();
        let filter = Filter {
            source_info: SourceInfo::new(Pos::new(1, 1), Pos::new(1, 1)),
            value: FilterValue::Add {
                space0: whitespace(),
                value: NumberValue::Literal(hurl_core::ast::Number::Float(
                    hurl_core::ast::Float::new(2.5, "2.5".to_source()),
                )),
            },
        };

        assert_eq!(
            eval_filter(
                &filter,
                &Value::Number(Number::Float(10.5)),
                &variables,
                false
            )
            .unwrap()
            .unwrap(),
            Value::Number(Number::Float(13.0))
        );
    }

    #[test]
    fn eval_filter_add_to_string() {
        let variables = VariableSet::new();
        let filter = Filter {
            source_info: SourceInfo::new(Pos::new(1, 1), Pos::new(1, 1)),
            value: FilterValue::Add {
                space0: whitespace(),
                value: NumberValue::Literal(hurl_core::ast::Number::Integer(I64::new(
                    5,
                    "5".to_source(),
                ))),
            },
        };

        // String that can be parsed as integer
        assert_eq!(
            eval_filter(&filter, &Value::String("10".to_string()), &variables, false)
                .unwrap()
                .unwrap(),
            Value::Number(Number::Integer(15))
        );

        // String that can be parsed as float
        assert_eq!(
            eval_filter(
                &filter,
                &Value::String("10.5".to_string()),
                &variables,
                false
            )
            .unwrap()
            .unwrap(),
            Value::Number(Number::Float(15.5))
        );
    }

    #[test]
    fn eval_filter_add_overflow() {
        let variables = VariableSet::new();
        let filter = Filter {
            source_info: SourceInfo::new(Pos::new(1, 1), Pos::new(1, 1)),
            value: FilterValue::Add {
                space0: whitespace(),
                value: NumberValue::Literal(hurl_core::ast::Number::Integer(I64::new(
                    1,
                    "1".to_source(),
                ))),
            },
        };

        // Test wrapping overflow
        assert_eq!(
            eval_filter(
                &filter,
                &Value::Number(Number::Integer(i64::MAX)),
                &variables,
                false
            )
            .unwrap()
            .unwrap(),
            Value::Number(Number::Integer(i64::MIN))
        );
    }

    #[test]
    fn eval_filter_add_invalid_input() {
        let variables = VariableSet::new();
        let filter = Filter {
            source_info: SourceInfo::new(Pos::new(1, 1), Pos::new(1, 1)),
            value: FilterValue::Add {
                space0: whitespace(),
                value: NumberValue::Literal(hurl_core::ast::Number::Integer(I64::new(
                    5,
                    "5".to_source(),
                ))),
            },
        };

        // Boolean input
        let err = eval_filter(&filter, &Value::Bool(true), &variables, false)
            .err()
            .unwrap();
        assert_eq!(
            err.kind,
            RunnerErrorKind::FilterInvalidInput("boolean <true>".to_string())
        );

        // Invalid string
        let err = eval_filter(
            &filter,
            &Value::String("not a number".to_string()),
            &variables,
            false,
        )
        .err()
        .unwrap();
        assert_eq!(
            err.kind,
            RunnerErrorKind::FilterInvalidInput("string <not a number>".to_string())
        );
    }
}
