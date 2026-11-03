# One protected durable image qualification job

Task [#4](https://github.com/taskqorg/assetq/issues/4), A1-A6; AssetQ R1-R8. The [owner decision and exact limits](image-qualification-next.md#approved-bounded-job) permit one distinct blue-wedge job. This candidate implements its slot/preparation/completion path; a native image result is still unobserved. General dispatch remains refused. Preserve the accepted crate, asset `2c887345-9f66-491f-8a0c-81d81cf0c4af`, unchanged for 3D.

## Operator recipe

Use the existing operator channel on the qualified Mac. TaskQ explicitly refuses external slot and app-server control when `TASKQ_ATTEMPT` is present. The driver refuses in that context before creating files. Never remove role variables, impersonate an operator or alter grants to run it. The bounded subscription turn is already authorized; this handoff requests execution under existing authority, not another spending decision.

Use the exact reviewed AssetQ binary and retained signed TaskQ adapter with SHA-256 `bd6dc0de5024ff7b6b0ae0a987f5467aa0c5196b005c7cdfd434d043709c7e42`. Its known transport proof is [separate](cancel-readback.md#native-readback-at-7d22481). Native image-tool/output and this automatic lifecycle remain unqualified. A mutable installed replacement, old adapter fallback or receipt-selected digest is refused.

From this candidate worktree, substitute private canonical paths in the following command. The workspace must be the existing registered AssetQ root; the new directory must be inside that workspace, preferably under this worktree's ignored `.taskq` directory. PostgreSQL server tools must already be available. No managed service, website or native configuration is changed.

```sh
python3 scripts/qualify_image.py \
  --binary '<exact candidate assetq binary>' \
  --adapter '<retained pinned TaskQ adapter>' \
  --workspace '<existing registered AssetQ root>' \
  --directory '<new private qualification directory>'
```

The driver creates a private, isolated PostgreSQL cluster and store, with no PostgreSQL TCP listener. It preserves fsync, the existing free-space floor and metadata guards. It uploads the invented blue swatch, verifies its hash, accepts the exact bounded request through the authenticated API with executors initially stopped, and records the duplicate request before effects. Tokens stay in memory and private process environments; never publish them.

`assetq image-serve <accepted-job-id> <pinned-adapter>` is the explicit operator-only path. It binds one durable job and one effect with create-once selectors. It accepts only the exact wedge brief, one attachment, `image.worker.v1@5`, 1024 by 1024 limits and zero paid-API credits. API fields and environment strings cannot enable it. A competing job/effect, corrupt selector or duplicate runner cannot replace it. Ordinary `serve`, `work` and unscoped `image-turn` keep their new-dispatch refusal.

The detached runner reserves a slot for its actual PID/start/boot, persists the original key/request, and observes that same slot until fresh reservation. AgentQ `prepare` independently resolves host authority before launch; its checkpoint is persisted before `launch_prepared`. No expected identity is copied from a launch observation. Runtime verifies subscription/provider before one input and inherits model, effort and optional optimization.

The driver kills only its isolated control process after input intent and starts the same job-bound control service. The original runner continues. It verifies the duplicate job ID, unchanged provider operation and one submission. Cancellation remains a separate deterministic check: interrupting this real turn solely to test cancellation would preclude its required bitmap. At most 600 seconds generation plus 30 seconds interrupt observation are allowed; the driver has a 720-second total observation window including admission/cleanup. It never replaces a failed/unknown job.

The runner persists terminal tool/turn evidence before host cancellation, retains Runtime through cancellation readback, then closes transport. AgentQ reconciliation must validate whole-operation/native-descendant completion against the independent checkpoint before original-slot release. Slot release must be acknowledged before terminal publication. Unknown completion, missing turn evidence, corrupt outputs or release acknowledgment retain the reason and temporary/job reservations. Recovery observes original identities, never starts another turn.

A successful driver run downloads all three manifest outputs, verifies SHA-256 against manifest and response headers, retains object provenance and replays the durable event audit. It prints only the job ID and scoped outcome. Native tool/path, original PNG, dimensions, preview, input identities and usage remain in the private spool/outputs; export only sanitized evidence. Subscription charge remains unknown if not reported. Visual acceptance starts pending.

The driver stops only its disposable API and PostgreSQL processes on exit; it never deletes the cluster, store or operation receipts. An early driver failure does not prove native completion. Inspect the retained spool and original slot; do not rerun the driver in a new directory to bypass a failure. A native capability refusal exhausts this run without authorizing a replacement. If recovery needs the isolated database, restart that retained cluster with its original private socket configuration; use the same job-bound command and adapter. No backup qualification is claimed.

## Deterministic evidence and remaining risks

The defect was missing automatic slot/preparation and an unconditional unknown completion path. A reproduction using the previous completion helper, after simulated validated reconciliation, failed `image_worker_terminal_recovery_requires_completion_and_intact_original` with `protected native/descendant completion unknown; original operation retained; no replay`. This is a reconstruction of the old path, not a historical native failure. The same regression passes with the new completion path. Unknown certificate/turn and corrupt PNG cases still retain reservations; successful recovery retrieves identical original fixture bytes.

`python3 scripts/check.py --test image_worker` passes 13 core image checks, two PostgreSQL image checks and three driver checks. The driver checks use invented responses/bytes; they verify identical duplicate submissions, control replacement, hash refusal, terminal refusal and worker-role refusal. They do not qualify native generation. The canonical gate includes them. Existing `observation-unknown.json` and `protected-supervisor-exit.json` traces retain the executable model's original-operation/no-replay and uncertain-completion protections; no job state or event meaning changes.

Current native evidence remains open for automatic preparation, current image tool/saved path, original PNG and the full protected durable/retrieval journey (A2/A3/A6). Exact installed adapter source/build-to-CI mapping remains unknown. Prior native transport PASS, historical crate evidence and unchanged method versions are preserved; no new acceptance item is declared complete from fixtures or health.

Canonical candidate gate: `python3 scripts/check.py` PASS in 16.04 seconds on Mac, including setup: four checker checks, three driver checks, 16 core, 26 isolated PostgreSQL service and two trace checks, plus pinned contract lint/format/whitespace. An initial gate stopped at four new contract paragraphs over 400 characters (1.00 second); splitting those paragraphs preserved their wording/conditions and lint then passed. Combined measured attempts remain below the 600-second budget. Mac timing does not qualify Ubuntu timing or merge-result CI. No additional full run was made because main moved.

Local debug candidate SHA-256: `6fa39bbbd0837d75364eb6553c82b4dbcb6fcd3a64bb474e4b33b5c44540b109`. Any operator build records its actual source/worktree cleanliness and binary hash instead of borrowing this digest. No native model/image operation was dispatched during implementation; native evidence is still open. The new image qualification has not consumed its authorized turn.
