# Bounded blue-wedge image run

Task [#4](https://github.com/taskqorg/assetq/issues/4); the single run the [owner approved](image-qualification-next.md#approved-bounded-job), executed once by the PM on the qualified Mac through the [operator recipe](bounded-image-job.md). This authorization is now consumed; it does not authorize a second turn or a new directory.

## Candidate

- Source `2357b6254acf27fe3d0d9cfde08758b5f35f716f` (branch `taskq-4` after merging main at `5fe6736`), clean worktree, `python3 scripts/check.py` PASS.
- Debug binary SHA-256 `4f3965af2c1eb48f3c61115033098a75dffa24be5e4036ca2e8e08e2efeb8858`.
- Retained signed TaskQ adapter SHA-256 `bd6dc0de5024ff7b6b0ae0a987f5467aa0c5196b005c7cdfd434d043709c7e42` (installed `4.6.66-261103-26` bytes); no installed or prior adapter was used.
- Operator context: `TASKQ_ATTEMPT` absent; no role variable, grant, account, model or configuration was changed.

## Outcome: native image produced, import refused, job left unknown

Job `006a1ef7-a38f-43ad-af9f-a61f51794cd2`, method `image.worker.v1@5`, provider operation `86e7d75b-6d30-421a-a8d8-e52889e1c958`. Invented swatch attachment `c41a393a-2f79-4a33-ae28-eba71bcf3493` (SHA-256 `61efeaadc52dcaa6e8d2292d00488d835d4258453998955e93a8bfb20f20f9c6`).

| Step | Result |
|---|---|
| Accept, duplicate before effects | Same job ID, `created: false` |
| Job-bound `image-serve`, slot reserve, AgentQ `prepare` | Resolved existing workspace grant, revision 34; one launch |
| Runtime input | One submission (`submissions: 1`) |
| Control-service interruption after input intent | Duplicate after restart returned the same job; provider operation unchanged |
| Native turn | `turn/completed`; one `imageGeneration` item `completed`, `failure: null`, with a native saved path |
| Original PNG | 1254 by 1254 pixels, 1,806,057 bytes, SHA-256 `880bdc25d7add790a766bad2123aa36b4157c6585e991c8d07743cf0be35e960`; retained unchanged in the private native artifact directory |
| AssetQ validation | Refused: `PNG dimensions exceed worker/request limits` (request limit 1024 by 1024) |
| Job state | `waiting`, reason `original native operation ended but turn result remains unknown; no replay`; no result manifest, no retrieval |
| Reconciliation and slot | AgentQ reconciliation `completed`; original slot `released` ("consumer confirmed entire turn ended") |
| Driver deadline | 720 s window expired; cancel `uncertain`; credits unknown |

Timing (UTC): driver start 14:16:40, job accepted and submitted 14:16:42, first observation of the ended native operation 14:17:40, job deadline 14:26:42, driver exit 14:28:42 (exit code 1, `qualification deadline; original operation/reservations retained; no replay`).

Reported native usage: 152,203 total tokens (151,480 input of which 120,960 cached, 723 output). Subscription charge and the image backend model remain unknown. No paid API was used.

## Reading

The native image tool and its saved-path route work under the protected composed launch (A3 tool/output path observed). The tool ignored the requested 1024 limit and returned 1254 by 1254, the same size as the accepted crate. AssetQ correctly refused to import an out-of-bounds bitmap and replayed nothing, but it recorded this as unknown turn result instead of a precise terminal refusal, so the job stays `waiting` with its reservations.

Open: a method decision on the size limit (accept the native fixed size, or downscale as a declared derived output while preserving the original), and a terminal refusal path for a completed turn whose output fails validation. Duplicate request, control interruption, one-submission and no-replay evidence are observed; immutable import and authenticated retrieval (A2/A6) remain unproved. Private spool, cluster and logs are retained for recovery; no rerun was made.
