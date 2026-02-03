# Design Plan: JavaScript Filter Extension for Hurl

## Overview

This plan describes the implementation of a `jsfilter` feature that allows users to extend Hurl's filtering capabilities using JavaScript code. The feature introduces:

1. **`--jsfilter <file.js>` CLI option** - Load a JavaScript file containing custom filter functions
2. **`jsfilter <name> [args...]` syntax** - Call JavaScript filter functions from Hurl files
3. **`client` and `response` global variables** - Access session metadata and response data in JavaScript

## Architecture Decision: JavaScript Engine

**Recommended: [boa_engine](https://github.com/nickmass/boa)** - A pure-Rust JavaScript engine

Alternatives considered:
- **V8 (via rusty_v8/deno_core)**: Most complete but heavy dependency, complex build
- **QuickJS (via rquickjs)**: Lightweight C library, requires FFI
- **boa_engine**: Pure Rust, good ECMAScript compliance, moderate performance, easy integration

`boa_engine` is the best choice because:
- Pure Rust = no C dependencies, easy cross-platform builds
- Active development with good ES6+ support
- Reasonable performance for filter operations
- Simple API for embedding

## Implementation Plan

### Phase 1: Core Infrastructure

#### 1.1 Add boa_engine dependency

**File: `packages/hurl/Cargo.toml`**
```toml
[dependencies]
# ... existing deps
boa_engine = "0.20"  # or latest stable
```

#### 1.2 Create JavaScript Runtime Module

**New File: `packages/hurl/src/runner/js/mod.rs`**
```rust
mod runtime;
mod client;
mod response;

pub use runtime::JsRuntime;
pub use client::JsClient;
pub use response::JsResponse;
```

**New File: `packages/hurl/src/runner/js/runtime.rs`**

Core JavaScript runtime wrapper:
- Initialize boa_engine Context
- Load user's JavaScript file
- Register `client` and `response` global objects
- Provide `call_filter(name, input, args)` method

```rust
use boa_engine::{Context, JsValue, Source};
use std::path::Path;

pub struct JsRuntime {
    context: Context,
}

impl JsRuntime {
    pub fn new() -> Self {
        let context = Context::default();
        Self { context }
    }

    pub fn load_file(&mut self, path: &Path) -> Result<(), JsError> {
        let source = std::fs::read_to_string(path)?;
        self.context.eval(Source::from_bytes(&source))?;
        Ok(())
    }

    pub fn set_client(&mut self, client: JsClient) { ... }
    pub fn set_response(&mut self, response: JsResponse) { ... }

    pub fn call_filter(
        &mut self,
        name: &str,
        input: &Value,
        args: &[Value],
    ) -> Result<Value, JsError> {
        // Look up function `filter_{name}`
        // Convert input and args to JsValue
        // Call function
        // Convert result back to hurl Value
    }
}
```

#### 1.3 Value Conversion Layer

**New File: `packages/hurl/src/runner/js/convert.rs`**

Bidirectional conversion between Hurl `Value` and `JsValue`:

```rust
pub fn value_to_js(value: &Value, context: &mut Context) -> JsValue {
    match value {
        Value::String(s) => JsValue::from(s.as_str()),
        Value::Number(Number::Integer(i)) => JsValue::from(*i),
        Value::Number(Number::Float(f)) => JsValue::from(*f),
        Value::Bool(b) => JsValue::from(*b),
        Value::Null => JsValue::null(),
        Value::List(items) => { /* convert to JS Array */ }
        Value::Object(pairs) => { /* convert to JS Object */ }
        Value::Bytes(bytes) => { /* convert to Uint8Array */ }
        // ...
    }
}

pub fn js_to_value(js_value: JsValue, context: &mut Context) -> Result<Value, JsError> {
    // Reverse conversion
}
```

### Phase 2: CLI Integration

#### 2.1 Add --jsfilter CLI argument

**File: `packages/hurl/src/cli/options/commands.rs`**

Add new function:
```rust
pub fn jsfilter() -> clap::Arg {
    clap::Arg::new("jsfilter")
        .long("jsfilter")
        .value_name("FILE")
        .help("Load JavaScript file containing custom filter functions")
        .help_heading("Run options")
        .num_args(1)
}
```

**File: `packages/hurl/src/cli/options/mod.rs`**

Add to `CliOptions` struct:
```rust
pub struct CliOptions {
    // ... existing fields
    pub jsfilter: Option<PathBuf>,
}
```

Add to argument registration in `parse()` function.

#### 2.2 Add to RunnerOptions

**File: `packages/hurl/src/runner/runner_options.rs`**

```rust
pub struct RunnerOptions {
    // ... existing fields
    pub js_runtime: Option<Arc<Mutex<JsRuntime>>>,
}

impl RunnerOptionsBuilder {
    pub fn js_runtime(&mut self, runtime: Option<Arc<Mutex<JsRuntime>>>) -> &mut Self {
        self.js_runtime = runtime;
        self
    }
}
```

#### 2.3 Initialize JsRuntime at startup

**File: `packages/hurl/src/run.rs`**

In `run_seq()` and `run_par()`:
```rust
// Load JavaScript filter file if specified
let js_runtime = if let Some(jsfilter_path) = &options.jsfilter {
    let mut runtime = JsRuntime::new();
    runtime.load_file(jsfilter_path)?;
    Some(Arc::new(Mutex::new(runtime)))
} else {
    None
};
```

### Phase 3: Parser Changes (hurl_core)

#### 3.1 Add JsFilter to AST

**File: `packages/hurl_core/src/ast/core.rs`**

Add new variant to `FilterValue`:
```rust
pub enum FilterValue {
    // ... existing variants

    /// JavaScript filter: `jsfilter <name> [arg1] [arg2] ...`
    JsFilter {
        space0: Whitespace,
        name: String,                    // Function name (without filter_ prefix)
        space1: Whitespace,
        args: Vec<(Whitespace, Expr)>,   // Variable number of arguments
    },
}

impl FilterValue {
    pub fn identifier(&self) -> &'static str {
        match self {
            // ... existing
            FilterValue::JsFilter { .. } => "jsfilter",
        }
    }
}
```

#### 3.2 Add JsFilter Parser

**File: `packages/hurl_core/src/parser/filter.rs`**

Add parser function:
```rust
fn jsfilter(reader: &mut Reader) -> ParseResult<FilterValue> {
    try_literal("jsfilter", reader)?;
    let space0 = one_or_more_spaces(reader)?;

    // Parse filter name (identifier)
    let name = identifier(reader)?;

    // Parse optional arguments (expressions/templates)
    let mut args = Vec::new();
    loop {
        let save = reader.cursor();
        match one_or_more_spaces(reader) {
            Ok(space) => {
                // Try to parse an expression (template/variable/literal)
                match expr(reader) {
                    Ok(arg_expr) => args.push((space, arg_expr)),
                    Err(_) => {
                        reader.seek(save);
                        break;
                    }
                }
            }
            Err(_) => break,
        }
    }

    let space1 = Whitespace::default(); // or capture trailing
    Ok(FilterValue::JsFilter { space0, name, space1, args })
}
```

Add to choice array in `filter()`:
```rust
let value = choice(
    &[
        // ... existing filters
        jsfilter,
    ],
    reader,
)?;
```

### Phase 4: Runtime Evaluation

#### 4.1 Add JsFilter Evaluator

**New File: `packages/hurl/src/runner/filter/jsfilter.rs`**

```rust
use crate::runner::js::JsRuntime;
use crate::runner::value::Value;
use crate::runner::variable::VariableSet;
use hurl_core::ast::{Expr, SourceInfo};
use std::sync::{Arc, Mutex};

pub fn eval_jsfilter(
    value: &Value,
    name: &str,
    args: &[(hurl_core::ast::Whitespace, Expr)],
    variables: &VariableSet,
    js_runtime: &Option<Arc<Mutex<JsRuntime>>>,
    source_info: SourceInfo,
    in_assert: bool,
) -> Result<Option<Value>, RunnerError> {
    // Check if JS runtime is available
    let runtime = js_runtime.as_ref().ok_or_else(|| {
        RunnerError::new(
            source_info,
            RunnerErrorKind::JsFilterNotConfigured,
            in_assert,
        )
    })?;

    // Evaluate arguments
    let evaluated_args: Vec<Value> = args
        .iter()
        .map(|(_, expr)| eval_expr(expr, variables))
        .collect::<Result<_, _>>()?;

    // Call JavaScript function
    let mut runtime = runtime.lock().unwrap();
    let result = runtime.call_filter(name, value, &evaluated_args)?;

    Ok(Some(result))
}
```

#### 4.2 Update Filter Dispatcher

**File: `packages/hurl/src/runner/filter/eval.rs`**

Add import and match arm:
```rust
use crate::runner::filter::jsfilter::eval_jsfilter;

pub fn eval_filter(
    filter: &Filter,
    value: &Value,
    variables: &VariableSet,
    js_runtime: &Option<Arc<Mutex<JsRuntime>>>,  // NEW PARAMETER
    in_assert: bool,
) -> Result<Option<Value>, RunnerError> {
    match &filter.value {
        // ... existing filters

        FilterValue::JsFilter { name, args, .. } => {
            eval_jsfilter(value, name, args, variables, js_runtime, source_info, in_assert)
        }
    }
}
```

**Note:** The `js_runtime` parameter needs to be threaded through all callers of `eval_filter` and `eval_filters`.

#### 4.3 Add Error Kinds

**File: `packages/hurl/src/runner/error.rs`**

```rust
pub enum RunnerErrorKind {
    // ... existing variants

    /// jsfilter used but no --jsfilter file specified
    JsFilterNotConfigured,

    /// JavaScript filter function not found
    JsFilterFunctionNotFound { name: String },

    /// JavaScript runtime error
    JsFilterRuntimeError { message: String },
}
```

### Phase 5: Client and Response Objects

#### 5.1 JsClient Implementation

**New File: `packages/hurl/src/runner/js/client.rs`**

```rust
/// Client object available in JavaScript filters
/// Stores session metadata that persists across requests
pub struct JsClient {
    /// Custom variables that persist across the session
    pub global: HashMap<String, Value>,
}

impl JsClient {
    pub fn new() -> Self {
        Self {
            global: HashMap::new(),
        }
    }

    /// Register as global object in JavaScript context
    pub fn register(context: &mut Context) {
        // Create JS object with:
        // - client.global (object for storing custom data)
        // - client.set(name, value) method
        // - client.get(name) method
    }
}
```

#### 5.2 JsResponse Implementation

**New File: `packages/hurl/src/runner/js/response.rs`**

```rust
/// Response object available in JavaScript filters
/// Read-only access to current HTTP response
pub struct JsResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
    pub content_type: Option<String>,
}

impl JsResponse {
    pub fn from_http_response(response: &HttpResponse) -> Self {
        Self {
            status: response.status,
            headers: response.headers.clone(),
            body: Some(response.body.clone()),
            content_type: response.content_type().map(|s| s.to_string()),
        }
    }

    /// Register as global object in JavaScript context
    pub fn register(context: &mut Context) {
        // Create JS object with:
        // - response.status (number)
        // - response.headers (object)
        // - response.body (string or Uint8Array)
        // - response.contentType (string)
    }
}
```

#### 5.3 Update Response Context

Before evaluating filters that might use `jsfilter`, update the response object:

**File: `packages/hurl/src/runner/entry.rs`**

```rust
// After receiving HTTP response, before evaluating captures/assertions:
if let Some(runtime) = &runner_options.js_runtime {
    let js_response = JsResponse::from_http_response(&http_response);
    let mut runtime = runtime.lock().unwrap();
    runtime.set_response(js_response);
}
```

### Phase 6: Module Registration

#### 6.1 Register JS module

**File: `packages/hurl/src/runner/mod.rs`**

```rust
mod js;  // Add this line
```

**File: `packages/hurl/src/runner/filter/mod.rs`**

```rust
mod jsfilter;  // Add this line
```

### Phase 7: Testing

#### 7.1 Unit Tests

**New File: `packages/hurl/src/runner/js/tests.rs`**

- Test JsRuntime initialization
- Test value conversion (Hurl ↔ JavaScript)
- Test filter function calls
- Test error handling

#### 7.2 Integration Tests

**New Directory: `integration/hurl/tests_ok/jsfilter/`**

Create test files:

**`filters.js`:**
```javascript
function filter_add(input, n) {
    return input + n;
}

function filter_multiply(input, n) {
    return input * n;
}

function filter_sort_keys(input) {
    if (typeof input === 'object' && input !== null) {
        const sorted = {};
        Object.keys(input).sort().forEach(key => {
            sorted[key] = input[key];
        });
        return sorted;
    }
    return input;
}

function filter_sign(input, secret) {
    // Example: simple signature computation
    const json = JSON.stringify(input);
    return json + ":" + secret;
}

function filter_use_response(input) {
    // Access response global
    return input + " (status: " + response.status + ")";
}

function filter_store_and_retrieve(input) {
    // Access client global for session persistence
    if (!client.global.counter) {
        client.global.counter = 0;
    }
    client.global.counter++;
    return input + " (call #" + client.global.counter + ")";
}
```

**`jsfilter.hurl`:**
```hurl
# Test basic jsfilter with add
GET http://localhost:8000/hello
HTTP 200
[Captures]
original: jsonpath "$.count"
incremented: jsonpath "$.count" jsfilter add 10

# Test jsfilter with multiple args
GET http://localhost:8000/data
HTTP 200
[Captures]
computed: jsonpath "$.value" jsfilter multiply 2

# Test jsfilter with object manipulation
GET http://localhost:8000/json
HTTP 200
[Captures]
sorted_data: jsonpath "$" jsfilter sort_keys

# Test jsfilter accessing response
GET http://localhost:8000/hello
HTTP 200
[Captures]
with_status: body jsfilter use_response

# Test jsfilter with session persistence
GET http://localhost:8000/hello
HTTP 200
[Captures]
call1: body jsfilter store_and_retrieve

GET http://localhost:8000/hello
HTTP 200
[Captures]
call2: body jsfilter store_and_retrieve
```

**`jsfilter.sh`:**
```bash
#!/bin/bash
set -e
hurl --jsfilter filters.js jsfilter.hurl
```

## File Change Summary

### New Files
| File | Purpose |
|------|---------|
| `packages/hurl/src/runner/js/mod.rs` | JS module root |
| `packages/hurl/src/runner/js/runtime.rs` | JsRuntime wrapper |
| `packages/hurl/src/runner/js/convert.rs` | Value conversion |
| `packages/hurl/src/runner/js/client.rs` | JsClient object |
| `packages/hurl/src/runner/js/response.rs` | JsResponse object |
| `packages/hurl/src/runner/filter/jsfilter.rs` | Filter evaluator |

### Modified Files
| File | Changes |
|------|---------|
| `packages/hurl/Cargo.toml` | Add `boa_engine` dependency |
| `packages/hurl/src/cli/options/commands.rs` | Add `jsfilter()` arg |
| `packages/hurl/src/cli/options/mod.rs` | Add to CliOptions, parse |
| `packages/hurl/src/cli/options/matches.rs` | Parse jsfilter arg |
| `packages/hurl/src/runner/runner_options.rs` | Add js_runtime field |
| `packages/hurl/src/runner/mod.rs` | Export js module |
| `packages/hurl/src/runner/filter/mod.rs` | Export jsfilter module |
| `packages/hurl/src/runner/filter/eval.rs` | Add JsFilter dispatch |
| `packages/hurl/src/runner/error.rs` | Add JS error kinds |
| `packages/hurl/src/runner/entry.rs` | Update response context |
| `packages/hurl/src/run.rs` | Initialize JsRuntime |
| `packages/hurl_core/src/ast/core.rs` | Add JsFilter variant |
| `packages/hurl_core/src/parser/filter.rs` | Add jsfilter parser |

## Usage Examples

### Basic Usage
```bash
# Run hurl with JavaScript filters
hurl --jsfilter my_filters.js test.hurl
```

### Example filters.js
```javascript
// Simple arithmetic
function filter_add(input, n) {
    return input + n;
}

// JSON manipulation with sorting
function filter_sign_request(input, secret_key) {
    // Sort keys for consistent signing
    const sorted = {};
    Object.keys(input).sort().forEach(key => {
        sorted[key] = input[key];
    });

    // Create signature
    const payload = JSON.stringify(sorted);
    const signature = simpleHash(payload + secret_key);

    // Return modified object with signature
    return {
        ...sorted,
        signature: signature
    };
}

function simpleHash(str) {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
        const char = str.charCodeAt(i);
        hash = ((hash << 5) - hash) + char;
        hash = hash & hash;
    }
    return hash.toString(16);
}

// Access response data
function filter_check_with_response(input) {
    if (response.status >= 400) {
        return "ERROR: " + input;
    }
    return input;
}

// Session persistence
function filter_accumulate(input) {
    if (!client.global.sum) {
        client.global.sum = 0;
    }
    client.global.sum += input;
    return client.global.sum;
}
```

### Example test.hurl
```hurl
# Capture and increment
GET http://api.example.com/counter
HTTP 200
[Captures]
count: jsonpath "$.count"
next_count: jsonpath "$.count" jsfilter add 1

# Sign request data
POST http://api.example.com/secure
{
    "action": "transfer",
    "amount": {{amount}}
}
[Options]
# Prepare signed payload
[Captures]
signed_payload: jsonpath "$" jsfilter sign_request {{api_secret}}

# Use signed payload
POST http://api.example.com/execute
{{signed_payload}}
HTTP 200
```

## Implementation Order

1. **Phase 1**: Core JS infrastructure (runtime, conversion)
2. **Phase 2**: CLI integration (--jsfilter option)
3. **Phase 3**: Parser changes (AST, parser)
4. **Phase 4**: Runtime evaluation (filter dispatch)
5. **Phase 5**: Client/Response objects
6. **Phase 6**: Module registration
7. **Phase 7**: Testing

## Open Questions / Considerations

1. **Error handling**: How verbose should JS error messages be? Include stack traces?

2. **Security**: Should there be sandboxing or restrictions on what JS can do? (No filesystem access by default is good)

3. **Performance**: For parallel execution, each thread would need its own JsRuntime instance. Consider `Arc<Mutex<JsRuntime>>` vs per-thread cloning.

4. **Async support**: boa_engine supports async JS, but initial implementation can be sync-only.

5. **ES Module support**: Should `--jsfilter` support ES modules with imports? Initial implementation can be single-file only.

6. **Multiple filter files**: Should `--jsfilter` be repeatable to load multiple files? Could be useful but adds complexity.
