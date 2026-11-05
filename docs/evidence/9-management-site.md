# Management website candidate evidence

Task [#9](https://github.com/taskqorg/assetq/issues/9), applying R1-R9. The owner selected the table library and shared styling in [answer 6638318903](https://github.com/taskqorg/assetq/issues/9#issuecomment-6638318903). [Design package](../design/management-site.md) records scenarios, reference images and the state matrix. [Service](../service.md#management-browser-boundary) owns exact shared/browser API paths; [deployment](../deploy.md#management-website-candidate) owns the restricted host handoff.

## Implemented boundary

The existing binary now embeds the browser application. It provides scoped table browsing/search, immutable asset detail and relationships, private bounded PNG/retained-render previews and original downloads; text/attachment admission with retained idempotency keys; job progress/history/cancel/results; exact method inspection/version creation; immutable dataset/evaluator registration, frozen comparison setup, sample outputs and quality/cost/time/failure/missing evidence; scoped recommendations and owner promotion.

Browser sessions adapt verified host identity to existing project grants. They do not create authority or durable asset state. Machine routes remain Bearer-only. Browser mutations check exact origin and session-bound CSRF; same-project lookups precede previews/downloads. The adapter defaults off until protected operator configuration exists. Configuration is not host qualification.

## Development observations

`python3 scripts/check.py --test management` passed the isolated HTTP/PostgreSQL management fixture and affected Python checks. Its media input is the unchanged published #5 front render, imported into a disposable store. It is not the private managed image/GLB catalog and does not qualify production sign-in, current native generation or paid comparison execution.

The fixture checks session subject binding, unauthorized scope/duplicate project refusal, CSRF/origin denials, machine-cookie refusal, owner-only edit denial, private literal text/type search, bounded PNG decode/preview, corrupt-PNG refusal, unchanged original bytes, foreign metadata/preview/relationship/download denial, idempotent durable admission and grant revocation. Session termination/reconnection preserves the original durable job. Exact method and retained-key lookups are covered at the same shared database boundary.

`node --test --test-reporter=tap scripts/test_web.cjs` passed form-to-API component checks for exact method pins, original request keys, attachment/numeric/constraint bounds and the published method/comparison/recommendation payloads. These checks are part of the canonical gate alongside JavaScript syntax checking. They are not browser interaction tests.

Initial Windows canonical execution failed because the selected GNU toolchain lacked `cargo-fmt.exe`. A checksum-verified official formatter was staged in temporary task tooling and used without changing global Rust settings. The new Node runner initially failed on a Unicode checkmark under the terminal encoding; TAP output then passed with unchanged assertions. Windows packaging then exposed a separate failure: `platform.libc_ver()` tried opening the Windows Store Python alias and returned OS error 22. Packaging now observes libc only on the Linux target and records an empty, inapplicable pair on Windows; it still hash-checks and executes the extracted candidate. No existing lifecycle, recovery or authority guard was weakened. No provider work or production configuration change occurred.

## Acceptance status

| Acceptance | Current evidence | Remaining proof |
|---|---|---|
| A1 | Owner table/style answer, owning contract edited before code, linked scenarios and exact API contract | Final candidate gate/readback |
| A2-A5 | Implemented browser surface and shared API; isolated HTTP and payload-component checks | Actual browser interactions, responsive/focus matrix and live host guard/private records |
| A6 | Isolated private-access denials and retained public render integrity | Real managed image/3D browser/API journeys; no synthetic fixture may stand in for these |
| A7 | Same branch/PR, embedded UI digest manifest support, compatible machine routes | Frozen full gate, exact packaged executable/UI digests, merge-result CI and actual UI captures |
| A8 | Restricted publication input checklist and rollback boundary specified | Exact private DNS/site/TLS/upstream/guard/mapping/rollback readback and authorized activation |

Computer-use inventory reports no enabled apps or browsers. Actual browser captures and geometry/interaction cells are **Not tested**, not passed or simulated. Host TLS, sign-in, private catalog and publication were not accessed from this Windows source attempt. The existing private host workstream must supply those observations before final acceptance. New backup/restore qualification stays deferred/unknown under the contract.

## Source review and corrections

A read-only Astra review of frozen revision `d7767e1` found four failures: default Jobs supplied `phase=` and hid the history; default Methods supplied `operation=` and hid the registry; verified by-key recovery retained the pending key indefinitely; and member comparison/proposal controls were hidden despite shared API permission. No auth/CSRF/cross-project bypass was established by that source review; it did not prove browser or live behavior.

The client now omits absent optional predicates, clears a pending key only after verified original-job recovery, and retains owner checks specifically around execution, registration and promotion. Component checks cover default/cleared URLs and absent/unknown/recovered request-key transitions; the HTTP fixture exercises the actual client-generated list URLs and member creation/proposal versus owner execution/promotion denials. The isolated HTTP reproducer observed the old empty-filter requests returning zero rows. Corrected client URLs returned one durable job and eight seeded methods on the same fixture. The focused management checks passed, including session expiry without renewal and member-versus-owner boundaries. Back navigation now retains the list URL, scroll and matching link focus; a failed same-view refresh labels retained output stale and clears it on session/project changes. These DOM behaviors still require actual browser qualification. Pending-key recovery is component-level before/after state evidence, not browser proof.

A read-only public DNS lookup observed `assets.ufobe.com` as a CNAME to `ufobe.com`, with a 300-second TTL and A resolution `104.250.138.186`. It does not establish DNS account ownership or the intended private host/site. The canonical HTTPS HEAD could not connect from this environment; TLS, sign-in and live reachability remain unknown. No DNS or HTTPS configuration was changed.

Full canonical and packaged-candidate outcomes are recorded on the task with their exact source revision when available. This record does not claim them in advance. The workstream remains incomplete until the authenticated deployed application and required real journeys exist.

## Browser layout correction

PM note 6644578630 reports two failures on exact Linux candidate `d714ca513113e68673d850c4d953cb768842bca2` in Chromium 156 with disposable PostgreSQL/API fixtures: at 1440 px, bare header text occupied the thumbnail column; at 390 px, the unsupported `application/x-assetq-synthetic` thumbnail label overflowed into the following PNG. These are failures of the approved table reference and R9, under design fidelity F1/F2; the fixtures do not qualify live authentication or managed records.

The header now has three explicit element cells, including the empty thumbnail cell. Compact unsupported previews use a short `File` label with thumbnail-specific padding; the full MIME remains in the adjacent metadata, hover title and asset detail, with the original download unchanged. The same fallback handles failed image previews. No API, authority, storage, dispatch or lifecycle behavior changes. JavaScript syntax and the existing four request-component checks pass; they do not establish rendered geometry. PM note 6644946579 supplies browser requalification and representative screenshots after the refreshed Linux candidate. The 1440/390 px visual recheck and text scaling remain Not tested by this source worker; fixed-pixel text did not enlarge in the PM's prior root-font scaling attempt.

## Missing model preview caption correction

PM note 6646983048 reports an R4/R9 defect on `1cfeb654e9a6ffa85c61c3e8dbac4e366f59452c`: an isolated retained GLB with no render/job relationship shows the unavailable fallback and falsely claims a retained render is displayed. The selected design and API already require truthful missing-preview evidence; no product decision or approved layout changes.

Before the fix, the new `scripts/test_web.cjs` component case failed with `model/gltf-binary: loading/unavailable is not a retained render`. After the fix, all five Node cases and JavaScript syntax pass. The actual shared preview handler now appends the retained-render caption only on model-image load, removes it with the preview on error and leaves the original download unchanged. The case covers missing/failed previews, successful/repeated load, PNG and unsupported types, and compact thumbnails without inferring relationships from MIME or inventing outputs. Existing assertions and provider/no-replay guards are unchanged. The canonical gate already runs this Node runner; no separate dependency or browser harness was added.

These are component observations with invented metadata and simulated image events, not a new real-browser, retained-record, host-auth or visual acceptance receipt. PM/#25 must requalify the exact refreshed Linux binary/UI. Canonical and artifact outcomes are recorded on the task against the final source identity, never inherited from `1cfeb65`. A2-A8, text scaling, live TLS/sign-in and managed-record journeys remain open until their respective receipts exist; backup/restore stays deferred/unknown.

## Reviewed-source publication handoff

Task [#32](https://github.com/taskqorg/assetq/issues/32) owns publication of the existing candidate through independent TaskQ review and the normal merge queue. Its original request and A1-A6 remain on that task. The frozen inputs are PR24 head `3f9def717038814261c434f497be2736e36d9e3e`, tree `8ef87205ee6505f35b844272c48d762a9cd13924`, and main `9c101b46b37a3d4688cb97abdd754fde253e8850`, including #26/#28/#30. The publication branch adopts that exact tree before this handoff addition; this section is its only change relative to the frozen candidate. The parked #9 worktree remains untouched. No source behavior, approved UI, assertion, timeout, authority, budget or provider-effect barrier changes (R1-R9).

Fresh unchanged Windows `python3 scripts/check.py` with the 480-second Cargo deadline and Ubuntu canonical CI with its 600-second job budget must identify the publication candidate. Prior PR24 gates and isolated browser receipts remain historical evidence, not automatic passes. The worker's result records its exact commit and observed checks; independent review, final green `merge_group` jobs `tests` and `candidate`, and actual main readback remain required before delivery (R5/R8; TaskQ Â§7).

After merge, the publication reviewer/service handoff to #25 must bind the result commit, main/source commit and tree, merge-group run, GitHub artifact ID/digest, inner archive name/SHA-256, `candidate.json` source, executable SHA-256 and all four embedded UI SHA-256 values. Verify downloaded checksums, source documents/UI blobs and embedded executable bytes; keep temporary PR sources distinct from the delivered source even if binaries match. Record these revision-specific receipts on #32 and #25 rather than committing a self-referential publication identity here.

Code delivery does not close #9 or #25. Canonical sign-in, managed image/3D and denied-access journeys, full browser/state/text-scaling coverage and final site acceptance remain with those tasks. Backup/restore remains deferred/unknown. This publication grants no host, DNS, Caddy, secret, credential, grant, model/account, provider, LIVE deployment or security-policy effect. Functional defects return to source owner #9; final acceptance stays intact (R3/R4/R8/R9).
