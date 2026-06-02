use serde_json::{Value, json};

pub fn spec() -> Value {
    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Browserless HTML to PDF",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Pure-Rust, browserless service for HTML→PDF and PDF→image conversion."
        },
        "paths": {
            "/": {
                "get": {
                    "summary": "Scalar API reference",
                    "description": "Interactive API documentation rendered by Scalar.",
                    "responses": {
                        "200": { "description": "HTML documentation page." }
                    }
                }
            },
            "/openapi.json": {
                "get": {
                    "summary": "OpenAPI specification",
                    "description": "This OpenAPI document.",
                    "responses": {
                        "200": {
                            "description": "The OpenAPI specification as JSON.",
                            "content": { "application/json": {} }
                        }
                    }
                }
            },
            "/health": { "get": { "summary": "Liveness", "responses": { "200": { "description": "OK" } } } },
            "/v1/pdf": { "post": {
                "summary": "HTML → PDF",
                "requestBody": { "required": true, "content": { "application/json": { "schema": {
                    "type": "object", "required": ["html"], "properties": {
                        "html": { "type": "string" },
                        "page": { "type": "object", "properties": {
                            "format": { "type": "string", "examples": ["A4", "Letter"] },
                            "landscape": { "type": "boolean" },
                            "margin": { "type": "string", "examples": ["1cm"] } } },
                        "header": { "type": "string" },
                        "footer": { "type": "string" },
                        "filename": { "type": "string" } } } } } },
                "responses": {
                    "200": { "description": "PDF (base64) or signed downloadUrl" },
                    "422": { "description": "Validation / unsupported option" } } } },
            "/v1/images": { "post": {
                "summary": "PDF → PNG/JPEG",
                "description": "Upload a PDF file (multipart/form-data) or send it base64-encoded (application/json).",
                "requestBody": { "required": true, "content": {
                    "multipart/form-data": { "schema": { "type": "object", "required": ["file"],
                        "properties": {
                            "file": { "type": "string", "format": "binary", "description": "PDF file to rasterize" },
                            "format": { "type": "string", "enum": ["png", "jpeg"] },
                            "scale": { "type": "number", "exclusiveMinimum": 0, "maximum": 1.0 },
                            "pages": { "type": "string", "examples": ["1-3,5"] } } } },
                    "application/json": { "schema": { "type": "object", "required": ["pdf_base64"],
                        "properties": {
                            "pdf_base64": { "type": "string" },
                            "format": { "type": "string", "enum": ["png", "jpeg"] },
                            "scale": { "type": "number", "exclusiveMinimum": 0, "maximum": 1.0 },
                            "pages": { "type": "string", "examples": ["1-3,5"] } } } } } },
                "responses": { "200": { "description": "Images (base64) or signed downloadUrls" },
                    "422": { "description": "Validation error" } } } }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_lists_new_paths() {
        let s = spec();
        assert!(s["paths"]["/v1/pdf"].is_object());
        assert!(s["paths"]["/v1/images"].is_object());
        assert!(s["paths"]["/health"].is_object());
    }
}
