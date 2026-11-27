// SPDX-License-Identifier: MIT OR Apache-2.0

//! Canonical types of secure-notify, from which the JSON Schemas in `schemas/` are generated
//! (docs/architecture/data-model.md#canonical-schema).
//!
//! The normalized event is defined once, here. WASM plugins, declarative sources, and the
//! canonical feed exchange events as JSON validated against the generated schema.

mod content;
mod event;
mod manifest;
mod purl;
mod validate;

pub use content::{Mappings, TaxonomyEntity, load_taxonomy, normalize_text};
pub use event::{
    Affected, Breach, Component, Cvss, DataClass, Entities, Epss, EventKind, Identifiers, Incident,
    IncidentImpact, IncidentStatus, KindFamily, NormalizedEvent, Range, RangeEvent, RangeType,
    SCHEMA_VERSION, Severity, SeverityLevel,
};
pub use manifest::{Capabilities, PluginKind, PluginManifest, PluginRuntime, load_manifest};
pub use purl::{npm_purl, validate_purl};
pub use validate::{FieldError, ValidationError, parse_event};

/// Generated JSON Schemas: file name in `schemas/` and schema document.
pub fn json_schemas() -> Vec<(&'static str, serde_json::Value)> {
    vec![
        (
            "normalized-event.schema.json",
            serde_json::to_value(schemars::schema_for!(NormalizedEvent)).unwrap_or_default(),
        ),
        (
            "plugin-manifest.schema.json",
            serde_json::to_value(schemars::schema_for!(PluginManifest)).unwrap_or_default(),
        ),
        (
            "taxonomy-entity.schema.json",
            serde_json::to_value(schemars::schema_for!(TaxonomyEntity)).unwrap_or_default(),
        ),
    ]
}
