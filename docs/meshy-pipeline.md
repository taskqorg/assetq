# Bounded Meshy prop pipeline

Task [#5](https://github.com/taskqorg/assetq/issues/5), acceptance A1-A7, applies Pipe R2-R8. The original request remains in [intake](intake/2026-11-01-owner-request.md). This specification authorizes no credential, host-grant, purchase, deployment or configuration change.

## Immutable input and recipe

The first source is the unchanged owner-accepted crate, asset `2c887345-9f66-491f-8a0c-81d81cf0c4af`, SHA-256 `fe2077d15b0c50617b84c6267277d46339c6a8cedbd6334e923cfa5475529baa`. Resolve its scoped object record, verify its bytes and PNG decoding, and retain its original identity when importing a copy into an isolated store. A copy gets a new local object ID; never impersonate the original record.

Use `POST /openapi/v1/image-to-3d` with `model_type: smart-topology`, `ai_model: meshy-t2`, `topology: triangle`, `target_polycount: 3000`, `should_texture: true`, `enable_pbr: true`, `texture_resolution: 2k`, and `target_formats: [glb]`. Submit the verified PNG as a data URI; no public input URL is required. There is no remesh experiment or `latest` fallback. Record the resolved texture model separately if the provider reports it; an unreported texture model stays unknown.

Fresh official verification on 2026-11-04: [request and task schema](https://docs.meshy.ai/en/api/image-to-3d), [pricing](https://docs.meshy.ai/en/api/pricing), [balance](https://docs.meshy.ai/en/api/balance). The documented price is 15 credits for meshy-t2 with 2K textures. The target is approximate, not a guaranteed exact triangle count. `consumed_credits` reports task usage. These public observations do not prove account access, balance, prior spend or live provider behavior.

## Dispatch and recovery

Use the existing job/effect/fence model. A local operator binds one already accepted job before enabling its executor; API fields cannot enable paid dispatch. General Meshy dispatch stays unavailable. The binding names the immutable source and an independently reconciled workstream ledger on the qualified executor. It must establish known prior credits, account balance and no unresolved prior attempts.

Initial testing has no cumulative credit ceiling; record actual and cumulative costs, retain per-job limits and prohibit purchases/top-ups. Preflight and ledger receipts identify `spend_policy: uncapped_initial_testing`; a null cap means no cumulative ceiling under that policy. Missing qualification, credentials, current verification, balance or ledger refuses effects. Never initialize unknown prior spend as zero.

The first POST requires a durable create-once intent and the shared authoritative store lock, with independently verified cross-host exclusivity. Accept HTTP 200, 201 or 202 only with a parsed original provider ID retained before acknowledging the service. Restart adopts that receipt and polls the same task. The official API provides no documented caller idempotency key or sealed-absence barrier: if an intent has no retained original provider ID, stay uncertain and stop paid work. A list result or matching timestamp is not an automatic identity proof. No automatic POST retry or replacement is permitted.

Cancellation records the local request and returns unconfirmed while the remote task is outstanding. Do not call DELETE: it permanently deletes a task, refuses in-progress work and can lose results without refund. Continue polling the original operation, retain any observed charge, and keep capacity/spend reserved until terminal evidence. Missing usage remains unknown even after local cancellation.

## Retention and validation

Retain original GLB, all returned texture maps, private original provider receipts and sanitized public evidence. Signed download locators and credentials never enter public manifests or board receipts. Fetch only HTTPS provider asset URLs without forwarding API credentials. Stream into the counted immutable store with per-chunk capacity checks, bounded response/file counts and existing physical metadata guards. Before the paid effect, charge a 640 MiB output/temporary reservation and require twice that available room; retain it while submission is uncertain and release only its placeholder after terminal evidence. Initial qualification caps each imported file at 64 MiB.

Read back the existing inherited 512 MiB through 2 GiB native memory boundary before dispatch and before Blender; refuse silent Windows child breakaway. No host limit or grant is installed. Before successful publication, validate the GLB container and embedded resource bounds and import it in factory-startup headless Blender. Measure triangles, connected components, bounding dimensions, UVs, materials and texture sizes; report the actual result against the 3000-triangle target and 2048-by-2048 texture recipe. Render bounded textured views without changing the source GLB. Retain Blender version, script hash, reports and rendered hashes. File validity and renders are technical evidence; final likeness and visual acceptance belong to the owner.

The Linux operator allocation uses two allowed CPUs and a 2048 KiB inherited stack; the child receives fixed `MALLOC_ARENA_MAX=2` and `OIDN_NUM_THREADS=2`. Qualify those native bounds with the exact Blender artifact. Explicit temporary quota and launch/readback instructions are in the [operator procedure](https://github.com/taskqorg/assetq/blob/7388734a40502323fb34932471ed4b8f7a0d089b/docs/evidence/5/operator.md).

Completed provider work with invalid output fails precisely and never resubmits. Retain downloaded originals outside a success manifest on failure. Storage exhaustion pauses safe import and preserves prior objects and provider identity. Restart resumes downloads/validation without another generation. Missing native Blender or provider evidence remains unqualified.

## Acceptance evidence

The focused fake-provider chain must exercise duplicate admission, accepted HTTP 202 ID persistence, lost POST reply with no repeat, receipt recovery, restart polling, unknown usage/balance refusal, admission beyond the former cumulative ceiling, cancellation without DELETE, partial import and validation refusal. The canonical R8 gate includes those deterministic checks. The bounded live run additionally needs a qualified executor, reconciled ledger and balance before spend, original crate source/result manifests, authenticated download hashes, native headless measurements and textured views. The managed-store journey is separately coordinated with #6; local evidence never qualifies pds18.
