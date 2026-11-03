# Catalogue readback and transport lifetime during cancellation

Task [#4](https://github.com/taskqorg/assetq/issues/4), AssetQ R3-R8. Original continuation: "PM re-run at 1f0f2e7 not qualified: model/list response logged natively after consumer receives returned empty; cancel readback unknown at exit. Slot released. Details in the latest issue comment. Fix late-response delivery and cancel readback, then ask for a new slot."

## Fresh catalogue evidence

The initial [PM summary](https://github.com/taskqorg/assetq/issues/4#issuecomment-6603868199) describes missing consumer delivery. Fresh readback of that operation's retained stdout gives two `events` batches of sizes 21 and 0. The first contains the `id:4` response with `result.data` holding eight models and `nextCursor: null`. The private stdout file's SHA-256 is `2501bb54c6b11382925270e1d1b1a601694a3642afeb7a660610023b436db342`.

The native journal records the same allocated pending identity `[4,"model/list"]`, then response `id:4` and a cleared pending checkpoint. Those observations establish catalogue delivery in the first stdout batch; the later empty batch does not undo it. No second catalogue delivery change or native replay is justified by the summary. The corrected candidate still needs its own native run, and model enumeration does not qualify an image tool or artifact path.

## Cancellation failure and correction

Both native attempts retained `protected cancel readback unknown: exit`. In both AssetQ consumers, the `Run` was owned by a work closure and dropped before `ProtectedHost::cancel`. Dropping its transport closes frontend stdin; host cancellation then races native shutdown. PM later reconciled the original operation and released its slot. The generic exit diagnostic does not prove every host-side failure cause, but the premature close is a reproducible consumer defect.

The qualification command and prepared generation runner now share one lifetime guard. It retains the opened `Run`, passes a mutable borrow to work, performs the original host cancellation/readback, and only then drops the transport. Work failures still enter cleanup. An opening failure also attempts the same owned cleanup; failed opening/cancellation remains unknown. No second operation, launch or cancellation retry is created.

## Executable evidence

`image_worker_cancel_readback_precedes_native_eof` uses the existing invented Runtime protocol fixture. A real fixture process records EOF only when its input closes. Before correction, the focused check fails with `native EOF before cancel readback`. After correction, it passes success, work-error, cleanup-error and opening-error cases, retaining both work and cleanup errors. It verifies transport close after cleanup without credentials, image tooling or provider effects.

Command: `cargo test --locked --lib image_worker_cancel_readback_precedes_native_eof`. This is a consumer process check, not native Mac host cancellation qualification. No adapter anchor, runtime pin, account, model, grant, capacity, method version or accepted crate changes. Existing unknown-completion and protected-supervisor-exit traces remain required: no job lifecycle transition changed, and neither cancel acknowledgment nor transport exit confirms descendants or releases capacity.

`cargo test --locked --lib image_worker` passes all ten affected checks. The frozen implementation's `python3 scripts/check.py` passes on Mac in 26.01 seconds including setup: four checker checks, pinned contract lint/whitespace/formatting, 13 core, 26 isolated PostgreSQL service and two trace checks. The local rebuilt debug binary is SHA-256 `159c15541af677c5e0d7486274fc12614e6d5a5ae648434f3c41f5272bb5279e`. Record each PM build's actual source/worktree cleanliness and binary hash separately; a different build path does not permit borrowing this artifact's identity. Refreshed native and merge-result CI outcomes remain unknown.

## Remaining native qualification

Use a new original slot and the retained adapter through the existing operator-controlled recipe. Inspect the complete first catalogue batch, then record cancellation readback while the consumer is alive. Independently reconcile whole-operation completion before the original slot release. Worker host actions remain refused; there is no role bypass or native/model/paid turn in this continuation. Image-tool/output and durable runner-bound slot/completion acceptance remain open. Task #4 is incomplete.
