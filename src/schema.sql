-- AssetQ schema version 1. Applied once inside one transaction.
CREATE TABLE projects (
  id text PRIMARY KEY CHECK (id ~ '^[a-z0-9][a-z0-9-]{0,62}$'),
  budget_credits bigint NOT NULL DEFAULT 0 CHECK (budget_credits >= 0),
  max_active_jobs int NOT NULL DEFAULT 4 CHECK (max_active_jobs > 0),
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE tokens (
  hash text PRIMARY KEY,
  project text NOT NULL REFERENCES projects,
  role text NOT NULL CHECK (role IN ('member', 'owner')),
  created_at timestamptz NOT NULL DEFAULT now(),
  revoked_at timestamptz
);
CREATE TABLE blobs (
  sha256 text PRIMARY KEY CHECK (sha256 ~ '^[0-9a-f]{64}$'),
  size bigint NOT NULL CHECK (size >= 0),
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE methods (
  project text NOT NULL REFERENCES projects,
  id text NOT NULL,
  version int NOT NULL CHECK (version > 0),
  status text NOT NULL CHECK (status IN ('experimental', 'recommended', 'deprecated', 'disabled')),
  spec jsonb NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (project, id, version)
);
CREATE TABLE jobs (
  id text PRIMARY KEY,
  project text NOT NULL REFERENCES projects,
  idempotency_key text NOT NULL,
  request_hash text NOT NULL,
  request jsonb NOT NULL,
  method_id text NOT NULL,
  method_version int NOT NULL,
  phase text NOT NULL,
  state jsonb NOT NULL,
  fence bigint NOT NULL DEFAULT 0,
  lease_owner text,
  lease_until timestamptz,
  wake_at timestamptz NOT NULL DEFAULT now(),
  reserved_credits bigint NOT NULL,
  events int NOT NULL DEFAULT 0,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now(),
  UNIQUE (project, idempotency_key),
  UNIQUE (id, project),
  FOREIGN KEY (project, method_id, method_version) REFERENCES methods
);
CREATE INDEX jobs_runnable ON jobs (phase, wake_at);
CREATE TABLE job_events (
  job_id text NOT NULL REFERENCES jobs,
  seq int NOT NULL,
  event jsonb NOT NULL,
  at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (job_id, seq)
);
CREATE TABLE attempts (
  id text PRIMARY KEY,
  job_id text NOT NULL REFERENCES jobs,
  fence bigint NOT NULL,
  owner text NOT NULL,
  started_at timestamptz NOT NULL DEFAULT now(),
  ended_at timestamptz,
  UNIQUE (job_id, fence)
);
-- External effects: dispatch intent is persisted before the provider call.
CREATE TABLE effects (
  id text PRIMARY KEY,
  job_id text NOT NULL REFERENCES jobs,
  attempt_id text NOT NULL REFERENCES attempts,
  provider text NOT NULL,
  request jsonb NOT NULL,
  op_id text,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE objects (
  id text PRIMARY KEY,
  project text NOT NULL REFERENCES projects,
  sha256 text NOT NULL REFERENCES blobs,
  size bigint NOT NULL,
  content_type text NOT NULL,
  kind text NOT NULL CHECK (kind IN ('upload', 'output')),
  job_id text REFERENCES jobs,
  import_key text,
  provenance jsonb NOT NULL,
  synthetic boolean NOT NULL DEFAULT false,
  acceptance text NOT NULL DEFAULT 'pending' CHECK (acceptance IN ('pending', 'accepted', 'rejected')),
  retention_until timestamptz,
  created_at timestamptz NOT NULL DEFAULT now(),
  UNIQUE (job_id, import_key),
  FOREIGN KEY (job_id, project) REFERENCES jobs (id, project)
);
CREATE INDEX objects_project ON objects (project, created_at, id);
CREATE TABLE manifests (
  job_id text PRIMARY KEY REFERENCES jobs,
  items jsonb NOT NULL,
  committed_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE recommendations (
  id text PRIMARY KEY,
  project text NOT NULL REFERENCES projects,
  use_case text NOT NULL,
  constraints jsonb NOT NULL,
  method_id text NOT NULL,
  method_version int NOT NULL,
  evidence jsonb NOT NULL,
  observed_at timestamptz NOT NULL,
  state text NOT NULL CHECK (state IN ('proposed', 'promoted', 'withdrawn')),
  proposed_by text NOT NULL,
  promoted_by text,
  created_at timestamptz NOT NULL DEFAULT now(),
  FOREIGN KEY (project, method_id, method_version) REFERENCES methods
);
CREATE TABLE comparisons (
  id text PRIMARY KEY,
  use_case text NOT NULL,
  dataset_version text NOT NULL,
  evaluator_version text NOT NULL,
  method_versions jsonb NOT NULL,
  results jsonb NOT NULL,
  missing_cases jsonb NOT NULL,
  created_by text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);
-- Latest health/backup observations; absence reads as unknown.
CREATE TABLE observations (
  kind text NOT NULL,
  subject text NOT NULL,
  value jsonb NOT NULL,
  observed_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (kind, subject)
);
