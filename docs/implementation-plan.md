# Implementation and qualification order

The owner request is preserved in [intake](intake/2026-11-01-owner-request.md). This is a rollout plan and evidence ledger, not a claim that implementation has shipped. Current execution states come from the linked TaskQ cards.

## Tracked tasks

| Outcome | Task | Blocking inputs |
|---|---|---|
| Shared runtime with optional optimization | [agentq#182](https://github.com/taskqorg/agentq/issues/182) | Existing CoreQ v0.18 compatibility baseline |
| TaskQ adoption | [taskq#2111](https://github.com/taskqorg/taskq/issues/2111) | AgentQ runtime; existing [taskq#2094](https://github.com/taskqorg/taskq/issues/2094) native proof |
| PilotQ adoption | [pilotq#294](https://github.com/taskqorg/pilotq/issues/294) | AgentQ runtime; existing [pilotq#281](https://github.com/taskqorg/pilotq/issues/281) qualification |
| CoreQ runtime retirement | [coreq#112](https://github.com/taskqorg/coreq/issues/112) | Runtime and both verified consumer adoptions |
| Durable service, registry and object API | [assetq#3](https://github.com/taskqorg/assetq/issues/3) | Bootstrap contract and CI |
| Real image-worker method | [assetq#4](https://github.com/taskqorg/assetq/issues/4) | Foundation, published [agentq#186](https://github.com/taskqorg/agentq/issues/186) protected runtime and composed native qualification |
| Image-to-3D prop method | [assetq#5](https://github.com/taskqorg/assetq/issues/5) | Image-worker method |
| Managed-store connected live journeys | [assetq#6](https://github.com/taskqorg/assetq/issues/6) | Both methods and reviewed private host activation receipts |
| Method comparisons and guidance | [assetq#7](https://github.com/taskqorg/assetq/issues/7) | Real generation and storage receipts |
| Management website and API origin | [assetq#9](https://github.com/taskqorg/assetq/issues/9) | Versioned controls and benchmark evidence |
| Fleet summary and links to the AssetQ site | [dashq#246](https://github.com/taskqorg/dashq/issues/246) | Stable AssetQ metadata and canonical destinations |

Storage preparation has a separate task in the private host repository. Its identity, infrastructure inventory and operator effects stay in restricted PM evidence. GitHub and GitLab dependencies do not cross providers; the PM creates the concrete activation task once the candidate and reviewed effects exist.

## Bootstrap verification

The initial specification commit is `1c21af2ec2522e26c7b74ed57acf289ae970b3c2`; its [GitHub check](https://github.com/taskqorg/assetq/actions/runs/41290652268) passed. The canonical local gate also passed. These receipts qualify the bootstrap documents/checker, not generation or storage implementation.

The public repository requires the `tests` check and a squash merge queue on main, blocks force-push and deletion, and has no bypass actors. Workflow defaults are read-only; native secret scanning/push protection, vulnerability alerts and security updates are enabled. TaskQ registration, board read/write and label catalog were verified; Focus is all. Incident disclosure remains off.

The PM created a ten-minute heartbeat in the existing owner chat. It follows this task set and explicit blockers, resolves technical questions within the approved scope and escalates owner decisions. It preserves the cumulative Meshy ledger and separates code, native, provider, storage and visual acceptance.

## Runtime workstream

1. AgentQ: extract the shared runtime with its tests and a thin non-Rust entry point; establish optional optimization semantics and publish an immutable pin. Preserve the current CoreQ release during migration.
2. TaskQ: finish or coordinate existing #2094 native-proof work; adopt AgentQ only after its pin exists and this runtime surface has one writer. Preserve worker/reviewer/operator paths, recovery, native UI, usage and capacity.
3. PilotQ: adopt the same AgentQ pin for Codex and Claude manager paths; preserve dynamic tools, private drafts, catalogue, conversation identity, delivery and slot accounting. Use Codex on pds18 and qualify Claude on the Mac.
4. CoreQ: retire the old runtime only after the consumer inventory is complete and both consumers' adoption and native qualification are accepted. Keep infrastructure primitives and rollback-compatible old releases.

## Asset workstream

1. Build the durable AssetQ request/job/effect model, method registry, immutable object API and first service/CLI, with recovery/fencing/idempotency tests. An initial filesystem object store avoids introducing another storage service.
2. Prepare pds18 deployment with a dedicated identity, PostgreSQL role/database and data root; verify capacity, streaming upload/download, additive activation and rollback. Defer new backup work and local/external backup-restore qualification without blocking initial functionality. Preserve existing backups and keep unproved backup/restore states unknown; no backup-driven stop/restart, cold-backup timer, rotation or new platform. Host changes belong to the private host repository and concrete deployment plan.
3. Implement and qualify the Codex image-worker route through AgentQ. Consume the immutable protected runtime and provide PM one [composed, non-generating consumer](evidence/4/protected-consumer.md) under the existing Mac grant. Qualify current authority/tool/output paths before enabling durable slot-backed dispatch. Preserve the accepted crate and operation without generation/replay. A real image-generation capability is required; never substitute a procedural drawing or silently use a direct paid API. Expose a precise capability gap when the selected runtime lacks the tool.
4. Implement image-to-3D with the csgo Meshy prop baseline, explicit parameters, provider reconciliation and headless Blender validation. Consume the first image, retain source GLB/textures and render views.
5. Run the connected real journey on the managed store, record generated image/3D asset IDs, methods, actual costs, recovery and download verification, and present outputs for owner visual acceptance.
6. Expose shared controls and comparison/recommendation records to humans and agents. Seed the two methods without calling either universally best.
7. Build the canonical management site at `assets.ufobe.com` through the shared design workflow: asset browsing/previews/search/detail, generation jobs, methods and pipeline benchmarks, using the same scoped AssetQ API. Include owner visual review and real browser/API acceptance.
8. Publish this approved name through existing pds18 Caddy HTTPS and a loopback-only application after exact DNS/site ownership, authentication and rollback checks. Change only the named record/site, preserving apex/wildcard and other services. Keep private host installation evidence restricted.
9. Integrate source-backed summary metadata and deep links into DashQ, preserving its existing views and ServerQ's read-only host boundary. The complete asset/benchmark workspace belongs in AssetQ.

## Spend and operational boundaries

Meshy authorization: at most 1000 credits total for the initial AssetQ example/qualification work, with no purchase or top-up. This is a cumulative ceiling, not an instruction to spend it. Reconcile balance and all prior authorized AssetQ attempts before each paid submission; preserve a durable ledger and stop on unknown spend or an insufficient remaining allowance.

No other paid-provider budget is implied. Native subscription image generation uses the selected qualified worker and existing account; any paid API fallback needs a separate decision. Do not change worker models, effort, permissions or ordinary host capacity.

Prepare changes on isolated task worktrees. Cross-repository dependencies use board-supported references and are read back before start. New tasks are held only during intake, then start with dependencies. The PM resolves routine technical choices; product/visual/authority changes go to the owner.

## Monitoring and closure

The PM follows this task set through a persistent chat heartbeat, reconciles actual board states, handles technical blockers within the accepted architecture, and preserves owner questions. A task is not complete because a worker exited or opened a PR; require its declared merge and qualification receipts.

Keep existing unrelated TaskQ/PilotQ attempts intact. Monitor only this workstream plus explicit blocking dependencies. Stop the monitor when every required outcome is accepted, or when the owner cancels it; preserve remaining gaps and any unfinished jobs.
