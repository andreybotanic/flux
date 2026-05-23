# S06 — External test mod + patching

## Depends on

- S05

## Можно выполнять параллельно с

- S07
- S08
- S10

## Цель этапа

Реализовать патчинг контента модов и проверить, что внешний мод может добавить контент и пропатчить контент `base`.


## Требования к реализации

- Изменить текущий формат ron файлов контента: в файле должен явно указываться тип контента (SolidCellPrototype(id: ...), StructurePrototype(id: ...) и т.д.);
- Реализовать патчинг контента;
- Создать `mods/test_content_mod`.
- Мод должен:
  - зависеть от `base`;
  - добавить новый контент (любой);
  - пропатчить любую сущность из `base`.
- Diagnostic summary должен показывать source mod и applied patches.

## Патч

Для патча нужны следующие структуры:
```rust
pub struct PrototypePatch {
    pub target: PrototypeId,
    pub body: PrototypePatchBody,
}

pub enum PrototypePatchBody {
    SolidCell(SolidCellPrototypePatch),
    Structure(StructurePrototypePatch),
    ...
}
```
Где:
| Тип                      | Роль                                          |
| ------------------------ | --------------------------------------------- |
| `PrototypePatch`         | общий контейнер патча: target, body       |
| `PrototypePatchBody`     | enum, выбирающий тип патча                    |
| `SolidCellPrototypePatch` | конкретный patch-body для `SolidCellPrototype` |
| `StructurePrototypePatch`    | конкретный patch-body для `StructurePrototype`    |

Каждый вариант PrototypePatchBody почти полностью повторяет свой Prototype. Есть только два отличия:
- у PrototypePatchBody нет поля id;
- у PrototypePatchBody все поля - Option с `#[serde(default)]`, но при этом хотя бы одно поле должно быть заполнено, чтобы патчи считался валидным.

Правила применения патчей:
- патчи применяются в порядке загрузки плагинов;
- попытка пропатчить несуществующий контент - ошибка загрузки плагина;
- плагин может применить только один патч на каждый контент (в пределах плагина каждый target в PrototypePatch может встречаться только один раз);

Примерный формат патча (ron файл):
```rust
PrototypePatch(
    target: "base:structure/door",
    body: Structure(
        display_name: "$test_content_mod.structure.test_door",
        size: (2, 1),
    ),
)
```

### Prototype/Patch type binding

Для каждого Prototype должен существовать строго связанный Patch-тип. Добавление нового prototype-kind без patch-type должно приводить к ошибке компиляции.

Использовать associated type:
```rust
pub trait Prototype {
    type Patch: PrototypePatchFor<Self>;

    const KIND: PrototypeKind;
}
```

И trait патча:
```rust
pub trait PrototypePatchFor<P: Prototype> {
    fn is_empty(&self) -> bool;

    fn apply_to(
        self,
        target: &mut P,
    ) -> PatchResult;
}
```

Пример:
```rust
pub struct BuildingPrototype {
    pub id: PrototypeId,
    pub display_name: LocalizationKey,
}

pub struct BuildingPrototypePatch {
    #[serde(default)]
    pub display_name: Option<LocalizationKey>,
}

impl Prototype for BuildingPrototype {
    type Patch = BuildingPrototypePatch;

    const KIND: PrototypeKind = PrototypeKind::Building;
}

impl PrototypePatchFor<BuildingPrototype>
    for BuildingPrototypePatch
{
    fn is_empty(&self) -> bool {
        self.display_name.is_none()
    }

    fn apply_to(
        self,
        target: &mut BuildingPrototype,
    ) -> PatchResult {
        if let Some(display_name) = self.display_name {
            target.display_name = display_name;
        }

        Ok(())
    }
}
```

Все prototype-kinds должны регистрироваться централизованно через единый registry/macro:
```rust
define_prototype_kinds! {
    Building => (
        prototype: BuildingPrototype,
        patch: BuildingPrototypePatch,
    ),
}
```

Макрос должен генерировать:
- PrototypeKind
- PrototypeBody
- PrototypePatchBody

Добавление нового prototype-kind вне этого registry запрещено.

## Ручная проверка

1. Запустить content summary.
2. Убедиться, что внешний материал появился.
3. Убедиться, что patch к `base` применен.
4. Испортить dependency, сломать патч.
5. Убедиться, что ошибка понятна.


## Automated checks

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Definition of Done

- Реализована только зона ответственности этапа.
- Все automated checks проходят.
- Выполнена ручная проверка из этого документа.
- Нет изменений вне зоны ответственности без объяснения.
- Отчет этапа заполнен по `docs/STAGE_COMPLETION_REPORT_TEMPLATE.md`.
