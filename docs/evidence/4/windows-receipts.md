# Windows reconciliation receipt replacement

Task [#4](https://github.com/taskqorg/assetq/issues/4), A6 and AssetQ R3-R8. Review continuation request, verbatim: "Fix Windows receipt replacement; retain write-through durability, R3/R4 guards and assertions."

## Failure and fix

At `11deb2ce23d993f930a8f2847cbeda1943681d6d`, the existing check reproduced the review failure on Windows:

```sh
cargo test --locked --lib image_worker_slot_and_completion_order_preserve_original_operation
```

Result: FAIL, 0 passed and 1 failed, at `src/image_worker/protected.rs:524`: `Cannot create a file when that file already exists. (os error 183)`. After an unknown reconciliation had been retained, the completed observation could not replace `reconciliation.json`; the original slot could not be released.

The owning [image-worker specification](../../image-worker.md) now distinguishes mutable observation snapshots from create-once launch markers and sealed original identities. No lifecycle decision, guarding assertion or model transition changes. Windows `durable_rename` now combines `MOVEFILE_REPLACE_EXISTING` with the retained `MOVEFILE_WRITE_THROUGH`, matching Unix replacement semantics without deleting the prior receipt first. Temporary bytes are still flushed before the move; native errors still propagate.

The same command then PASSed: 1 passed, 0 failed. The unchanged test protects original slot identity, unknown/mismatched admission refusal, retained occupancy on unknown or certificate mismatch, and completion before release. It now reaches the completed reconciliation and acknowledged release on Windows. Existing checks already detect the defect, so no duplicate test was added (specq Testing, Before adding or changing a test).

Scope review found two callers of `durable_rename`: receipt `save_bytes` and object installation. Object installation retains its writer lock, content-address verification, existing-object branch and read-only committed files. `seal` and the launch markers retain their separate create-once operations; the replacement flag does not bypass them. Runtime checkpoints remain in the bounded append journal, as recorded in the earlier [runtime adoption](runtime-v020.md#checks-and-defect-evidence).

## Candidate verification

The canonical gate and final candidate identity are recorded with this task's result and PR. This correction does not rerun the consumed native image turn, change authority/account/model/optimizer, or edit retained source/import receipts. The [native import result](native-size-import.md#native-operator-result) and [A1-A6 matrix](image-qualification-next.md#acceptance-readback) retain their exact earlier artifacts and limitations.

General dispatch remains refused. Adapter source/build-to-CI mapping, source charge, uncertain source cancellation, new visual acceptance, managed deployment/site and deferred backup/restore retain their recorded status. Local Windows checks do not establish Ubuntu timing, merge-result CI or hardware power-loss behavior.
