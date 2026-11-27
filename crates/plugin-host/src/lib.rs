// SPDX-License-Identifier: AGPL-3.0-or-later

//! Plugin host services and capability enforcement for secure-notify plugins: the shared HTTP
//! client, restricted per plugin to its declared hosts (docs/architecture/plugins.md#source-context).
//! The WASM component host joins this crate after the MVP.

mod http;
mod ssrf;

pub use http::{HttpSettings, ScopedHttp, SharedHttpClient};
