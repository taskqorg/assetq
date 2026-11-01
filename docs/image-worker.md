# Codex image worker

This path implements AssetQ R2-R8 and task [#4](https://github.com/taskqorg/assetq/issues/4), preserving the [owner request](intake/2026-11-01-owner-request.md). New builds use the published `agentq-runtime` 0.1.0 source pin `aab1a190ab76f1e932578520d7a8787ef1d4b660`, not CoreQ's frozen agent module. The original crate and version 2 receipts retain `513193c8a96f3de1c71a86291a679230817ce0b2`. Technical qualification never supplies owner visual acceptance.

## Authority and inputs

New project registries seed experimental `image.worker.v1@3` for the protected pipeline and retain the unchanged version 2 specification. Version 1 stays disabled and immutable. `ASSETQ_IMAGE_WORKER_ROOT` names an operator-selected workspace for observing retained operations; omitting it leaves the provider unavailable. New generation launches refuse with `capability_unavailable` until composed Mac consumer and current image-tool qualification satisfy R3 and [permission authority](https://github.com/taskqorg/specq/blob/537db47c9f01c17d8cf6259a3698d8bd15435d16/engineering/permission-authority.md). There is no environment switch or request override for this refusal.

Requests cannot choose executable, account, model, permissions, environment, output paths or optimization. The prepared native app-server path uses inherited settings, with AgentQ optimization intent `inherit`; absent optimization remains a plain native worker. Its tool/output path has one bounded owner-authorized qualification; reusable project dispatch remains incomplete.

AgentQ #186/PR188 supplies the protected app-server API and preserves ordinary consumers. The [original blocker](blockers/4-protected-launch.md) is historical diagnosis; its upstream implementation is published. Current composed AssetQ qualification remains required, using TaskQ's existing operator authority and the PM-qualified staged Mac adapter digest. No new Windows grant or AssetQ policy store is introduced.

The worker resolves attachment IDs through the job's project, verifies store hashes, and accepts PNG image references only. It passes these verified immutable files as native local-image inputs. Parameters are bounded `max_width` and `max_height` (64..4096, default 4096); one original PNG of at most 16 million pixels and the configured upload-byte limit is supported. Preview PNGs fit 512 by 512. Unsupported input or absent native image generation is an explicit `capability_unavailable` refusal; no paid API or Claude fallback is selected.

## Durable operation

Use the existing executable lifecycle in `src/model.rs`: dispatch intent precedes the provider call, an acknowledged operation releases the executor lease, and completion precedes import/validation/manifest commit. A server-owned spool under the store's counted temporary directory binds the effect UUID to one runner. The detached `assetq image-turn <effect>` runner uses AgentQ's native app-server transport and a private, bounded journal. A create-once launch marker prevents duplicate runners. An incomplete launch receipt remains unknown. A sealed absence prevents a delayed old submission, as for the existing provider contract.

The runner journals each outbound request before sending it and each inbound native event before interpreting it. It never automatically approves native server requests. It preserves native thread/turn identity, image-tool item ID and artifact path, including completion events arriving before the turn-start acknowledgment. Only a matching successful terminal turn plus a successful native `imageGeneration` item can produce an image result. Original bytes must come from the selected native home's `generated_images` directory, pass full PNG decoding and the requested limits, and remain byte-for-byte unchanged. Evidence and preview are separate immutable outputs. Image-tool availability without such a receipt is unqualified.

The runner survives a control-service restart. Reconciliation reads the same effect spool and never launches a second turn. A runner interruption or missing terminal record remains unknown with a reason; a native operation is never replayed to obtain a replacement. Cancellation writes a durable request and sends one native interrupt for the accepted thread/turn. Confirmation requires native terminal interruption and runner-owned process-exit readback. Lost confirmation stays uncertain, retaining active-job reservations. A late completed image after cancellation is not imported by the lifecycle model.

One turn has a maximum 600-second generation interval, followed by at most 30 seconds observing interruption. Missing terminal evidence remains unknown. The protocol journal is bounded to 32 MiB; raw diagnostic stderr goes to the platform null device so it cannot exhaust storage. Original output and attachment bytes follow the configured upload limit and store admission. Temporary files remain charged and retained for recovery. A full store stops new dispatch/import; it does not delete existing assets.

This route requires a native ChatGPT subscription account and the inherited OpenAI/AgentQ provider; API-key authentication or another provider is refused before generation. `max_credits: 0` is a no-paid-API ceiling, not a claim of zero subscription usage. Native token usage is retained, and image-specific subscription usage remains unknown when unreported.

The registry's `native-inherited` model selector preserves ordinary native settings; the actual worker model is recorded by the native turn report. The image tool does not report a separate backend model ID in its receipt, so that backend identity remains unknown. This is not authority to select or replace either model.

## Result and qualification

The manifest retains original PNG, preview PNG and native evidence JSON. Object provenance records role, dimensions, source hash, method/runtime revision, native thread/turn/tool identity, verified attachment identities and reported token usage. Subscription charge/credits remain explicitly unknown when native usage supplies no charge. Owner acceptance starts `pending`.

Deterministic tests cover effect deduplication, lost receipts/interruption, absent optimizer, capability refusal, bad/foreign attachments, forged artifact paths, PNG decoding and immutable retrieval. Existing model traces retain fencing, cancellation and no-paid-replay protection. The native qualification record separately identifies the actual Codex executable/version, exact candidate, tool event and original bitmap hash. The first real brief is an original stylized wooden crate with blue metal corners, no text or third-party references; retain its original object ID as the image-to-3D input.

## Configuration and operation

Set `ASSETQ_IMAGE_WORKER_ROOT` in the executor's protected environment to an existing absolute native worker workspace, alongside the existing database/data-root configuration, to observe retained operations. It never enables a new native launch. Before enabling dispatch, the supported host interface must resolve and read back the exact project/profile target, protected grant, approver/revision and effective read/write/network/child-process boundaries. Inherited mode names and prompt restrictions are insufficient. Do not edit grants or native permissions to bypass the refusal.

New project initialization seeds version 3 alongside unchanged version 2; an existing project retains its immutable registry. Its owner may register the spec returned by `image_worker::method` through the existing versioned method API. No installed service or ordinary native configuration is edited by this build.

After qualification and dispatch integration, use the existing CLI with `--method image.worker.v1@3`. Do not resubmit the accepted crate: its version 2 operation and original object ID remain the image-to-3D input. Use `status`, `events`, `cancel` and `result` on the retained job ID. The original is manifest `output-0`, the preview `output-1`, and native evidence `output-2`. The API's authenticated content download verifies the immutable original.

The prepared runner uses the same binary's internal `image-turn` command; that entry point also refuses new launches pending host authority integration. Its create-once marker independently prevents replay. Preserve the spool during recovery. A missing/unreadable process identity or native terminal receipt remains unknown, and manual replacement generation requires resolving the original operation first.

The actual Windows qualification and current limitations are recorded with the [crate receipt](evidence/4/README.md). Windows rename durability uses the platform write-through primitive; the qualification proves native writes/readback/restart, not a hardware power-loss experiment.

The [protected consumer recipe](evidence/4/protected-consumer.md) supplies one bounded `image-qualify` command for PM, using the image runner's actual protected launch and native exchange functions. `image-observe` reconciles its original host operation without relaunch or slot release. This prepares qualification; it does not enable generation, automate slot acquisition for durable jobs or prove current image-tool availability.
