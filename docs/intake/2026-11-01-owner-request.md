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

## Preserved broader requirements

The earlier request also requires text and attachment inputs, image/video/3D/audio and other asset kinds, durable jobs lasting hours, alternative APIs/models/pipelines, adding and changing methods, comparative tests, agent-and-human recommendations, operational control and owned asset storage on pds18. The two initial methods are the first implementation scope, not deletion of those requirements.
