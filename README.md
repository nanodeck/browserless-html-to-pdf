![Build](https://github.com/nanodeck/browserless-html-to-pdf/actions/workflows/release.yml/badge.svg)
![License: MIT](https://img.shields.io/github/license/nanodeck/browserless-html-to-pdf)
![Docker](https://img.shields.io/badge/ghcr.io-nanodeck%2Fbrowserless--html--to--pdf-blue?logo=docker)

# Browserless HTML to PDF

A self-hosted Rust HTTP service that converts **HTML to PDF** and renders **PDF to images** (PNG/JPEG) — without a headless browser. The entire pipeline is pure Rust and runs in-process, so the image stays small (~50–60 MB) and starts instantly.

## Features

- **HTML → PDF** — convert raw HTML with page format, orientation, margins, and header/footer support
- **PDF → image** — rasterize PDF pages to PNG or JPEG with scale and page selection
- **No headless browser** — pure in-process rendering, no external rendering engine to install or manage
- **Bundled fonts** — common font families ship on disk so HTML resolves them by name
- **Flexible output** — inline base64 by default, or time-limited signed download URLs backed by local disk, S3, or GCS
- **Hardened defaults** — request size caps and per-IP rate limiting
- **OpenAPI docs** — interactive API reference served at `/`
- **Multi-arch image** — `linux/amd64` and `linux/arm64`

## Quickstart

```bash
docker run --rm -p 3000:3000 ghcr.io/nanodeck/browserless-html-to-pdf:latest
```

Then open the interactive API docs at <http://localhost:3000/>.

### Build locally

```bash
docker build -t browserless-html-to-pdf:slim .
docker run --rm -p 3000:3000 browserless-html-to-pdf:slim
```

## API

| Method | Path             | Description                                   |
| ------ | ---------------- | --------------------------------------------- |
| `GET`  | `/`              | Interactive API documentation                 |
| `GET`  | `/openapi.json`  | OpenAPI specification                         |
| `GET`  | `/health`        | Health check                                  |
| `POST` | `/v1/pdf`        | Convert HTML to a PDF                         |
| `POST` | `/v1/images`     | Render a PDF to PNG/JPEG images               |
| `GET`  | `/downloads/{key}` | Download a stored artifact (signed-URL mode) |

### `POST /v1/pdf`

Request body (JSON):

```json
{
  "html": "<h1>Hello</h1>",
  "page": { "format": "A4", "landscape": false, "margin": "20mm" },
  "header": "optional header html",
  "footer": "optional footer html",
  "filename": "report.pdf"
}
```

Only `html` is required. Margins accept CSS units (`pt`, `px`, `in`, `cm`, `mm`).

```bash
curl -X POST http://localhost:3000/v1/pdf \
  -H 'content-type: application/json' \
  -d '{"html":"<h1>Hello</h1>","page":{"format":"A4"}}'
```

By default the response is JSON with the PDF inline as base64. With storage enabled
(`STORAGE_ENABLED=true`) it instead returns a `downloadUrl`.

### `POST /v1/images`

Accepts either a JSON body or `multipart/form-data` file upload.

```bash
curl -X POST http://localhost:3000/v1/images \
  -H 'content-type: application/json' \
  -d '{"pdf_base64":"<base64-pdf>","format":"png","scale":1,"pages":"1-3"}'
```

- `format` — `png` (default) or `jpeg`
- `scale` — render scale factor greater than `0` and up to `1.0` (default `1.0`)
- `pages` — selection like `1,3,5-7` (default: all pages)

## Configuration

All settings are read from environment variables (a local `.env` is loaded if present).
See [`.env.example`](.env.example) for the full list.

| Variable             | Default                  | Description                                            |
| -------------------- | ------------------------ | ------------------------------------------------------ |
| `PORT`               | `3000`                   | Listen port                                            |
| `STORAGE_ENABLED`    | `false`                  | `true` switches responses to signed-URL mode           |
| `OPENDAL_SCHEME`     | `fs`                     | Storage backend: `fs`, `s3`, `gcs`, `azblob`, `memory` |
| `OPENDAL_ROOT`       | `./storage`              | Backend root path (fs)                                 |
| `PUBLIC_BASE_URL`    | `http://localhost:3000`  | Base URL used to build signed download links           |
| `SIGNING_KEY`        | random per boot          | HMAC key for fs signed URLs; required and at least 32 bytes when fs storage is enabled |
| `URL_TTL_SECS`       | `3600`                   | Signed-URL lifetime, from `1` to `604800` seconds      |
| `MAX_BODY_BYTES`     | `8388608`                | Outer request body cap                                 |
| `MAX_HTML_BYTES`     | `2097152`                | `/v1/pdf` HTML cap (`413` over limit)                  |
| `MAX_IMAGE_PAGES`    | `10`                     | `/v1/images` page cap                                  |
| `RATE_LIMIT_PER_MIN` | `60`                     | Per-IP request cap                                     |
| `MAX_CONCURRENT_RENDERS` | `4`                  | Maximum concurrent CPU-bound render jobs               |

## Fonts

Common font families are installed on disk so HTML resolves them by name, including the
Microsoft Core fonts (Arial, Times New Roman, Georgia, Verdana, Courier New, Trebuchet MS,
Comic Sans MS, Impact, Webdings, Arial Black, Andale Mono), Liberation (Sans/Serif/Mono),
and Open Sans. A core set is also embedded directly in the binary as a fallback.

## Development

```bash
cargo run                                    # start the service
cargo test                                   # run all tests
cargo fmt                                     # format
cargo clippy --all-targets --all-features     # lint
```

## License

[MIT](LICENSE)
