# Owner request and accepted scope

The owner accepted the architecture discussed in this chat: AgentQ owns a reusable worker runtime with optional optimization; CoreQ retains generic infrastructure; AssetQ owns durable generation jobs, versioned methods, comparisons/recommendations and managed asset storage on pds18.

## Original request

> давай зафиксируем эту архитектуру и составим план: что переносим из coreq в agentq, как подключаем taskq и pilotq и с чего начинаем assetq с хранилищем на pds18
> создай проект assetq - убедись что в нем настроены наши принципы SDD, добавь его в taskq
> добавь задачи, в цикле следи за их выполнением - отвечай на технические вопросы и если что надо обсудить со мной - спрашивай
>
> давай начнем с 2х примеров - создать картинку и создать 3d модель по картинке
> картинку давай как базовый метод - сгенерировать через агент воркера на codex или claude
> а пример 3d пайплайна - можешь взять пайплайн из проекта csgo - какой там сейчас у нас победитель для создания 3d моделей

## Confirmed follow-up answers

- Repository visibility: first "Закрытый"; after GitHub refused branch protection on the current private plan, the owner changed the decision: "ок сделай публичным". Repository: public `taskqorg/assetq`; protected main and CI remain required.
- Asked whether to authorize a bounded Meshy run without purchases/top-ups, the owner answered: "1000". Record a cumulative 1000-credit maximum for this initial workstream, with no purchase or top-up.

## Meshy testing spend amendment

Task #5 answer [6629641665](https://github.com/taskqorg/assetq/issues/5#issuecomment-6629641665) supersedes the original cumulative Meshy cap and the unchanged-ceiling statement in the website direction below:

> Amend A6: uncapped testing - + follows the latest owner direction; - removes the cumulative spend guard. Amend owning rules; keep cost evidence.

Initial testing now retains actual/cumulative costs without a cumulative credit ceiling. Unknown usage, reconciliation, no purchases/top-ups and per-job/resource protections remain required.

## Preserved broader requirements

The earlier request also requires text and attachment inputs, image/video/3D/audio and other asset kinds, durable jobs lasting hours, alternative APIs/models/pipelines, adding and changing methods, comparative tests, agent-and-human recommendations, operational control and owned asset storage on pds18. The two initial methods are the first implementation scope, not deletion of those requirements.

## Website direction

> assets.ufobe.com - сайт для api и для менеджмента наших ассетов - просмотр, поиск, бенчмарк пайплайнов и т.д.

The canonical management application and API belong at `https://assets.ufobe.com`. AssetQ owns browsing/search, previews/detail, generation jobs, method management and pipeline benchmarks. This supersedes the earlier plan to put the complete asset workspace in DashQ; DashQ retains summary metadata and deep links. Private asset access, scoped authority and the 1000-credit Meshy ceiling remain unchanged.

## Initial backup baseline

> не заморачивайся, сделай как-то там сейчас, как у нас в других проектах сделано по дефолту. И давай дальше продолжай.

Task #4 steering note 6557600407 retains this owner direction: reuse the managed host's existing backup mechanism, consistent object/metadata snapshots and a verified isolated local restore. Independent external retrieval/restoration remains Open/unknown and does not block initial release or image/3D/site work. Data integrity, scoped access, paired snapshots and local restore/hash checks remain required; native image-tool and original bitmap acceptance is unchanged.

## Initial functionality without backup outages

> Ничего он останавливаться не должен.
> Пока сейчас надо функционал быстрее запустить, про протестировать.

Task #4 steering note 6558009015 records the newer direct owner decision, superseding the earlier initial backup gate: install and test functionality now; defer new backup work and both local and external backup/restore qualification. Preserve existing host backups and keep unproved backup/restore evidence unknown. No backup-driven AssetQ stop/restart, cold-backup timer, outage, rotation or new backup platform is authorized. Data integrity, authentication, resource limits and durable job recovery remain required.

The same note records owner acceptance of the existing crate for image-to-3D. Preserve that exact asset and original operation; do not regenerate it. Reusable protected launch and current-source native qualification remain separate open requirements.
