# Runtime and asset architecture

The owner-approved target separates agent execution, optional optimization, product scheduling and asset jobs. This document describes required implementation; it does not claim consumer adoption or a deployed service.

## Ownership and dependencies

| Owner | Responsibility |
|---|---|
| CoreQ | Installation, releases, service units, generic process identity/execution, telemetry storage and transport primitives |
| AgentQ runtime | Codex/Claude adapters, start/resume/cancel/observe, native session identity, account selection, usage/limits and explicit optional optimization integration |
| AgentQ optimization | Existing tool-output compression, original retrieval, engine qualification and integration controls |
| TaskQ | Board truth, task/attempt/reviewer lifecycle, admission/capacity, workspaces and publication |
| PilotQ | Conversation input, PM instructions, user/route grants, tools, delivery and its durable journal |
| AssetQ | Asset request/job/effect lifecycle, method registry, executors, validation, comparisons, asset records and the management site/API at `assets.ufobe.com` |
| pds18 storage | AssetQ-owned persistent bytes, database, backup and restoration |

Dependency direction: TaskQ/PilotQ/other projects -> `agentq-runtime` -> CoreQ. AssetQ uses AgentQ only for steps requiring an agent. Direct Meshy or Blender steps do not start an LLM. CoreQ never depends upward on AgentQ.

## Agent extraction scope

Move the existing CoreQ `agent` implementation and its relevant fake-client/contract tests, including `src/agent.rs`, native/background/app compatibility, app-server transport, session/turn reports, argument/environment adaptation and account selection into a public Rust library crate inside the existing agentq repository.

Expose a thin JSON-capable CLI for non-Rust consumers without replacing the existing `agentq run` command, which wraps native commands for output optimization. Reuse the runtime contract across CLI and library. Installing the optimizer service, Headroom, RTK or an MCP server is not a prerequisite for a plain agent turn.

Preserve native compaction, auth/account, permissions, model/effort, Remote Control, Codex app and app-server, dynamic tools, private draft events, model catalogue, original submission recovery and one attributable usage observation per turn. Caller-owned grants, storage, process ownership and UI remain with the caller.

Optimization has explicit inherit/off/on intent plus observed effective state and reason. Existing host disable and privacy restrictions prevail. A launch cannot toggle host/global configuration, mutate previous replay bytes or silently reroute provider/model. Optimizer failure returns original tool output; runtime failure remains a runtime failure.

## Compatible adoption and retirement

Publish an immutable AgentQ runtime revision with deterministic contract tests first. Keep the existing released CoreQ API compatible and frozen during adoption; no upward dependency or circular re-export. Inventory actual consumers, including in-flight TaskQ integration work, before scheduling replacements.

TaskQ and PilotQ update their own contracts before adopting the pin. Preserve capabilities and qualify their real native paths. The existing TaskQ #2094 retains its current native-proof scope; do not create a simultaneous writer on its runtime surface.

Only after all inventoried consumers adopt and required qualification passes may a separate CoreQ retirement remove the old agent module and tests. A rollback selects the previous immutable consumer build; it preserves native sessions, persisted execution boundaries and active work without paid replay or credential changes.

## Asset job execution

One initial service owns PostgreSQL metadata and an immutable local object store. A job resolves a method version once, checkpoints step/effect state, releases agent capacity during provider waits and imports final outputs before completion. Executors claim fenced work and can run on another qualified host.

The first service needs a versioned method registry, bounded step sequences and an executable state model. It does not need a visual workflow editor, a new agent framework or a separate scheduler per project. TaskQ tracks the implementation work; it does not become AssetQ's asset database.

## Management site and public entry

`https://assets.ufobe.com` is the canonical AssetQ web application and API origin. AssetQ owns asset previews/search/detail, generation controls, method/pipeline management and benchmark comparisons. The browser uses the same project-scoped service and versioned registry as agents and CLI.

DashQ remains the fleet/tool overview and links to the canonical AssetQ workspace with source-backed summary metadata. Reuse suitable shared components without implementing a second asset editor or benchmark workspace inside DashQ.

The public HTTPS entry is the existing Caddy service on pds18; the application and storage remain loopback/private. Configure only the named DNS record/site after current-state verification. Browser access uses the qualified existing guard; machine API routes enforce their own explicit scoped authentication. Publication never makes private assets anonymous.

## Rollout evidence

Evidence is separate for library contracts, each consumer's native runtime, provider generation, storage integrity, service activation and backup restoration. The two first real journeys are defined in [acceptance](acceptance.md). A green fake-provider suite cannot substitute for their real outputs.
