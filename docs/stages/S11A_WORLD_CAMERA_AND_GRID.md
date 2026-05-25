# S11A — World Camera & Grid MVP

## Цель этапа

Цель: создать экран отладки мира без отображения содержимого клеток.


## Требования к реализации

По части графики и интерактива нужно реализовать:
- orthographic camera;
- zoom через mouse scroll;
- pan через MMB drag
- pan через WASD
- отображение сетки мира GridSize;
- координатную привязку TilePos -> screen position;

Вспомогательный функционал:
- добавить новый action для кнопок: RunWorld.
- добавить новую кнопку в главное меню с действием RunWorld.


## Запрещено

Не реализовывать:
- solid cells;
- gases;
- structures;
- sprites;
- overlays.


## Automated checks

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```


## Definition of Done

- открывается окно;
- после запуска RunWorld видна сетка мира, например 64×64;
- работает zoom и панорамирование
