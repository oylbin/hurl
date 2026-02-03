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

function filter_jsonstr(input) {
    return JSON.stringify(input);
}