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
