-- Additive schema-1 extension; existing job/method/provenance rows stay intact.
CREATE TABLE IF NOT EXISTS benchmark_revisions (
  project text NOT NULL REFERENCES projects,
  kind text NOT NULL CHECK (kind IN ('dataset', 'evaluator')),
  id text NOT NULL,
  version int NOT NULL CHECK (version > 0),
  document jsonb NOT NULL,
  sha256 text NOT NULL,
  PRIMARY KEY (project, kind, id, version)
);
CREATE TABLE IF NOT EXISTS benchmark_plans (
  comparison text PRIMARY KEY REFERENCES comparisons,
  project text NOT NULL REFERENCES projects,
  plan jsonb NOT NULL
);
CREATE TABLE IF NOT EXISTS benchmark_scores (
  comparison text NOT NULL REFERENCES benchmark_plans,
  sample text NOT NULL,
  judgment jsonb NOT NULL,
  observed_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (comparison, sample)
);
-- Read lane grants: web reads only, never generation. Published to the lane as a snapshot (pipe.md § Read lane).
CREATE TABLE IF NOT EXISTS read_grants (
  hash text PRIMARY KEY CHECK (hash ~ '^[0-9a-f]{64}$'),
  project text NOT NULL CHECK (project ~ '^[A-Za-z0-9._:-]+(/[A-Za-z0-9._:-]+)+$' AND length(project) <= 128),
  created_at timestamptz NOT NULL DEFAULT now(),
  revoked_at timestamptz
);
