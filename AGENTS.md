# AssetQ

Shared rules: [specq](https://github.com/taskqorg/specq) is binding for every task: [principles](https://github.com/taskqorg/specq/blob/main/engineering/principles.md), [spec first](https://github.com/taskqorg/specq/blob/main/engineering/spec-first.md), [contracts](https://github.com/taskqorg/specq/blob/main/engineering/contracts.md), [testing](https://github.com/taskqorg/specq/blob/main/engineering/testing.md), [verification](https://github.com/taskqorg/specq/blob/main/engineering/verification.md), [security](https://github.com/taskqorg/specq/blob/main/engineering/security.md), [agent workflow](https://github.com/taskqorg/specq/blob/main/engineering/agent-workflow.md).

Read [assetq.md](assetq.md) first. Change its owning rule before implementation; preserve the owner's request in the task or linked intake. Check [acceptance](docs/acceptance.md) and [implementation order](docs/implementation-plan.md). Lifecycle changes require an executable behavioral model and replayable recovery traces before acceptance.

Canonical gate: `python3 scripts/check.py`. Add affected implementation checks to that gate when implementation arrives. Distinguish implemented, tested, published, deployed and live-qualified. Technical choices belong to the worker; product, visual, spending and authority changes go to the owner.
