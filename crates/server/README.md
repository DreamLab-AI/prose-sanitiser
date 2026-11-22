# prose-sanitiser-server

HTTP service exposing the
[prose-sanitiser](https://github.com/DreamLab-AI/agentbox) cleaning pipeline, so
an agent skill, a CI job or a web app can call it without running the CLIs
locally.

**Unpublished** (`publish = false`). It is the deployment surface for the
workspace, not a library anyone should depend on.

## Routes

| Endpoint | Method | Purpose |
|---|---|---|
| `/health` | GET | `{"ok": true, "version": ...}` |
| `/capabilities` | GET | Which optional tools and pixel backends are present |
| `/openapi.json` | GET | The generated OpenAPI 3.0.3 spec |
| `/rules` | GET | The stylistic rule table: ids, tiers, dates, sources and the ruleset changelog |
| `/inspect` | POST | `{"file": "<base64>", "name": "x.png"}` returns findings (`Content-Type: application/json` required) |
| `/clean` | POST | `{"file": ..., "options": {...}}` returns cleaned bytes (`Content-Type: application/json` required) |

## Hardening

Mirrors the CLIs: input size caps, the binary-as-text guard, atomic writes, a
**loopback-only bind by default**, and an optional bearer key
(`WATERMARKS_SERVER_API_KEY`). Intended for a trusted network. Put it behind a
reverse proxy if it is reachable from untrusted clients.

### Browser-origin checks

A web page on the same machine can reach a loopback service, so two checks
apply even with no key set. The key stays optional.

- `POST /inspect` and `POST /clean` require `Content-Type: application/json`
  (parameters such as `charset` are fine); anything else, or no header, is
  `415`. A page can send `text/plain` cross-origin without a CORS preflight,
  and the body would otherwise be parsed as JSON anyway. The service sends no
  CORS headers.
- When bound to a loopback address with no key, every route validates the `Host`
  header against the bound address: `localhost`, `127.0.0.1`, `[::1]` and the
  bound IP literal, bare or with the bound port. Anything else is `421`. This
  stops DNS rebinding, where an attacker's hostname resolves to `127.0.0.1` and
  the page then reads replies as same-origin. The check is off when a key is set
  (a rebound page cannot present it, and a proxy may forward its own `Host`) and
  on a non-loopback bind (the names are not knowable from here).

The clean options the service accepts are an explicit allowlist rather than a
pass-through, so a request cannot reach a flag the service did not intend to
expose.

## Environment

| Variable | Purpose |
|---|---|
| `WATERMARKS_SERVER_VERSION` | The advertised version. Defaults to `dev` |
| `WATERMARKS_SERVER_API_KEY` | Bearer token. Unset means no auth, so bind loopback |
| `WATERMARKS_REWRITE_*` | Rewrite backend, model, base URL, API key, remote allowance |
| `PROSE_SANITISER_SCRIPTS_DIR` | Where the optional torch harnesses live. Absolute (or `~/`) paths only; a relative value is ignored, as is every `*_DIR` override |

API keys are read from the environment only. There is deliberately no flag for
one, because keys on argv are visible in `ps` and shell history.

## Honest scope

The service inherits every limit of the layers beneath it. It strips
container-level provenance losslessly and verifiably; it cannot detect or remove
a statistical sampling watermark, cannot touch a pixel-domain watermark, and
cannot know whether a durable Content Credential links a stripped asset back to
its manifest. A `200` from `/clean` means the container is clean, not that the
file is anonymous.

## Licence

**MIT OR Apache-2.0**, at your option.

Copyright (c) 2026 DreamLab AI Consulting Ltd and contributors. The full texts
are in [`LICENSE-MIT`](LICENSE-MIT) and [`LICENSE-APACHE`](LICENSE-APACHE)
beside this file, and both ship inside the published archive.

This crate was extracted from the agentbox `services/prose-sanitiser` tree,
where [agentbox ADR-2030](https://github.com/DreamLab-AI/agentbox/blob/main/docs/adr/ADR-2030-permissive-licensing-for-publishable-service-crates.md) settled the position: a service crate is
permissive per crate while the containing repository stays AGPL-3.0-only. That
is not a contradiction — the AGPL governs the aggregate hosted service, not the
licence of each part, and the grant travels with the crate. Since extraction
this repository is `MIT OR Apache-2.0` throughout, so the grant no longer sits
inside a copyleft aggregate at all.
