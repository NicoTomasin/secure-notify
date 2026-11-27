-- SPDX-License-Identifier: AGPL-3.0-or-later
-- Correlation details (docs/architecture/deduplication.md, docs/architecture/taxonomy.md#tagging).

-- Tags added by the core are recorded separately from tags set by the source.
ALTER TABLE story_entities ADD COLUMN origin TEXT NOT NULL DEFAULT 'source'
    CHECK (origin IN ('source', 'core'));

-- Priority of the event's source for the story title: structured databases (2) before feeds (1)
-- before news (0) (docs/architecture/data-model.md#story).
ALTER TABLE events ADD COLUMN title_priority INTEGER NOT NULL DEFAULT 1;
