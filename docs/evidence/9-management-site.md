# Management website candidate evidence

Task [#9](https://github.com/taskqorg/assetq/issues/9), applying R1-R9. The owner selected the table library and shared styling in [answer 6638318903](https://github.com/taskqorg/assetq/issues/9#issuecomment-6638318903). [Design package](../design/management-site.md) records scenarios, reference images and the state matrix. [Service](../service.md#management-browser-boundary) owns exact shared/browser API paths; [deployment](../deploy.md#management-website-candidate) owns the restricted host handoff.

## Implemented boundary

The existing binary now embeds the browser application. It provides scoped table browsing/search, immutable asset detail and relationships, private bounded PNG/retained-render previews and original downloads; text/attachment admission with retained idempotency keys; job progress/history/cancel/results; exact method inspection/version creation; immutable dataset/evaluator registration, frozen comparison setup, sample outputs and quality/cost/time/failure/missing evidence; scoped recommendations and owner promotion.

Browser sessions adapt verified host identity to existing project grants. They do not create authority or durable asset state. Machine routes remain Bearer-only. Browser mutations check exact origin and session-bound CSRF; same-project lookups precede previews/downloads. The adapter defaults off until protected operator configuration exists. Configuration is not host qualification.

## Development observations

`python3 scripts/check.py --test management` passed the isolated HTTP/PostgreSQL management fixture and affected Python checks. Its media input is the unchanged published #5 front render, imported into a disposable store. It is not the private managed image/GLB catalog and does not qualify production sign-in, current native generation or paid comparison execution.

The fixture checks session subject binding, unauthorized scope/duplicate project refusal, CSRF/origin denials, machine-cookie refusal, owner-only edit denial, private literal text/type search, bounded PNG decode/preview, corrupt-PNG refusal, unchanged original bytes, foreign metadata/preview/relationship/download denial, idempotent durable admission and grant revocation. Session termination/reconnection preserves the original durable job. Exact method and retained-key lookups are covered at the same shared database boundary.

`node --test --test-reporter=tap scripts/test_web.cjs` passed form-to-API component checks for exact method pins, original request keys, attachment/numeric/constraint bounds and the published method/comparison/recommendation payloads. These checks are part of the canonical gate alongside JavaScript syntax checking. They are not browser interaction tests.

Initial Windows canonical execution failed because the selected GNU toolchain lacked `cargo-fmt.exe`. A checksum-verified official formatter was staged in temporary task tooling and used without changing global Rust settings. The new Node runner initially failed on a Unicode checkmark under the terminal encoding; TAP output then passed with unchanged assertions. No existing lifecycle, recovery or authority guard was weakened. No provider work or production configuration change occurred.

## Acceptance status

| Acceptance | Current evidence | Remaining proof |
|---|---|---|
| A1 | Owner table/style answer, owning contract edited before code, linked scenarios and exact API contract | Final candidate gate/readback |
| A2-A5 | Implemented browser surface and shared API; isolated HTTP and payload-component checks | Actual browser interactions, responsive/focus matrix and live host guard/private records |
| A6 | Isolated private-access denials and retained public render integrity | Real managed image/3D browser/API journeys; no synthetic fixture may stand in for these |
| A7 | Same branch/PR, embedded UI digest manifest support, compatible machine routes | Frozen full gate, exact packaged executable/UI digests, merge-result CI and actual UI captures |
| A8 | Restricted publication input checklist and rollback boundary specified | Exact private DNS/site/TLS/upstream/guard/mapping/rollback readback and authorized activation |

Computer-use inventory reports no enabled apps or browsers. Actual browser captures and geometry/interaction cells are **Not tested**, not passed or simulated. Host TLS, sign-in, private catalog and publication were not accessed from this Windows source attempt. The existing private host workstream must supply those observations before final acceptance. New backup/restore qualification stays deferred/unknown under the contract.

Full canonical and packaged-candidate outcomes are recorded on the task with their exact source revision when available. This record does not claim them in advance. The workstream remains incomplete until the authenticated deployed application and required real journeys exist.
