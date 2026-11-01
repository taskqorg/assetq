# Implementation and qualification order

The owner request is preserved in [intake](intake/2026-11-01-owner-request.md). This is a rollout plan and evidence ledger, not a claim that implementation has shipped. Task receipts will be linked here after board creation.

## Runtime workstream

1. AgentQ: extract the shared runtime with its tests and a thin non-Rust entry point; establish optional optimization semantics and publish an immutable pin. Preserve the current CoreQ release during migration.
2. TaskQ: finish or coordinate existing #2094 native-proof work; adopt AgentQ only after its pin exists and this runtime surface has one writer. Preserve worker/reviewer/operator paths, recovery, native UI, usage and capacity.
3. PilotQ: adopt the same AgentQ pin for Codex and Claude manager paths; preserve dynamic tools, private drafts, catalogue, conversation identity, delivery and slot accounting. Use Codex on pds18 and qualify Claude on the Mac.
4. CoreQ: retire the old runtime only after the consumer inventory is complete and both consumers' adoption and native qualification are accepted. Keep infrastructure primitives and rollback-compatible old releases.

## Asset workstream

1. Build the durable AssetQ request/job/effect model, method registry, immutable object API and first service/CLI, with recovery/fencing/idempotency tests. An initial filesystem object store avoids introducing another storage service.
2. Prepare pds18 deployment with a dedicated identity, PostgreSQL role/database and data root; verify capacity, streaming upload/download, additive activation, rollback and an off-host backup restore. Host changes belong to the private host repository and concrete deployment plan.
3. Implement and qualify the Codex image-worker route through AgentQ. A real image-generation capability is required; never substitute a procedural drawing or silently use a direct paid API. Expose a precise capability gap when the selected runtime lacks the tool.
4. Implement image-to-3D with the csgo Meshy prop baseline, explicit parameters, provider reconciliation and headless Blender validation. Consume the first image, retain source GLB/textures and render views.
5. Run the connected real journey on the managed store, record generated image/3D asset IDs, methods, actual costs, recovery and download verification, and present outputs for owner visual acceptance.
6. Expose shared controls and comparison/recommendation records to humans and agents, with DashQ/ServerQ integration as dependencies permit. Seed the two methods without calling either universally best.

## Spend and operational boundaries

Meshy authorization: at most 1000 credits total for the initial AssetQ example/qualification work, with no purchase or top-up. This is a cumulative ceiling, not an instruction to spend it. Reconcile balance and all prior authorized AssetQ attempts before each paid submission; preserve a durable ledger and stop on unknown spend or an insufficient remaining allowance.

No other paid-provider budget is implied. Native subscription image generation uses the selected qualified worker and existing account; any paid API fallback needs a separate decision. Do not change worker models, effort, permissions or ordinary host capacity.

Prepare changes on isolated task worktrees. Cross-repository dependencies use board-supported references and are read back before start. New tasks are held only during intake, then start with dependencies. The PM resolves routine technical choices; product/visual/authority changes go to the owner.

## Monitoring and closure

The PM follows this task set through a persistent chat heartbeat, reconciles actual board states, handles technical blockers within the accepted architecture, and preserves owner questions. A task is not complete because a worker exited or opened a PR; require its declared merge and qualification receipts.

Keep existing unrelated TaskQ/PilotQ attempts intact. Monitor only this workstream plus explicit blocking dependencies. Stop the monitor when every required outcome is accepted, or when the owner cancels it; preserve remaining gaps and any unfinished jobs.
