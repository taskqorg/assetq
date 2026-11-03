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

## Native readback at 7d22481

[PM note 6604172803](https://github.com/taskqorg/assetq/issues/4#issuecomment-6604172803) reports exact source `7d22481009b6bd59a983e4985590d60302cd1b81` with the retained adapter. The steering is preserved verbatim: "Done: 7d22481 PM run passed initialize/verify/model-list (id:4, 8 models)/cancel readback before exit; reconcile completed before slot release; image tool unknown (no model turn); earlier 1f0f2e7 no-delivery verdict was wrong (PM log truncation). https://github.com/taskqorg/assetq/issues/4#issuecomment-6604172803"

Fresh private stdout readback has SHA-256 `b49245ee9f4256377c613ac38691f9c41ea3316308f90c6f1b86c17ca247325f`: `initialized`, `verified-effective`, typed `Subscription`, catalogue batches of 21 and 0 events, eight models in response `id:4`, then `cancel-requested`. `cancel.json` has no readback error. Its host receipt still says `state: unknown`: termination was requested, not proven complete. PM observed consumer exit 0, then original-operation reconcile `completed` before sending and observing slot release `released`; the retained slot journal confirms that terminal state. This proves the requested cancellation readback ordering, not completion from cancellation alone.

PM also confirms the earlier `1f0f2e7` missing-delivery verdict was a truncated-log interpretation. The corrected catalogue wait and cancel ordering now have their separate deterministic and native observations. No model turn or image call was submitted, and no grant/account/model change or accepted-crate replay occurred. Image capability remains unknown. Do not repeat this healthy non-generating qualification to obtain image evidence.

## Remaining image qualification

Worker host actions remain refused; there is no role bypass or native/model/paid turn in this continuation. Current image-tool/output and durable runner-bound slot/completion acceptance remain open. The [remaining acceptance and proposed bounded job](image-qualification-next.md) require an owner decision before extending the explicitly non-generating qualification. Task #4 is incomplete.
